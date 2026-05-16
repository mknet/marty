//! Complex benchmark: three CPU-heavy GET routes via [`marty::serve_cgi`].
//!
//! Use `?profile=1` for phase timings (stderr + JSON `profile` field). See `just profile`.

mod compute;
mod salt;
mod timing;

use axum::Json;
use axum::Router;
use axum::extract::{Path, Query};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use compute::{fibonacci_work, matrix_checksum, prime_count};
use marty::{multi_mount_cgi_router_from_env, serve_cgi};
use salt::{effective_matrix_size, effective_prime_limit, request_salt};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use timing::{build_profile, log_phase, process_start};

const DEFAULT_PRIME_LIMIT: usize = 400_000;
const DEFAULT_MATRIX_SIZE: usize = 128;
const MAX_PRIME_LIMIT: usize = 1_000_000;
const MAX_MATRIX_SIZE: usize = 200;

#[derive(Debug, Deserialize)]
struct BenchQuery {
    #[serde(default)]
    salt: Option<u64>,
    /// Query `profile=1` (serde bool only accepts true/false, not `1`).
    #[serde(default)]
    profile: Option<String>,
}

#[derive(Serialize)]
struct ComputeResponse {
    route: &'static str,
    salt: u64,
    result: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<timing::PhaseProfile>,
}

fn wants_profile(q: &BenchQuery) -> bool {
    matches!(
        q.profile.as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

fn no_cache_json<T: Serialize>(body: T) -> Response {
    (
        StatusCode::OK,
        [
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("no-store, no-cache, must-revalidate"),
            ),
            (header::PRAGMA, HeaderValue::from_static("no-cache")),
            (header::CONTENT_TYPE, HeaderValue::from_static("application/json")),
        ],
        Json(body),
    )
        .into_response()
}

fn run_route<F>(route: &'static str, q: &BenchQuery, compute: F) -> Response
where
    F: FnOnce() -> serde_json::Value,
{
    let profile_on = wants_profile(q);
    let handler_enter_us = timing::elapsed_us();
    log_phase(profile_on, "handler_enter");

    let pre_compute_us = timing::elapsed_us();
    timing::log_bench_pre_compute(route);
    let until_compute_us = timing::bench_sent_us().map(timing::since_sent_us);
    log_phase(profile_on, "pre_compute");
    let compute_start = Instant::now();

    let result = compute();

    let compute_us = compute_start.elapsed().as_micros();
    log_phase(profile_on, "post_compute");

    let salt = request_salt(q.salt);
    let phase = profile_on.then(|| {
        let total_us = timing::elapsed_us();
        let post_compute_us = total_us.saturating_sub(pre_compute_us + compute_us);
        let p = build_profile(
            handler_enter_us,
            pre_compute_us,
            compute_us,
            post_compute_us,
            until_compute_us,
        );
        log_phase(true, "response_ready");
        eprintln!(
            "bench-timing lang=marty-complex summary until_compute_us={:?} startup_us={} compute_us={} total_us={}",
            p.until_compute_us, p.startup_us, p.compute_us, p.total_us
        );
        p
    });

    no_cache_json(ComputeResponse {
        route,
        salt,
        result,
        profile: phase,
    })
}

async fn route_primes(Path(limit): Path<usize>, Query(q): Query<BenchQuery>) -> Response {
    let base = limit.min(MAX_PRIME_LIMIT);
    let salt = request_salt(q.salt);
    let effective = effective_prime_limit(base, salt, MAX_PRIME_LIMIT);
    run_route("primes", &q, || prime_count(effective).into())
}

async fn route_fibonacci(Path(_n): Path<u32>, Query(q): Query<BenchQuery>) -> Response {
    let salt = request_salt(q.salt);
    run_route("fibonacci", &q, || fibonacci_work(salt).into())
}

async fn route_matrix(Path(size): Path<usize>, Query(q): Query<BenchQuery>) -> Response {
    let base = size.min(MAX_MATRIX_SIZE);
    let salt = request_salt(q.salt);
    let effective = effective_matrix_size(base, salt, MAX_MATRIX_SIZE);
    run_route("matrix", &q, || {
        serde_json::json!({ "size": effective, "checksum": matrix_checksum(effective) })
    })
}

async fn route_primes_default(Query(q): Query<BenchQuery>) -> Response {
    route_primes(Path(DEFAULT_PRIME_LIMIT), Query(q)).await
}

async fn route_fibonacci_default(Query(q): Query<BenchQuery>) -> Response {
    route_fibonacci(Path(42), Query(q)).await
}

async fn route_matrix_default(Query(q): Query<BenchQuery>) -> Response {
    route_matrix(Path(DEFAULT_MATRIX_SIZE), Query(q)).await
}

async fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        [(header::CACHE_CONTROL, HeaderValue::from_static("no-store"))],
        "routes: /primes/{limit}, /fibonacci/{n}, /matrix/{size} (?salt= &profile=1 optional)\n",
    )
        .into_response()
}

#[tokio::main]
async fn main() {
    process_start();

    let inner = Router::new()
        .route("/primes/{limit}", get(route_primes))
        .route("/fibonacci/{n}", get(route_fibonacci))
        .route("/matrix/{size}", get(route_matrix))
        .route("/primes", get(route_primes_default))
        .route("/fibonacci", get(route_fibonacci_default))
        .route("/matrix", get(route_matrix_default))
        .fallback(not_found);

    let app = multi_mount_cgi_router_from_env(inner, &[]);

    log_phase(
        std::env::var("QUERY_STRING")
            .map(|q| q.contains("profile=1"))
            .unwrap_or(false),
        "pre_serve_cgi",
    );

    if let Err(e) = serve_cgi(app).await {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
