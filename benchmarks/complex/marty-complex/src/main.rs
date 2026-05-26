//! Complex benchmark: CPU-heavy GET routes through the `marty` sync CGI router.

mod compute;
mod json;
mod salt;
mod timing;

use compute::{fibonacci_work, matrix_checksum, prime_count};
use marty::{CgiRequest, CgiResponse, Router};
use salt::{effective_matrix_size, effective_prime_limit, request_salt};
use std::ffi::OsStr;
use std::time::Instant;

const DEFAULT_PRIME_LIMIT: usize = 400_000;
const DEFAULT_MATRIX_SIZE: usize = 128;
const MAX_PRIME_LIMIT: usize = 1_000_000;
const MAX_MATRIX_SIZE: usize = 200;

fn query_param<'a>(query_string: &'a str, name: &str) -> Option<&'a str> {
    query_string.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then_some(value)
    })
}

fn wants_profile(query_string: &str) -> bool {
    matches!(
        query_param(query_string, "profile"),
        Some("1" | "true" | "yes")
    )
}

fn parse_salt(query_string: &str) -> Option<u64> {
    query_param(query_string, "salt").and_then(parse_u64_ascii)
}

fn parse_u64_ascii(s: &str) -> Option<u64> {
    if s.is_empty() {
        return None;
    }
    let mut n = 0u64;
    for b in s.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?.checked_add(u64::from(b - b'0'))?;
    }
    Some(n)
}

fn parse_usize_arg(arg: Option<&str>, default: usize, max: usize) -> usize {
    arg.and_then(parse_u64_ascii)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(default)
        .min(max)
}

fn run_route_u64(
    route: &'static str,
    profile_on: bool,
    bench_timing: bool,
    salt: u64,
    compute: impl FnOnce() -> u64,
) -> CgiResponse {
    let mut json_buf = Vec::with_capacity(256);

    if !profile_on {
        timing::log_bench_pre_compute(bench_timing, route);
        let result = compute();
        json::write_compute_u64(&mut json_buf, route, salt, result, None);
        return CgiResponse::json_no_cache_bytes(json_buf);
    }

    let handler_enter_us = timing::elapsed_us();
    timing::log_phase(profile_on, "handler_enter");

    let pre_compute_us = timing::elapsed_us();
    timing::log_bench_pre_compute(bench_timing, route);
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

    json::write_compute_u64(&mut json_buf, route, salt, result, profile.as_ref());
    CgiResponse::json_no_cache_bytes(json_buf)
}

fn run_route_matrix(
    route: &'static str,
    profile_on: bool,
    bench_timing: bool,
    salt: u64,
    size: usize,
) -> CgiResponse {
    let mut json_buf = Vec::with_capacity(256);

    if !profile_on {
        timing::log_bench_pre_compute(bench_timing, route);
        let checksum = matrix_checksum(size);
        json::write_compute_matrix(&mut json_buf, route, salt, size, checksum, None);
        return CgiResponse::json_no_cache_bytes(json_buf);
    }

    let handler_enter_us = timing::elapsed_us();
    timing::log_phase(profile_on, "handler_enter");

    let pre_compute_us = timing::elapsed_us();
    timing::log_bench_pre_compute(bench_timing, route);
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

    json::write_compute_matrix(&mut json_buf, route, salt, size, checksum, profile.as_ref());
    CgiResponse::json_no_cache_bytes(json_buf)
}

fn route_salt(req: &CgiRequest) -> u64 {
    request_salt(parse_salt(&req.query_string))
}

fn handle_primes(req: &CgiRequest, bench_timing: bool) -> CgiResponse {
    let profile_on = wants_profile(&req.query_string);
    let salt = route_salt(req);
    let base = parse_usize_arg(req.param("limit"), DEFAULT_PRIME_LIMIT, MAX_PRIME_LIMIT);
    let effective = effective_prime_limit(base, salt, MAX_PRIME_LIMIT);
    run_route_u64("primes", profile_on, bench_timing, salt, || {
        prime_count(effective)
    })
}

fn handle_fibonacci(req: &CgiRequest, bench_timing: bool) -> CgiResponse {
    let profile_on = wants_profile(&req.query_string);
    let salt = route_salt(req);
    run_route_u64("fibonacci", profile_on, bench_timing, salt, || {
        fibonacci_work(salt)
    })
}

fn handle_matrix(req: &CgiRequest, bench_timing: bool) -> CgiResponse {
    let profile_on = wants_profile(&req.query_string);
    let salt = route_salt(req);
    let base = parse_usize_arg(req.param("size"), DEFAULT_MATRIX_SIZE, MAX_MATRIX_SIZE);
    let effective = effective_matrix_size(base, salt, MAX_MATRIX_SIZE);
    run_route_matrix("matrix", profile_on, bench_timing, salt, effective)
}

fn fallback(req: &CgiRequest) -> CgiResponse {
    if req.method != "GET" {
        return CgiResponse::new(405, "Method not allowed\n");
    }

    CgiResponse::new(
        404,
        "routes: /primes/{limit}, /fibonacci/{n}, /matrix/{size} (?salt= &profile=1 optional)\n",
    )
}

fn router(bench_timing: bool) -> Router {
    let mut router = Router::new();
    router.get("/primes/{limit}", move |req| {
        handle_primes(req, bench_timing)
    });
    router.get("/fibonacci/{n}", move |req| {
        handle_fibonacci(req, bench_timing)
    });
    router.get("/matrix/{size}", move |req| {
        handle_matrix(req, bench_timing)
    });
    router.fallback(fallback);
    router
}

fn split_response_enabled() -> bool {
    std::env::var_os("MARTY_COMPLEX_SPLIT_RESPONSE").as_deref() == Some(OsStr::new("1"))
}

fn main() {
    let req = match CgiRequest::from_env() {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Error while reading CGI request: {e}");
            let _ = CgiResponse::new(500, "Internal server error\n").write_response_to_stdout();
            std::process::exit(1);
        }
    };

    let profile_on = wants_profile(&req.query_string);
    let bench_timing = timing::bench_timing_enabled();
    timing::process_start(profile_on);
    timing::log_phase(profile_on, "pre_serve");

    let resp = router(bench_timing).handle_request(req);
    let result = if split_response_enabled() {
        resp.write_response_to_stdout_split()
    } else {
        resp.write_response_to_stdout()
    };
    if let Err(e) = result {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
