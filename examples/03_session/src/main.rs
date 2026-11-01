//! Example **03_session**: HMAC-signed cookie session via sync [`marty::Router`].
//!
//! The session rides in a signed cookie so it survives classic CGI (one process per
//! request). The secret is `MARTY_SESSION_SECRET` from the env (local) or from the config
//! file outside the docroot that `cargo marty deploy` uploads (see `marty.conf.example`).
//! Without it the binary answers 500 instead of signing with a public default.
//!
//! `.htaccess` / deploy: [`../02_routing/README.md`](../02_routing/README.md).

use marty::{CgiRequest, CgiResponse, Config, Router, SessionStore};

fn main() {
    let req = match CgiRequest::from_env() {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Error while reading CGI request: {e}");
            let _ = CgiResponse::new(500, "Internal server error\n").write_response_to_stdout();
            std::process::exit(1);
        }
    };

    let store = match Config::load().and_then(|cfg| SessionStore::from_config(&cfg)) {
        Ok(store) => store.cookie_name("marty_03"),
        Err(e) => {
            eprintln!("03_session: {e}");
            let _ = CgiResponse::new(500, "Internal server error\n").write_response_to_stdout();
            std::process::exit(1);
        }
    };
    let mut session = store.load(&req);
    let count: u32 = session
        .get("count")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let next = count.saturating_add(1);
    session.insert("count", next.to_string());

    let mut router = Router::new();
    router.get("/", move |_| {
        format!("03_session: visit counter = {next}\n")
    });
    router.fallback(|_| (404u16, "Not found\n"));

    let resp = store.save(&session, router.handle_request(req));
    if let Err(e) = resp.write_response_to_stdout() {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
