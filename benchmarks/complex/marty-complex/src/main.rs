//! Complex benchmark: CPU-heavy GET routes via PATH_INFO dispatch (standalone sync CGI).

mod cgi_out;
mod compute;
mod json;
mod salt;
mod timing;

use compute::{fibonacci_work, matrix_checksum, prime_count};
use salt::{effective_matrix_size, effective_prime_limit, request_salt};
use std::io;
use std::time::Instant;

const DEFAULT_PRIME_LIMIT: usize = 400_000;
const DEFAULT_MATRIX_SIZE: usize = 128;
const MAX_PRIME_LIMIT: usize = 1_000_000;
const MAX_MATRIX_SIZE: usize = 200;

fn wants_profile(query_string: &str) -> bool {
    query_string.contains("profile=1")
        || query_string.contains("profile=true")
        || query_string.contains("profile=yes")
}

fn parse_salt(query_string: &str) -> Option<u64> {
    for pair in query_string.split('&') {
        if let Some(v) = pair.strip_prefix("salt=") {
            return v.parse().ok();
        }
    }
    None
}

fn path_route_and_arg(path_info: &str) -> (&str, Option<&str>) {
    let p = path_info.trim();
    let p = p.strip_prefix('/').unwrap_or(p);
    let mut parts = p.splitn(2, '/');
    let route = parts.next().unwrap_or("");
    let arg = parts.next().filter(|s| !s.is_empty());
    (route, arg)
}

fn parse_usize_arg(arg: Option<&str>, default: usize, max: usize) -> usize {
    arg.and_then(|s| s.parse().ok())
        .unwrap_or(default)
        .min(max)
}

fn run_route_u64(
    route: &'static str,
    query_string: &str,
    salt: u64,
    compute: impl FnOnce() -> u64,
    json_buf: &mut Vec<u8>,
) -> io::Result<()> {
    let profile_on = wants_profile(query_string);
    let handler_enter_us = timing::elapsed_us();
    timing::log_phase(profile_on, "handler_enter");

    let pre_compute_us = timing::elapsed_us();
    timing::log_bench_pre_compute(route);
    let until_compute_us = timing::bench_sent_us().map(timing::since_sent_us);
    timing::log_phase(profile_on, "pre_compute");
    let compute_start = Instant::now();

    let result = compute();

    let compute_us = compute_start.elapsed().as_micros();
    timing::log_phase(profile_on, "post_compute");

    let profile = profile_on.then(|| {
        let total_us = timing::elapsed_us();
        let post_compute_us = total_us.saturating_sub(pre_compute_us + compute_us);
        let p = timing::build_profile(
            handler_enter_us,
            pre_compute_us,
            compute_us,
            post_compute_us,
            until_compute_us,
        );
        timing::log_phase(true, "response_ready");
        eprintln!(
            "bench-timing lang=marty-complex summary until_compute_us={:?} startup_us={} compute_us={} total_us={}",
            p.until_compute_us, p.startup_us, p.compute_us, p.total_us
        );
        p
    });

    json::write_compute_u64(json_buf, route, salt, result, profile.as_ref());
    cgi_out::write_json_200(json_buf)
}

fn run_route_matrix(
    route: &'static str,
    query_string: &str,
    salt: u64,
    size: usize,
    json_buf: &mut Vec<u8>,
) -> io::Result<()> {
    let profile_on = wants_profile(query_string);
    let handler_enter_us = timing::elapsed_us();
    timing::log_phase(profile_on, "handler_enter");

    let pre_compute_us = timing::elapsed_us();
    timing::log_bench_pre_compute(route);
    let until_compute_us = timing::bench_sent_us().map(timing::since_sent_us);
    timing::log_phase(profile_on, "pre_compute");
    let compute_start = Instant::now();

    let checksum = matrix_checksum(size);

    let compute_us = compute_start.elapsed().as_micros();
    timing::log_phase(profile_on, "post_compute");

    let profile = profile_on.then(|| {
        let total_us = timing::elapsed_us();
        let post_compute_us = total_us.saturating_sub(pre_compute_us + compute_us);
        let p = timing::build_profile(
            handler_enter_us,
            pre_compute_us,
            compute_us,
            post_compute_us,
            until_compute_us,
        );
        timing::log_phase(true, "response_ready");
        eprintln!(
            "bench-timing lang=marty-complex summary until_compute_us={:?} startup_us={} compute_us={} total_us={}",
            p.until_compute_us, p.startup_us, p.compute_us, p.total_us
        );
        p
    });

    json::write_compute_matrix(json_buf, route, salt, size, checksum, profile.as_ref());
    cgi_out::write_json_200(json_buf)
}

fn serve() -> io::Result<()> {
    let method = std::env::var("REQUEST_METHOD").unwrap_or_default();
    if method != "GET" {
        return cgi_out::write_status(405, "Method Not Allowed", b"Method not allowed\n");
    }

    let path_info = std::env::var("PATH_INFO").unwrap_or_default();
    let query_string = std::env::var("QUERY_STRING").unwrap_or_default();
    let (route, arg) = path_route_and_arg(&path_info);
    let salt = request_salt(parse_salt(&query_string));
    let mut json_buf = Vec::with_capacity(128);

    match route {
        "primes" => {
            let base = parse_usize_arg(arg, DEFAULT_PRIME_LIMIT, MAX_PRIME_LIMIT);
            let effective = effective_prime_limit(base, salt, MAX_PRIME_LIMIT);
            run_route_u64("primes", &query_string, salt, || prime_count(effective), &mut json_buf)
        }
        "fibonacci" => {
            run_route_u64(
                "fibonacci",
                &query_string,
                salt,
                || fibonacci_work(salt),
                &mut json_buf,
            )
        }
        "matrix" => {
            let base = parse_usize_arg(arg, DEFAULT_MATRIX_SIZE, MAX_MATRIX_SIZE);
            let effective = effective_matrix_size(base, salt, MAX_MATRIX_SIZE);
            run_route_matrix("matrix", &query_string, salt, effective, &mut json_buf)
        }
        _ => cgi_out::write_status(
            404,
            "Not Found",
            b"routes: /primes/{limit}, /fibonacci/{n}, /matrix/{size} (?salt= &profile=1 optional)\n",
        ),
    }
}

fn main() {
    timing::process_start();
    log_phase_env();

    if let Err(e) = serve() {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}

fn log_phase_env() {
    let profile = std::env::var("QUERY_STRING")
        .map(|q| wants_profile(&q))
        .unwrap_or(false);
    timing::log_phase(profile, "pre_serve");
}
