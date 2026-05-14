//! Example **02_routing**: Axum [`Router`] with two GET routes, served as one CGI binary via [`marty::serve_cgi`].
//!
//! Routes are registered **twice** (same handlers):
//! - **`/cgi-bin/marty-02-routing/…`** — direct CGI URL and what Marty builds from `SCRIPT_NAME` + `PATH_INFO`
//!   (typical after Apache rewrite to the script + tail).
//! - **`/routing/…`** — public path if something upstream leaves `REQUEST_URI` on `/routing/…` without
//!   a usable `PATH_INFO` / normalisation (defensive; cheap duplicate).
//!
//! - `GET …/marty-02-routing` or `…/routing` — no path parameters
//! - `GET …/hello/{name}` — one path segment as parameter
//! - Any other path → **404** with plain-text body `Not found`.

use axum::Router;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::routing::get;
use marty::serve_cgi;

async fn root() -> &'static str {
    "02_routing: root (no path parameters).\n"
}

async fn hello(Path(name): Path<String>) -> String {
    format!("02_routing: hello, {name}!\n")
}

#[tokio::main]
async fn main() {
    let under_cgi = Router::new()
        .route("/cgi-bin/marty-02-routing", get(root))
        .route("/cgi-bin/marty-02-routing/hello/{name}", get(hello));

    let under_pretty = Router::new()
        .route("/routing", get(root))
        .route("/routing/hello/{name}", get(hello));

    let app = under_cgi
        .merge(under_pretty)
        .fallback(|| async { (StatusCode::NOT_FOUND, "Not found\n") });

    if let Err(e) = serve_cgi(app).await {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
