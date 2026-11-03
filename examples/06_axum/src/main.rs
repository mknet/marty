//! Example **06_axum**: optional Axum [`Router`] behind [`marty::serve_cgi`].
//!
//! Prefer [`examples/02_routing`](../02_routing) (`sync-cgi`) for new shared-hosting apps.
//! Use this when you already have Axum handlers: pass the existing router to `serve_cgi`
//! unchanged. Routes stay as on a regular axum server (`/`, `/hello/{name}`); marty strips the
//! CGI script path (`/cgi-bin/marty-06-axum` or the `.htaccess` alias) before routing.

use axum::Router;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use marty::serve_cgi;

async fn root() -> &'static str {
    "06_axum: root (no path parameters).\n"
}

async fn hello(Path(name): Path<String>) -> impl IntoResponse {
    format!("06_axum: hello, {name}!\n")
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root))
        .route("/hello/{name}", get(hello))
        .fallback(|| async { (StatusCode::NOT_FOUND, "Not found\n") });

    if let Err(e) = serve_cgi(app).await {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
