//! Example **04_cookies**: sync [`Cookie`] / [`CgiResponse::set_cookie`] (no tower-cookies).
//!
//! `.htaccess` / deploy: [`../02_routing/README.md`](../02_routing/README.md).

use marty::{CgiResponse, Cookie, Router};

fn main() {
    let mut router = Router::new();
    router.get("/", |_| {
        CgiResponse::text("04_cookies: Set-Cookie sent (see response headers).\n")
            .set_cookie(Cookie::new("marty_04", "from-marty-04-cookies"))
    });
    router.fallback(|_| (404u16, "Not found\n"));

    if let Err(e) = router.serve() {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
