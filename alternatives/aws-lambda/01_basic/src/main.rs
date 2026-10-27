//! Alternative to Marty [`examples/01_basic`](../../../examples/01_basic/):
//! smallest HTTP response via **AWS Lambda Event Function** + Function URL.
//!
//! Same body as Marty (`88 miles per hour!`), different runtime (Lambda vs CGI).

use lambda_http::{run, service_fn, tracing, Body, Error, Request, Response};

async fn function_handler(_event: Request) -> Result<Response<Body>, Error> {
    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/plain; charset=utf-8")
        .body(Body::from("88 miles per hour!"))
        .map_err(Box::new)?;
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();
    run(service_fn(function_handler)).await
}
