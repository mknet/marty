//! Azure Functions twin of [`marty-sync`](../../../../benchmarks/complex/marty-sync/).
//! Routes under `/api/…` (Azure HTTP trigger prefix); same JSON as hosting.
//!
//! `GET /api/primes/{limit}`, `/api/fibonacci/{n}`, `/api/matrix/{size}` (`?salt=`).

use marty_bench_shared::dispatch::run_complex_route;
use std::env;
use std::net::Ipv4Addr;
use warp::http::{header, Response, StatusCode};
use warp::Filter;

#[tokio::main]
async fn main() {
    let query = warp::query::raw()
        .map(Some)
        .or(warp::any().map(|| None::<String>))
        .unify();

    let route = warp::get()
        .and(warp::path::full())
        .and(query)
        .map(|path: warp::path::FullPath, query: Option<String>| {
            let q = query.as_deref().unwrap_or("");
            match run_complex_route(path.as_str(), q) {
                Some(body) => Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::CACHE_CONTROL, "no-store, no-cache, must-revalidate")
                    .body(body)
                    .unwrap(),
                None => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                    .body(
                        b"routes: /api/primes/{limit}, /api/fibonacci/{n}, /api/matrix/{size} (?salt= optional)\n"
                            .to_vec(),
                    )
                    .unwrap(),
            }
        });

    let port: u16 = env::var("FUNCTIONS_CUSTOMHANDLER_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);

    warp::serve(route).run((Ipv4Addr::LOCALHOST, port)).await;
}
