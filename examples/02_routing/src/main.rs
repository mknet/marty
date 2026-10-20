//! Example **02_routing**: Axum [`Router`] with two GET routes, served as one CGI binary via [`marty::serve_cgi`].
//!
//! Uses [`marty::multi_mount_cgi_router_from_env`] so **`SCRIPT_NAME`** picks the CGI script URL
//! (here `/cgi-bin/marty-02-routing` under the test server) without hard-coding the binary name;
//! the same routes are also mounted under `/routing` (e.g. after Apache `mod_rewrite`). For a
//! fixed layout without relying on the environment, use [`marty::multi_mount_cgi_router`] or
//! [`marty::multi_mount_cgi_router_with_prefix`].
//!
//! - `GET …/marty-02-routing` or `…/routing` — no path parameters
//! - `GET …/hello/{name}` — one path segment as parameter (URL-decoded by Axum); the response
//!   **HTML-escapes** `name` so arbitrary characters are safe in clients that sniff or mislabel
//!   content type.
//! - Any other path → **404** with plain-text body `Not found`.

use axum::Router;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use marty::{multi_mount_cgi_router_from_env, serve_cgi};

async fn root() -> &'static str {
    "02_routing: root (no path parameters).\n"
}

/// Escape `s` for embedding in HTML/XML text (defense in depth for reflected path segments).
fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::escape_html;

    #[test]
    fn escape_html_biff_link() {
        assert_eq!(
            escape_html(r#"<a href="/">Biff</a>"#),
            "&lt;a href=&quot;/&quot;&gt;Biff&lt;/a&gt;"
        );
    }
}

async fn hello(Path(name): Path<String>) -> impl IntoResponse {
    let safe = escape_html(&name);
    format!("02_routing: hello, {safe}!\n")
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
