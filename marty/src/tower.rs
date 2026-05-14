//! Run any [`tower::Service`] (for example an [`axum::Router`](https://docs.rs/axum/latest/axum/struct.Router.html))
//! as a CGI script: parse the request from the process environment, call the service, write the CGI response.
//!
//! Enable with **`marty`’s `tower` feature** in `Cargo.toml` (this also enables **`axum`** for [`multi_mount_cgi_router`]):
//! ```toml
//! marty = { version = "…", features = ["tower"] }
//! ```
//! Then use [`serve_cgi`] (or [`serve_cgi_with_output`]) together with your Axum `Router` or other `Service`.

use crate::{CGIError, CGIRequest, CGIResponse, MetaVariableKind};
use axum::Router;
use http_body_util::{BodyExt, Full};
use hyper::body::{Body, Bytes};
use hyper::{Request, Response};
use snafu::ResultExt;
use std::convert::Infallible;
use std::fmt::{self, Debug, Display};
use std::io::Write;
use tower::{Service, ServiceExt};

/// Serve a CGI application.
///
/// Responses are emitted to stdout per RFC 3875.
pub async fn serve_cgi<S, B>(app: S) -> Result<()>
where
    S: Service<Request<Full<Bytes>>, Response = Response<B>, Error = Infallible>
        + Clone
        + Send
        + 'static,
    B: Body,
    <B as Body>::Error: Debug,
{
    serve_cgi_with_output(std::io::stdout(), app).await
}

/// Serve a CGI application, writing the CGI response to the given writer (e.g. stdout in production).
pub async fn serve_cgi_with_output<S, B>(output: impl Write, app: S) -> Result<()>
where
    S: Service<Request<Full<Bytes>>, Response = Response<B>, Error = Infallible>
        + Clone
        + Send
        + 'static,
    B: Body,
    <B as Body>::Error: Debug,
{
    let request = CGIRequest::<Full<Bytes>>::from_env()
        .and_then(Request::try_from)
        .context(error::CGIRequestParseSnafu)?;

    let response = app
        .oneshot(request)
        .await
        .expect("The Error type is Infallible, this should never fail.");

    let headers = response.headers().clone();
    let status = response.status().to_string();
    let reason = response.status().canonical_reason().map(|s| s.to_string());

    let collected = response.into_body().collect().await;

    let body_bytes = collected.unwrap().to_bytes();

    let cgi_response = CGIResponse {
        headers,
        status,
        reason,
        body: body_bytes,
    };

    cgi_response
        .write_response_to_output(output)
        .context(error::CGIResponseWriteSnafu)
}

/// Join `cgi_prefix` (e.g. `/cgi-bin`) and `binary_name` into one URL path with a single `/` between segments.
fn cgi_script_web_path(cgi_prefix: &str, binary_name: &str) -> String {
    let prefix = cgi_prefix.trim_end_matches('/');
    let name = binary_name.trim_start_matches('/');
    assert!(
        !prefix.is_empty(),
        "multi_mount_cgi_router: cgi_prefix must not be empty"
    );
    assert!(
        !name.is_empty(),
        "multi_mount_cgi_router: binary_name must not be empty"
    );
    format!("{prefix}/{name}")
}

/// Nest the same `inner` [`Router`] at `script_path` and at each path in `aliases` (Axum `nest` + `merge`).
fn nest_router_at_script_and_aliases(inner: Router, script_path: &str, aliases: &[&str]) -> Router {
    let script = script_path.trim_end_matches('/');
    assert!(
        !script.is_empty(),
        "multi_mount_cgi_router: script path must not be empty"
    );

    let mut app = Router::new().nest(script, inner.clone());
    for prefix in aliases {
        let p = prefix.trim_end_matches('/');
        if p.is_empty() {
            continue;
        }
        app = app.merge(Router::new().nest(p, inner.clone()));
    }
    app
}

