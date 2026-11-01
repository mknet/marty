//! Marty playground: two routes plus a visit counter in a signed cookie session.
//!
//! The session secret comes from `MARTY_SESSION_SECRET` (env, local) or from the config
//! file that `cargo marty deploy` uploads outside the docroot. Without it `/visits`
//! answers 500, which is the quickest way to see whether the host can read that file.

use marty::{CgiRequest, CgiResponse, Config, Router, SessionStore};

fn main() {
    let mut router = Router::new();
    router.get("/", |_| "Hello from the Marty playground!\n");
    router.get("/hello/{name}", |req| {
        let name = req.param("name").unwrap_or("world");
        format!("Hello, {name}!\n")
    });
    router.get("/visits", visits);
    router.fallback(|_| (404u16, "Not found\n"));

    if let Err(e) = router.serve() {
        eprintln!("playground: {e}");
        std::process::exit(1);
    }
}

fn visits(req: &CgiRequest) -> CgiResponse {
    let store = match Config::load().and_then(|cfg| SessionStore::from_config(&cfg)) {
        Ok(store) => store.cookie_name("marty_playground").secure(true),
        Err(e) => {
            eprintln!("playground: {e}");
            return CgiResponse::new(500, "config missing, see the CGI error log\n");
        }
    };
    let mut session = store.load(req);
    let n: u32 = session.get("n").and_then(|s| s.parse().ok()).unwrap_or(0) + 1;
    session.insert("n", n.to_string());
    store.save(&session, CgiResponse::text(format!("visit #{n}\n")))
}
