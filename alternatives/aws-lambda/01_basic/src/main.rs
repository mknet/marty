//! Alternative to Marty [`examples/01_basic`](../../../examples/01_basic/):
//! smallest HTTP response via **AWS Lambda Event Function** + Function URL.
//!
//! Body from [`marty_demo_logic::basic`] — same string as the Marty CGI example.

use lambda_http::{run, service_fn, tracing, Body, Error, Request, Response};
use marty_demo_logic::basic;

async fn function_handler(_event: Request) -> Result<Response<Body>, Error> {
    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/plain; charset=utf-8")
        .body(Body::from(basic::BODY))
        .map_err(Box::new)?;
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();
    run(service_fn(function_handler)).await
}