/// Mount the same Axum [`Router`] at **`/cgi-bin/{binary_name}`** (default) and at each `aliases` prefix.
///
/// Routes on `inner` must be **relative to the mount**: `/`, `/hello/{name}`, …
///
/// For a non-default CGI directory (not `/cgi-bin`), use [`multi_mount_cgi_router_with_prefix`].
pub fn multi_mount_cgi_router(inner: Router, binary_name: &str, aliases: &[&str]) -> Router {
    multi_mount_cgi_router_with_prefix(inner, binary_name, "/cgi-bin", aliases)
}

/// Like [`multi_mount_cgi_router`], but `cgi_prefix` replaces the default `/cgi-bin` segment (no trailing slash).
///
/// The script URL becomes `{cgi_prefix}/{binary_name}`, e.g. `cgi_prefix = "/app/cgi"` and `binary_name = "demo"` → `/app/cgi/demo`.
pub fn multi_mount_cgi_router_with_prefix(
    inner: Router,
    binary_name: &str,
    cgi_prefix: &str,
    aliases: &[&str],
) -> Router {
    let script_path = cgi_script_web_path(cgi_prefix, binary_name);
    nest_router_at_script_and_aliases(inner, &script_path, aliases)
}

/// Split a CGI [`MetaVariableKind::ScriptName`](crate::MetaVariableKind::ScriptName) value into
/// `(cgi_prefix, binary_name)` for [`multi_mount_cgi_router_with_prefix`].
///
/// Examples: `/cgi-bin/app` → `("/cgi-bin", "app")`; `app` → `("/cgi-bin", "app")`; `/app` → `("/cgi-bin", "app")`.
/// Returns [`None`] if the path is empty or ends with `/` with no script segment.
pub fn cgi_mount_parts_from_script_name(script_name: &str) -> Option<(String, String)> {
    let t = script_name.trim();
    if t.is_empty() {
        return None;
    }
    let had_trailing_slash = t.ends_with('/');
    let s = t.trim_end_matches('/');
    if s.is_empty() {
        return None;
    }

    let segments: Vec<&str> = s.split('/').filter(|p| !p.is_empty()).collect();
    match segments.as_slice() {
        [] => None,
        [only] if had_trailing_slash => None,
        [only] => Some(("/cgi-bin".to_string(), (*only).to_string())),
        parts => {
            let (init, last) = parts.split_at(parts.len() - 1);
            let last = last.first().copied()?;
            let prefix = format!("/{}", init.join("/"));
            Some((prefix, last.to_string()))
        }
    }
}

/// Why [`try_cgi_mount_parts_from_env`] or [`multi_mount_cgi_router_try_from_env`] failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgiMountFromEnvError {
    /// `SCRIPT_NAME` was not set.
    ScriptNameMissing,
    /// `SCRIPT_NAME` was not valid UTF-8.
    ScriptNameInvalidUtf8,
    /// `SCRIPT_NAME` was set but could not be split into a usable `(prefix, binary)` pair.
    ScriptNameUnusable,
}

impl Display for CgiMountFromEnvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScriptNameMissing => write!(f, "SCRIPT_NAME is not set"),
            Self::ScriptNameInvalidUtf8 => write!(f, "SCRIPT_NAME is not valid UTF-8"),
            Self::ScriptNameUnusable => write!(f, "SCRIPT_NAME has no usable script segment"),
        }
    }
}

impl std::error::Error for CgiMountFromEnvError {}

/// Reads `SCRIPT_NAME` from the environment and parses it with [`cgi_mount_parts_from_script_name`].
pub fn try_cgi_mount_parts_from_env() -> std::result::Result<(String, String), CgiMountFromEnvError>
{
    let mv = MetaVariableKind::ScriptName
        .from_env()
        .ok_or(CgiMountFromEnvError::ScriptNameMissing)?;
    let path = mv
        .as_str()
        .map_err(|_| CgiMountFromEnvError::ScriptNameInvalidUtf8)?;
    cgi_mount_parts_from_script_name(path).ok_or(CgiMountFromEnvError::ScriptNameUnusable)
}

