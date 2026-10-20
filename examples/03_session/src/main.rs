//! Example **03_session**: HMAC-signed cookie session via sync [`marty::Router`].
//!
//! The session rides in a signed cookie so it survives classic CGI (one process per
//! request). Set `MARTY_SESSION_SECRET` in production; a demo default is used otherwise.

use marty::{CgiRequest, CgiResponse, Router, SessionStore};

fn main() {
    let req = match CgiRequest::from_env() {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Error while reading CGI request: {e}");
            let _ = CgiResponse::new(500, "Internal server error\n").write_response_to_stdout();
            std::process::exit(1);
        }
    };

    let store = SessionStore::new(session_secret()).cookie_name("marty_03");
    let mut session = store.load(&req);
    let count: u32 = session
        .get("count")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let next = count.saturating_add(1);
    session.insert("count", next.to_string());

    let mut router = Router::new();
    router.get("/", move |_| format!("03_session: visit counter = {next}\n"));
    router.fallback(|_| (404u16, "Not found\n"));

    let resp = store.save(&session, router.handle_request(req));
    if let Err(e) = resp.write_response_to_stdout() {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}

fn session_secret() -> Vec<u8> {
    std::env::var("MARTY_SESSION_SECRET")
        .unwrap_or_else(|_| "dev-only-change-me-marty-03".into())
        .into_bytes()
}
