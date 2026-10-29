//! AWS Lambda twin of [`marty-sync`](../../../benchmarks/complex/marty-sync/):
//! same primes / fibonacci / matrix JSON via [`marty_bench_shared::dispatch`].
//!
//! Function URL paths: `/primes/{limit}`, `/fibonacci/{n}`, `/matrix/{size}` (`?salt=`).

use lambda_http::{run, service_fn, tracing, Body, Error, Request, Response};
use marty_bench_shared::dispatch::run_complex_route;

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let path = event.uri().path();
    let query = event.uri().query().unwrap_or("");
    match run_complex_route(path, query) {
        Some(body) => Ok(Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .header("cache-control", "no-store, no-cache, must-revalidate")
            .body(Body::from(body))
            .map_err(Box::new)?),
        None => Ok(Response::builder()
            .status(404)
            .header("content-type", "text/plain; charset=utf-8")
            .body(Body::from(
                "routes: /primes/{limit}, /fibonacci/{n}, /matrix/{size} (?salt= optional)\n",
            ))
            .map_err(Box::new)?),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();
    run(service_fn(function_handler)).await
}
