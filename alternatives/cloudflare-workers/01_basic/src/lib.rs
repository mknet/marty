//! Alternative to Marty [`examples/01_basic`](../../../../examples/01_basic/):
//! smallest HTTP response via **Cloudflare Workers** (Rust compiled to WASM).
//!
//! Body from [`marty_demo_logic::basic`] — same string as Marty CGI / AWS / Azure.

use marty_demo_logic::basic;
use worker::*;

#[event(fetch)]
async fn fetch(_req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    Response::ok(basic::BODY)
}
