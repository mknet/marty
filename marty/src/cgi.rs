//! Lightweight synchronous CGI request/response types for [`crate::Router`].
//!
//! With feature **`hyper-cgi`**, [`CGIRequest`](crate::CGIRequest) / [`CGIResponse`](crate::CGIResponse) use hyper types for Tower/Axum.
//! With **`sync-cgi`** only (no hyper), types are minimal for small CGI binaries.

use crate::{MetaVariableKind, Result, error};
use snafu::ResultExt;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{Read, Write, stdin};
use std::mem::ManuallyDrop;
use std::os::fd::FromRawFd;
use std::time::Duration;

#[cfg(feature = "hyper-cgi")]
use bytes::Bytes;
#[cfg(feature = "hyper-cgi")]
use hyper::HeaderMap;
#[cfg(feature = "hyper-cgi")]
use hyper::http::HeaderValue;

const STATUS_200_OK: &[u8] = b"Status: 200 OK\n";
const JSON_NO_CACHE_HEADERS: &[u8] =
    b"Cache-Control: no-store, no-cache, must-revalidate\nPragma: no-cache\nContent-Type: application/json\n\n";
/// Default pause used by the `apache-cgi-pacing` feature between header and
/// body writes.
///
/// The value is intentionally small: it is meant to give Apache `mod_cgi` a
/// chance to observe two pipe-read phases under short concurrent requests, not
/// to slow application work down. Applications that need a different value can
/// call [`CgiResponse::write_response_to_stdout_paced`] directly.
pub const APACHE_CGI_PACING_DELAY_MICROS: u64 = 50;

static REASONS: &[(&str, &str)] = &[
    ("200", "OK"),
    ("404", "Not Found"),
    ("500", "Internal Server Error"),
];

fn canonical_reason(status: u16) -> Option<&'static str> {
    let s = status.to_string();
    REASONS
        .iter()
        .find(|(code, _)| *code == s.as_str())
        .map(|(_, r)| *r)
}

/// Parsed CGI environment for one request (sync router).
#[derive(Debug, Clone)]
pub struct CgiRequest {
    pub method: String,
    pub script_name: String,
    pub path_info: String,
    pub query_string: String,
    /// Populated only when needed for routing fallback (no `PATH_INFO`).
    pub uri: String,
    #[cfg(feature = "hyper-cgi")]
    pub headers: HeaderMap<HeaderValue>,
    pub body: Vec<u8>,
    pub path_params: HashMap<String, String>,
}

impl CgiRequest {
    /// Read meta-variables and optional request body from the CGI process environment.
    pub fn from_env() -> Result<Self> {
        let method = MetaVariableKind::RequestMethod
            .try_from_env()?
            .as_str()?
            .to_string();

        let script_name = MetaVariableKind::ScriptName
            .from_env()
            .and_then(|v| v.as_str().ok().map(str::to_string))
            .unwrap_or_default();

        let path_info = MetaVariableKind::PathInfo
            .from_env()
            .and_then(|v| v.as_str().ok().map(str::to_string))
            .unwrap_or_default();

        let query_string = MetaVariableKind::QueryString
            .from_env()
            .and_then(|v| v.as_str().ok().map(str::to_string))
            .unwrap_or_default();

        let uri = if path_info.is_empty() {
            build_request_uri(&script_name, &path_info, &query_string)?
        } else {
            String::new()
        };

        let content_length = MetaVariableKind::ContentLength
            .from_env()
            .and_then(|v| v.as_str().ok().and_then(|s| s.parse().ok()))
            .unwrap_or(0);

        let mut body = Vec::new();
        if content_length > 0 {
            body.resize(content_length, 0);
            stdin()
                .read_exact(&mut body)
                .context(error::ReadRequestBodySnafu)?;
        }

        Ok(Self {
            method,
            script_name,
            path_info,
            query_string,
            uri,
            #[cfg(feature = "hyper-cgi")]
            headers: HeaderMap::new(),
            body,
            path_params: HashMap::new(),
        })
    }

    /// Path parameter extracted by the router (e.g. `limit` for `/primes/{limit}`).
    pub fn param(&self, name: &str) -> Option<&str> {
        self.path_params.get(name).map(String::as_str)
    }
}

fn build_request_uri(script_name: &str, path_info: &str, query_string: &str) -> Result<String> {
    let mut logical =
        String::with_capacity(script_name.len() + path_info.len() + query_string.len() + 1);
    logical.push_str(script_name);
    logical.push_str(path_info);
    if !query_string.is_empty() {
        logical.push('?');
        logical.push_str(query_string);
    }

    match MetaVariableKind::RequestUri.from_env() {
        Some(uri) => {
            let req = uri.as_str()?;
            if req == logical.as_str() {
                return Ok(logical);
            }
            if !req.starts_with(script_name) {
                return Ok(logical);
            }
            Ok(req.to_string())
        }
        None => Ok(logical),
    }
}

