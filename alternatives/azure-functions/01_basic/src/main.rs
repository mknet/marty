//! Alternative to Marty [`examples/01_basic`](../../../../examples/01_basic/):
//! smallest HTTP response via **Azure Functions custom handler** (no container).
//!
//! Body from [`marty_demo_logic::basic`] — same string as Marty CGI / AWS Lambda.
//! Route: `GET /api/basic`.

use marty_demo_logic::basic;
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
                .body(String::from(basic::BODY))
        });

    let port: u16 = env::var("FUNCTIONS_CUSTOMHANDLER_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);

    warp::serve(route).run((Ipv4Addr::LOCALHOST, port)).await;
}
