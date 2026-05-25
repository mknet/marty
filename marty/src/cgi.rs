//! Lightweight synchronous CGI request/response types for [`crate::Router`].
//!
//! With feature **`hyper-cgi`**, [`CGIRequest`](crate::CGIRequest) / [`CGIResponse`](crate::CGIResponse) use hyper types for Tower/Axum.
//! With **`sync-cgi`** only (no hyper), types are minimal for small CGI binaries.

use crate::{MetaVariableKind, Result, error};
use snafu::ResultExt;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{Read, Write, stdin};

#[cfg(feature = "hyper-cgi")]
use bytes::Bytes;
#[cfg(feature = "hyper-cgi")]
use hyper::HeaderMap;
#[cfg(feature = "hyper-cgi")]
use hyper::http::HeaderValue;

const STATUS_200_OK: &[u8] = b"Status: 200 OK\n";
const JSON_NO_CACHE_HEADERS: &[u8] =
    b"Cache-Control: no-store, no-cache, must-revalidate\nPragma: no-cache\nContent-Type: application/json\n\n";

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

    #[cfg(feature = "hyper-cgi")]
    pub fn with_header(mut self, name: hyper::header::HeaderName, value: HeaderValue) -> Self {
        self.json_no_cache = false;
        self.headers.insert(name, value);
        self
    }

    /// Write this response as a CGI "document" to `output` (typically stdout).
    pub fn write_response_to_output(self, mut output: impl Write) -> Result<()> {
        #[cfg(feature = "hyper-cgi")]
        let headers_empty = self.headers.is_empty();
        #[cfg(not(feature = "hyper-cgi"))]
        let headers_empty = true;

        if self.json_no_cache && self.status == "200" && headers_empty {
            output
                .write_all(STATUS_200_OK)
                .and_then(|_| output.write_all(JSON_NO_CACHE_HEADERS))
                .and_then(|_| output.write_all(self.body.as_ref()))
                .context(error::WriteResponseSnafu)?;
            return Ok(());
        }

        if let Some(reason) = &self.reason {
            writeln!(output, "Status: {} {}", self.status, reason)
                .context(error::WriteResponseSnafu)?;
        } else {
            writeln!(output, "Status: {}", self.status).context(error::WriteResponseSnafu)?;
        }

        #[cfg(feature = "hyper-cgi")]
        for (key, value) in self.headers.iter() {
            writeln!(output, "{}: {}", key, value.to_str().unwrap_or(""))
                .context(error::WriteResponseSnafu)?;
        }
        output.write_all(b"\n").context(error::WriteResponseSnafu)?;
        output
            .write_all(self.body.as_ref())
            .context(error::WriteResponseSnafu)?;
        Ok(())
    }
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
