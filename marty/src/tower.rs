//! Run any [`tower::Service`] (for example an [`axum::Router`](https://docs.rs/axum/latest/axum/struct.Router.html))
//! as a CGI script: parse the request from the process environment, call the service, write the CGI response.
//!
//! Enable with **`marty`’s `tower` feature** in `Cargo.toml`:
//! ```toml
//! marty = { version = "…", features = ["tower"] }
//! ```
//! Then use [`serve_cgi`] (or [`serve_cgi_with_output`]) together with your Axum `Router` or other `Service`.

use crate::{CGIError, CGIRequest, CGIResponse};
use http_body_util::{BodyExt, Full};
use hyper::body::{Body, Bytes};
use hyper::{Request, Response};
use snafu::ResultExt;
use std::convert::Infallible;
use std::fmt::Debug;
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
