//! Alternative to Marty [`examples/01_basic`](../../../../examples/01_basic/):
//! smallest HTTP response via **Scaleway Serverless Functions** (source zip → managed Rust runtime).
//!
//! Body from [`marty_demo_logic::basic`] — same string as Marty CGI / AWS / Azure / Cloudflare.
//!
//! Scaleway expects a public async handler; deploy packages source (not a prebuilt binary).

use axum::{body::Body, extract::Request, response::Response};
use http::StatusCode;
use marty_demo_logic::basic;

/// Scaleway handler name: `handle`.
pub async fn handle(_req: Request) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Body::from(basic::BODY))
        .unwrap()
}
