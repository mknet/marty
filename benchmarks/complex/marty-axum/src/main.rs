//! Complex bench — Axum [`Router`] behind [`marty::serve_cgi`] (upload A/B vs `marty-sync`).

use axum::Router;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;
use marty::{multi_mount_cgi_router_from_env, serve_cgi};
use marty_bench_shared::compute::{fibonacci_work, matrix_checksum, prime_count};
use marty_bench_shared::json::{write_compute_matrix, write_compute_u64};
use marty_bench_shared::salt::{effective_matrix_size, effective_prime_limit, request_salt};
use marty_bench_shared::{
    DEFAULT_MATRIX_SIZE, DEFAULT_PRIME_LIMIT, MAX_MATRIX_SIZE, MAX_PRIME_LIMIT, parse_u64_ascii,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SaltQuery {
    salt: Option<String>,
}

fn salt_from_query(q: &SaltQuery) -> u64 {
    request_salt(q.salt.as_deref().and_then(parse_u64_ascii))
}

fn json_no_cache(body: Vec<u8>) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    );
    headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    (headers, body)
}

async fn primes(Path(limit): Path<String>, Query(q): Query<SaltQuery>) -> impl IntoResponse {
    let salt = salt_from_query(&q);
    let base = parse_u64_ascii(&limit)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(DEFAULT_PRIME_LIMIT)
        .min(MAX_PRIME_LIMIT);
    let effective = effective_prime_limit(base, salt, MAX_PRIME_LIMIT);
    let result = prime_count(effective);
    let mut buf = Vec::with_capacity(128);
    write_compute_u64(&mut buf, "primes", salt, result);
    json_no_cache(buf)
}

async fn fibonacci(Path(_n): Path<String>, Query(q): Query<SaltQuery>) -> impl IntoResponse {
    let salt = salt_from_query(&q);
    let result = fibonacci_work(salt);
    let mut buf = Vec::with_capacity(128);
    write_compute_u64(&mut buf, "fibonacci", salt, result);
    json_no_cache(buf)
}

async fn matrix(Path(size): Path<String>, Query(q): Query<SaltQuery>) -> impl IntoResponse {
    let salt = salt_from_query(&q);
    let base = parse_u64_ascii(&size)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(DEFAULT_MATRIX_SIZE)
        .min(MAX_MATRIX_SIZE);
    let n = effective_matrix_size(base, salt, MAX_MATRIX_SIZE);
    let checksum = matrix_checksum(n);
    let mut buf = Vec::with_capacity(192);
    write_compute_matrix(&mut buf, "matrix", salt, n, checksum);
    json_no_cache(buf)
}

#[tokio::main]
async fn main() {
    let inner = Router::new()
        .route("/primes/{limit}", get(primes))
        .route("/fibonacci/{n}", get(fibonacci))
        .route("/matrix/{size}", get(matrix))
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                "routes: /primes/{limit}, /fibonacci/{n}, /matrix/{size} (?salt= optional)\n",
            )
        });

    let app = multi_mount_cgi_router_from_env(inner, &[]);

    if let Err(e) = serve_cgi(app).await {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
