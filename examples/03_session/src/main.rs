//! Example **03_session**: `tower_sessions` with Axum behind `marty::serve_cgi`.
//!
//! Uses `MemoryStore` — state lives in this process only. In classic CGI **one process per
//! request**, so this demo focuses on the **session API in a single request** (read counter,
//! increment, write). For session continuity across browser requests behind CGI, use a
//! persistent store (file/DB) or a client-carrying strategy your stack supports.

use axum::{Router, routing::get};
use marty::serve_cgi;
use tower_sessions::cookie::time::Duration;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};

#[tokio::main]
async fn main() {
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_expiry(tower_sessions::Expiry::OnInactivity(Duration::minutes(10)));

    let app = Router::new()
        .route(
            "/cgi-bin/marty-03-session",
            get(|session: Session| async move {
                let count: u32 = session.get("count").await.unwrap().unwrap_or(0);
                let next = count.saturating_add(1);
                session.insert("count", next).await.unwrap();
                format!("03_session: in-request visit counter = {next}\n")
            }),
        )
        .layer(session_layer);

    if let Err(e) = serve_cgi(app).await {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