fn binary_name_from_argv0() -> Option<String> {
    std::env::args_os().next().and_then(|p| {
        std::path::Path::new(&p)
            .file_name()
            .and_then(|f| f.to_str())
            .map(String::from)
    })
}

/// Like [`multi_mount_cgi_router`], but derives `cgi_prefix` and `binary_name` from **`SCRIPT_NAME`**
/// when the process runs as CGI (see [`try_cgi_mount_parts_from_env`]).
///
/// If that fails (e.g. local run without CGI vars), falls back to **`argv[0]`’s file name** and the
/// default `/cgi-bin` prefix — which may differ from production; use an explicit
/// [`multi_mount_cgi_router`] when you need a guaranteed match.
pub fn multi_mount_cgi_router_from_env(inner: Router, aliases: &[&str]) -> Router {
    match try_cgi_mount_parts_from_env() {
        Ok((prefix, name)) => multi_mount_cgi_router_with_prefix(inner, &name, &prefix, aliases),
        Err(_) => {
            let name = binary_name_from_argv0().unwrap_or_else(|| "cgi".to_string());
            multi_mount_cgi_router(inner, &name, aliases)
        }
    }
}

/// Strict variant: requires a usable **`SCRIPT_NAME`** (no `argv[0]` fallback).
pub fn multi_mount_cgi_router_try_from_env(
    inner: Router,
    aliases: &[&str],
) -> std::result::Result<Router, CgiMountFromEnvError> {
    let (prefix, name) = try_cgi_mount_parts_from_env()?;
    Ok(multi_mount_cgi_router_with_prefix(
        inner, &name, &prefix, aliases,
    ))
}

#[cfg(test)]
mod cgi_mount_tests {
    use super::cgi_mount_parts_from_script_name;

    #[test]
    fn script_name_cgi_bin() {
        assert_eq!(
            cgi_mount_parts_from_script_name("/cgi-bin/marty-02-routing"),
            Some(("/cgi-bin".to_string(), "marty-02-routing".to_string()))
        );
    }

    #[test]
    fn script_name_single_segment() {
        assert_eq!(
            cgi_mount_parts_from_script_name("marty-02-routing"),
            Some(("/cgi-bin".to_string(), "marty-02-routing".to_string()))
        );
    }

    #[test]
    fn script_name_root_style() {
        assert_eq!(
            cgi_mount_parts_from_script_name("/marty-02-routing"),
            Some(("/cgi-bin".to_string(), "marty-02-routing".to_string()))
        );
    }

    #[test]
    fn script_name_custom_prefix() {
        assert_eq!(
            cgi_mount_parts_from_script_name("/app/cgi/demo"),
            Some(("/app/cgi".to_string(), "demo".to_string()))
        );
    }

    #[test]
    fn script_name_trailing_slash_trimmed() {
        assert_eq!(
            cgi_mount_parts_from_script_name("/cgi-bin/demo/"),
            Some(("/cgi-bin".to_string(), "demo".to_string()))
        );
    }

    #[test]
    fn script_name_empty_none() {
        assert_eq!(cgi_mount_parts_from_script_name(""), None);
        assert_eq!(cgi_mount_parts_from_script_name("/cgi-bin/"), None);
    }
}

mod error {
    use super::CGIError;
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub))]
    pub enum CgiServiceError {
        #[snafu(display("Failed to parse CGI HTTP request: {}", source))]
        CGIRequestParse { source: CGIError },

        #[snafu(display("Failed to convert HTTP response into CGI response: {}", source))]
        CGIResponseParse { source: CGIError },

        #[snafu(display("Failed to write CGI response: {}", source))]
        CGIResponseWrite { source: CGIError },
    }
}

pub use error::CgiServiceError;

type Result<T> = std::result::Result<T, CgiServiceError>;
