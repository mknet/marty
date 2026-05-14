//! Example **02_routing**: Axum [`Router`] with two GET routes, served as one CGI binary via [`marty::serve_cgi`].
//!
//! Uses [`marty::multi_mount_cgi_router_from_env`] so **`SCRIPT_NAME`** picks the CGI script URL
//! (here `/cgi-bin/marty-02-routing` under the test server) without hard-coding the binary name;
//! the same routes are also mounted under `/routing` (e.g. after Apache `mod_rewrite`). For a
//! fixed layout without relying on the environment, use [`marty::multi_mount_cgi_router`] or
//! [`marty::multi_mount_cgi_router_with_prefix`].
//!
//! - `GET …/marty-02-routing` or `…/routing` — no path parameters
//! - `GET …/hello/{name}` — one path segment as parameter
//! - Any other path → **404** with plain-text body `Not found`.

use axum::Router;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::routing::get;
use marty::{multi_mount_cgi_router_from_env, serve_cgi};

async fn root() -> &'static str {
    "02_routing: root (no path parameters).\n"
}

async fn hello(Path(name): Path<String>) -> String {
    format!("02_routing: hello, {name}!\n")
}

#[tokio::main]
async fn main() {
    let inner = Router::new()
        .route("/", get(root))
        .route("/hello/{name}", get(hello))
        .fallback(|| async { (StatusCode::NOT_FOUND, "Not found\n") });

    let app = multi_mount_cgi_router_from_env(inner, &["/routing"]);

    if let Err(e) = serve_cgi(app).await {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
