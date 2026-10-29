//! Example **02_routing**: sync [`marty::Router`] — recommended path for shared hosting.
//!
//! - `GET …/marty-02-routing` — no path parameters
//! - `GET …/hello/{name}` — one path segment (percent-decoded); response **HTML-escapes** `name`
//! - Any other path → **404** with plain-text body `Not found`
//!
//! Deploy (`.htaccess` from `[package.metadata.marty]`, musl upload): see [`README.md`](../README.md)
//! and the root README *Quick start*.
//!
//! For existing Axum handlers, see `examples/06_axum`.

use marty::{CgiRequest, Router};

fn root(_req: &CgiRequest) -> &'static str {
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

fn hello(req: &CgiRequest) -> String {
    let name = req.param("name").unwrap_or("world");
    let safe = escape_html(name);
    format!("02_routing: hello, {safe}!\n")
}

fn main() {
    let mut router = Router::new();
    router.get("/", root);
    router.get("/hello/{name}", hello);
    router.fallback(|_| (404u16, "Not found\n"));

    if let Err(e) = router.serve() {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
