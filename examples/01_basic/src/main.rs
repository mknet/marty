//! Example **01_basic**: smallest Marty CGI binary — build a [`marty::CGIResponse`](marty::CGIResponse) and write it to stdout.
//! No `serve_cgi`, no Tower feature, no async runtime: plain synchronous `main`.
//! Body comes from shared [`marty_demo_logic::basic`] (same as cloud alternatives).
//!
//! `.htaccess` / deploy pattern: [`../02_routing/README.md`](../02_routing/README.md).

use bytes::Bytes;
use hyper::HeaderMap;
use hyper::header::CONTENT_TYPE;
use hyper::http::HeaderValue;
use marty::CGIResponse;
use marty_demo_logic::basic;

fn main() {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );

    let response = CGIResponse {
        headers,
        status: "200".into(),
        reason: Some("OK".into()),
        body: Bytes::from_static(basic::BODY.as_bytes()),
    };

    if let Err(e) = response.write_response_to_output(std::io::stdout()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