/// CGI document response (sync router).
#[derive(Debug)]
pub struct CgiResponse {
    #[cfg(feature = "hyper-cgi")]
    pub headers: HeaderMap<HeaderValue>,
    pub status: String,
    pub reason: Option<String>,
    #[cfg(feature = "hyper-cgi")]
    pub body: Bytes,
    #[cfg(not(feature = "hyper-cgi"))]
    pub body: Vec<u8>,
    /// Use [`Self::write_response_to_output`] fast path (no per-header `format!`).
    pub json_no_cache: bool,
}

impl CgiResponse {
    pub fn new(status: u16, body: impl AsRef<[u8]>) -> Self {
        Self {
            #[cfg(feature = "hyper-cgi")]
            headers: HeaderMap::new(),
            status: status.to_string(),
            reason: canonical_reason(status).map(str::to_string),
            #[cfg(feature = "hyper-cgi")]
            body: Bytes::copy_from_slice(body.as_ref()),
            #[cfg(not(feature = "hyper-cgi"))]
            body: body.as_ref().to_vec(),
            json_no_cache: false,
        }
    }

    /// `200` JSON body with standard no-cache headers (fast write path).
    pub fn json_no_cache(body: impl AsRef<[u8]>) -> Self {
        Self {
            #[cfg(feature = "hyper-cgi")]
            headers: HeaderMap::new(),
            status: "200".into(),
            reason: Some("OK".into()),
            #[cfg(feature = "hyper-cgi")]
            body: Bytes::copy_from_slice(body.as_ref()),
            #[cfg(not(feature = "hyper-cgi"))]
            body: body.as_ref().to_vec(),
            json_no_cache: true,
        }
    }

    /// `200` JSON body with standard no-cache headers, taking ownership of an
    /// already-built byte buffer.
    pub fn json_no_cache_bytes(body: Vec<u8>) -> Self {
        Self {
            #[cfg(feature = "hyper-cgi")]
            headers: HeaderMap::new(),
            status: "200".into(),
            reason: Some("OK".into()),
            #[cfg(feature = "hyper-cgi")]
            body: Bytes::from(body),
            #[cfg(not(feature = "hyper-cgi"))]
            body,
            json_no_cache: true,
        }
    }

    #[cfg(feature = "hyper-cgi")]
    pub fn with_header(mut self, name: hyper::header::HeaderName, value: HeaderValue) -> Self {
        self.json_no_cache = false;
        self.headers.insert(name, value);
        self
    }

    /// Write this response as a CGI "document" to `output` (typically stdout).
    pub fn write_response_to_output(self, mut output: impl Write) -> Result<()> {
        let bytes = self.into_cgi_bytes()?;
        output
            .write_all(&bytes)
            .context(error::WriteResponseSnafu)?;
        Ok(())
    }

    /// Write this response as two phases where possible: headers first, body
    /// second.
    ///
    /// This exists for diagnostics and deployment tuning when a CGI server or
    /// TCP stack behaves differently for single-write vs split header/body
    /// responses. Normal applications should prefer [`Self::write_response_to_stdout`]
    /// or [`Self::write_response_to_output`].
    pub fn write_response_to_output_split(self, mut output: impl Write) -> Result<()> {
        let bytes = self.into_cgi_bytes()?;
        write_split(&bytes, &mut output)
    }

    /// Write this response as two phases, with an optional delay between
    /// headers and body.
    ///
    /// This is a CGI server pacing knob, not an application-level performance
    /// optimization. Apache `mod_cgi` drains the child process stdout through a
    /// pipe. If a very small CGI program writes the full response and exits in
    /// one scheduler quantum, Apache can observe the response as a single pipe
    /// read and many prefork workers can become idle at nearly the same time.
    /// Under concurrent short requests that can amplify tail latency through
    /// accept-mutex, logging, and child-reaping contention.
    ///
    /// Splitting headers and body, optionally with a tiny delay, gives the CGI
    /// server a chance to observe separate pipe reads and pace the HTTP
    /// response more like buffered runtimes do naturally. Leave the delay at
    /// [`Duration::ZERO`] for a pure split write; use a small non-zero delay
    /// only after measuring that your CGI server benefits from it.
    pub fn write_response_to_output_paced(
        self,
        mut output: impl Write,
        header_body_delay: Duration,
    ) -> Result<()> {
        let bytes = self.into_cgi_bytes()?;
        write_paced(&bytes, &mut output, header_body_delay)
    }

