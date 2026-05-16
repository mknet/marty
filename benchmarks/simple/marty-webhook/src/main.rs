//! Benchmark **marty-webhook**: POST webhook (shared secret header + JSON body) via [`marty::serve_cgi`].
//!
//! Same contract as `benchmarks/simple/go-webhook`, `python-webhook`, and `php/webhook.php`.

use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use marty::{multi_mount_cgi_router_from_env, serve_cgi};
use serde::{Deserialize, Serialize};

const WEBHOOK_SECRET: &str = "bench-secret";

#[derive(Debug, Deserialize)]
struct WebhookEvent {
    id: String,
    #[serde(rename = "type")]
    event_type: String,
}

#[derive(Serialize)]
struct WebhookAck {
    received: bool,
    id: String,
}

async fn receive_webhook(request: Request) -> Response {
    if request.method() != axum::http::Method::POST {
        return (StatusCode::METHOD_NOT_ALLOWED, "method not allowed\n").into_response();
    }

    let headers: &HeaderMap = request.headers();
    let secret = headers
        .get("x-webhook-secret")
        .and_then(|v| v.to_str().ok());
    if secret != Some(WEBHOOK_SECRET) {
        return (StatusCode::UNAUTHORIZED, "unauthorized\n").into_response();
    }

    let body = match axum::body::to_bytes(request.into_body(), 1 << 20).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::BAD_REQUEST, "bad request\n").into_response(),
    };

    let evt: WebhookEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(_) => return (StatusCode::BAD_REQUEST, "bad request\n").into_response(),
    };

    let _ = evt.event_type;

    let ack = WebhookAck {
        received: true,
        id: evt.id,
    };

    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&ack).unwrap_or_default(),
    )
        .into_response()
}

#[tokio::main]
async fn main() {
    let inner = Router::new().route("/", post(receive_webhook));

    let app = multi_mount_cgi_router_from_env(inner, &[]);

    if let Err(e) = serve_cgi(app).await {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
