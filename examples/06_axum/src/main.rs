//! Example **06_axum**: optional Axum [`Router`] behind [`marty::serve_cgi`].
//!
//! Prefer [`examples/02_routing`](../02_routing) (`sync-cgi`) for new shared-hosting apps.
//! Use this when you already have Axum handlers and want to run them as CGI.

use axum::Router;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use marty::{multi_mount_cgi_router_from_env, serve_cgi};

async fn root() -> &'static str {
    "06_axum: root (no path parameters).\n"
}

async fn hello(Path(name): Path<String>) -> impl IntoResponse {
    format!("06_axum: hello, {name}!\n")
}

#[tokio::main]
async fn main() {
    let inner = Router::new()
        .route("/", get(root))
        .route("/hello/{name}", get(hello))
        .fallback(|| async { (StatusCode::NOT_FOUND, "Not found\n") });

    let app = multi_mount_cgi_router_from_env(inner, &[]);

    if let Err(e) = serve_cgi(app).await {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
