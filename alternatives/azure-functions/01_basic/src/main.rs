//! Alternative to Marty [`examples/01_basic`](../../../../examples/01_basic/):
//! smallest HTTP response via **Azure Functions custom handler** (no container).
//!
//! Same body as Marty (`88 miles per hour!`). The Functions host forwards the raw
//! HTTP request when `enableForwardingHttpRequest` is set (see `host.json`).
//! Route: `GET /api/basic` (folder name under this app).

use std::env;
use std::net::Ipv4Addr;
use warp::http::Response;
use warp::Filter;

#[tokio::main]
async fn main() {
    let route = warp::get()
        .and(warp::path("api"))
        .and(warp::path("basic"))
        .map(|| {
            Response::builder()
                .header("content-type", "text/plain; charset=utf-8")
                .body(String::from("88 miles per hour!"))
        });

    let port: u16 = env::var("FUNCTIONS_CUSTOMHANDLER_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);

    warp::serve(route).run((Ipv4Addr::LOCALHOST, port)).await;
}
