//! Scaleway Functions twin of [`marty-sync`](../../../../benchmarks/complex/marty-sync/).
//! Same primes / fibonacci / matrix JSON via [`marty_bench_shared::dispatch`].

use axum::{body::Body, extract::Request, response::Response};
use http::StatusCode;
use marty_bench_shared::dispatch::run_complex_route;

/// Scaleway handler name: `handle`.
pub async fn handle(req: Request) -> Response {
    let path = req.uri().path();
    let query = req.uri().query().unwrap_or("");
    match run_complex_route(path, query) {
        Some(body) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-store, no-cache, must-revalidate")
            .body(Body::from(body))
            .unwrap(),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(Body::from(
                "routes: /primes/{limit}, /fibonacci/{n}, /matrix/{size} (?salt= optional)\n",
            ))
            .unwrap(),
    }
}
