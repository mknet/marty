//! Cloudflare Workers twin of [`marty-sync`](../../../../benchmarks/complex/marty-sync/).
//! Same primes / fibonacci / matrix JSON via [`marty_bench_shared::dispatch`].
//!
//! **CPU time:** Workers free tier caps CPU (~10 ms). Matrix/128 may 5xx on free;
//! paid plans are higher. Fair compare does not shrink the workload to fit WASM.

use marty_bench_shared::dispatch::run_complex_route;
use worker::*;

#[event(fetch)]
async fn fetch(req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    if req.method() != Method::Get {
        return Response::error("GET only", 405);
    }
    let url = req.url()?;
    let path = url.path();
    let query = url.query().unwrap_or("");
    match run_complex_route(path, query) {
        Some(body) => {
            let headers = Headers::new();
            headers.set("content-type", "application/json")?;
            headers.set("cache-control", "no-store, no-cache, must-revalidate")?;
            Ok(Response::from_bytes(body)?.with_headers(headers))
        }
        None => Response::error(
            "routes: /primes/{limit}, /fibonacci/{n}, /matrix/{size} (?salt= optional)\n",
            404,
        ),
    }
}