    /// Write this response to process stdout.
    ///
    /// Without feature `apache-cgi-pacing`, this bypasses
    /// [`std::io::stdout`]'s `LineWriter`, which can otherwise flush once per
    /// CGI header newline. Building the full CGI document and writing it
    /// directly to fd 1 preserves the router API while keeping the default path
    /// as a single fast `write()`.
    ///
    /// With feature `apache-cgi-pacing`, this method becomes the paced Apache
    /// CGI path: headers and body are written as separate phases with a tiny
    /// delay. That trades a few microseconds of per-request latency for better
    /// P95/P99 behavior on Apache `mod_cgi`/prefork when many very short CGI
    /// requests finish in the same scheduler quantum.
    pub fn write_response_to_stdout(self) -> Result<()> {
        #[cfg(feature = "apache-cgi-pacing")]
        {
            return self.write_response_to_stdout_paced(Duration::from_micros(
                APACHE_CGI_PACING_DELAY_MICROS,
            ));
        }

        #[cfg(not(feature = "apache-cgi-pacing"))]
        {
            let bytes = self.into_cgi_bytes()?;
            write_stdout_raw(&bytes)
        }
    }

    /// Write this response to stdout as split header/body phases where possible.
    ///
    /// See [`Self::write_response_to_output_split`] for why this mode exists.
    pub fn write_response_to_stdout_split(self) -> Result<()> {
        let bytes = self.into_cgi_bytes()?;
        if let Some(split_at) = header_body_split(&bytes) {
            write_stdout_raw(&bytes[..split_at])?;
            write_stdout_raw(&bytes[split_at..])
        } else {
            write_stdout_raw(&bytes)
        }
    }

    /// Write this response to stdout with CGI server pacing.
    ///
    /// See [`Self::write_response_to_output_paced`] for why this exists.
    /// Normal applications should prefer [`Self::write_response_to_stdout`]
    /// unless measurements show the deployment CGI server benefits from
    /// header/body pacing under concurrent short requests.
    pub fn write_response_to_stdout_paced(self, header_body_delay: Duration) -> Result<()> {
        let bytes = self.into_cgi_bytes()?;
        if let Some(split_at) = header_body_split(&bytes) {
            write_stdout_raw(&bytes[..split_at])?;
            if !header_body_delay.is_zero() {
                std::thread::sleep(header_body_delay);
            }
            write_stdout_raw(&bytes[split_at..])
        } else {
            write_stdout_raw(&bytes)
        }
    }

    fn into_cgi_bytes(self) -> Result<Vec<u8>> {
        #[cfg(feature = "hyper-cgi")]
        let headers_empty = self.headers.is_empty();
        #[cfg(not(feature = "hyper-cgi"))]
        let headers_empty = true;

        let body: &[u8] = self.body.as_ref();
        if self.json_no_cache && self.status == "200" && headers_empty {
            let mut out =
                Vec::with_capacity(STATUS_200_OK.len() + JSON_NO_CACHE_HEADERS.len() + body.len());
            out.extend_from_slice(STATUS_200_OK);
            out.extend_from_slice(JSON_NO_CACHE_HEADERS);
            out.extend_from_slice(body);
            return Ok(out);
        }

        let mut out = Vec::with_capacity(64 + body.len());
        if let Some(reason) = &self.reason {
            writeln!(&mut out, "Status: {} {}", self.status, reason)
                .context(error::WriteResponseSnafu)?;
        } else {
            writeln!(&mut out, "Status: {}", self.status).context(error::WriteResponseSnafu)?;
        }

        #[cfg(feature = "hyper-cgi")]
        for (key, value) in self.headers.iter() {
            writeln!(&mut out, "{}: {}", key, value.to_str().unwrap_or(""))
                .context(error::WriteResponseSnafu)?;
        }
        out.write_all(b"\n").context(error::WriteResponseSnafu)?;
        out.write_all(body).context(error::WriteResponseSnafu)?;
        Ok(out)
    }
}

fn write_stdout_raw(buf: &[u8]) -> Result<()> {
    // SAFETY: fd 1 is the process stdout supplied by the CGI server. Wrapping
    // it in `ManuallyDrop<File>` prevents closing stdout when this function
    // returns; the wrapper exists only to use `Write::write_all`.
    let stdout = unsafe { std::fs::File::from_raw_fd(1) };
    let mut stdout = ManuallyDrop::new(stdout);
    stdout.write_all(buf).context(error::WriteResponseSnafu)
}

fn write_split(bytes: &[u8], output: &mut impl Write) -> Result<()> {
    if let Some(split_at) = header_body_split(bytes) {
        output
            .write_all(&bytes[..split_at])
            .and_then(|_| output.write_all(&bytes[split_at..]))
            .context(error::WriteResponseSnafu)?;
    } else {
        output.write_all(bytes).context(error::WriteResponseSnafu)?;
    }
    Ok(())
}

