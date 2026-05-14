//! Example **04_cookies**: `tower_cookies` with Axum behind `marty::serve_cgi`.
//!
//! Sets a response cookie via `CookieManagerLayer` and a small plain-text body.

use axum::{Router, routing::get};
use marty::serve_cgi;
use tower_cookies::{Cookie, CookieManagerLayer, Cookies};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route(
            "/cgi-bin/marty-04-cookies",
            get(|cookies: Cookies| async move {
                cookies.add(Cookie::new("marty_04", "from-marty-04-cookies"));
                "04_cookies: Set-Cookie sent (see response headers).\n".to_string()
            }),
        )
        .layer(CookieManagerLayer::new());

    if let Err(e) = serve_cgi(app).await {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