fn write_paced(bytes: &[u8], output: &mut impl Write, header_body_delay: Duration) -> Result<()> {
    if let Some(split_at) = header_body_split(bytes) {
        output
            .write_all(&bytes[..split_at])
            .context(error::WriteResponseSnafu)?;
        if !header_body_delay.is_zero() {
            std::thread::sleep(header_body_delay);
        }
        output
            .write_all(&bytes[split_at..])
            .context(error::WriteResponseSnafu)?;
    } else {
        output.write_all(bytes).context(error::WriteResponseSnafu)?;
    }
    Ok(())
}

fn header_body_split(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(2)
        .position(|window| window == b"\n\n")
        .map(|idx| idx + 2)
}

/// Relative path for routing: prefer `PATH_INFO`, else strip `SCRIPT_NAME` from `uri`.
pub fn relative_path<'a>(req: &'a CgiRequest) -> Cow<'a, str> {
    if !req.path_info.is_empty() {
        if req.path_info.starts_with('/') {
            return Cow::Borrowed(&req.path_info);
        }
        return Cow::Owned(format!("/{}", req.path_info));
    }
    if !req.script_name.is_empty() {
        let path = req.uri.split('?').next().unwrap_or(&req.uri);
        if let Some(rest) = path.strip_prefix(&req.script_name) {
            if rest.is_empty() || rest == "/" {
                return Cow::Borrowed("/");
            }
            return Cow::Borrowed(rest);
        }
    }
    let path = req.uri.split('?').next().unwrap_or(&req.uri);
    if path.is_empty() {
        Cow::Borrowed("/")
    } else {
        Cow::Borrowed(path)
    }
}

/// Convert a handler return type into a [`CgiResponse`].
pub trait IntoResponse {
    fn into_response(self) -> CgiResponse;
}

impl IntoResponse for CgiResponse {
    fn into_response(self) -> CgiResponse {
        self
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> CgiResponse {
        CgiResponse::new(200, self)
    }
}

impl IntoResponse for String {
    fn into_response(self) -> CgiResponse {
        CgiResponse::new(200, self)
    }
}

impl IntoResponse for (u16, &'static str) {
    fn into_response(self) -> CgiResponse {
        CgiResponse::new(self.0, self.1)
    }
}

#[cfg(feature = "hyper-cgi")]
impl<const N: usize> IntoResponse for (u16, [(&'static str, &'static str); N], &'static str) {
    fn into_response(self) -> CgiResponse {
        let (status, pairs, body) = self;
        let mut resp = CgiResponse::new(status, body);
        for (name, value) in pairs {
            if let (Ok(n), Ok(v)) = (
                hyper::header::HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                resp.headers.insert(n, v);
            }
        }
        resp
    }
}

#[cfg(feature = "hyper-cgi")]
impl<const N: usize> IntoResponse for (u16, [(&'static str, HeaderValue); N], &'static str) {
    fn into_response(self) -> CgiResponse {
        let (status, pairs, body) = self;
        let mut resp = CgiResponse::new(status, body);
        for (name, value) in pairs {
            if let Ok(n) = hyper::header::HeaderName::from_bytes(name.as_bytes()) {
                resp.headers.insert(n, value);
            }
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CountingWrite {
        writes: usize,
        bytes: Vec<u8>,
    }

    impl Write for CountingWrite {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.writes += 1;
            self.bytes.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn json_no_cache_writes_one_buffer() {
        let mut out = CountingWrite {
            writes: 0,
            bytes: Vec::new(),
        };

        CgiResponse::json_no_cache_bytes(b"{}".to_vec())
            .write_response_to_output(&mut out)
            .unwrap();

        assert_eq!(out.writes, 1);
        assert_eq!(
            out.bytes,
            b"Status: 200 OK\nCache-Control: no-store, no-cache, must-revalidate\nPragma: no-cache\nContent-Type: application/json\n\n{}"
        );
    }

    #[test]
    fn json_no_cache_can_split_header_and_body() {
        let mut out = CountingWrite {
            writes: 0,
            bytes: Vec::new(),
        };

        CgiResponse::json_no_cache_bytes(b"{}".to_vec())
            .write_response_to_output_split(&mut out)
            .unwrap();

        assert_eq!(out.writes, 2);
        assert_eq!(
            out.bytes,
            b"Status: 200 OK\nCache-Control: no-store, no-cache, must-revalidate\nPragma: no-cache\nContent-Type: application/json\n\n{}"
        );
    }

    #[test]
    fn json_no_cache_can_pace_header_and_body() {
        let mut out = CountingWrite {
            writes: 0,
            bytes: Vec::new(),
        };

        CgiResponse::json_no_cache_bytes(b"{}".to_vec())
            .write_response_to_output_paced(&mut out, Duration::ZERO)
            .unwrap();

        assert_eq!(out.writes, 2);
        assert_eq!(
            out.bytes,
            b"Status: 200 OK\nCache-Control: no-store, no-cache, must-revalidate\nPragma: no-cache\nContent-Type: application/json\n\n{}"
        );
    }
}
