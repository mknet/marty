//! Complex bench — sync [`marty::Router`] (Docker fair bench + uploadable hosting binary).

mod json;
mod timing;

use marty::{CgiRequest, CgiResponse, Router};
use marty_bench_shared::compute::{fibonacci_work, matrix_checksum, prime_count};
use marty_bench_shared::salt::{effective_matrix_size, effective_prime_limit, request_salt};
use marty_bench_shared::{
    DEFAULT_MATRIX_SIZE, DEFAULT_PRIME_LIMIT, MAX_MATRIX_SIZE, MAX_PRIME_LIMIT, parse_salt,
    parse_usize_arg, query_param,
};
use std::time::Instant;

fn wants_profile(query_string: &str) -> bool {
    matches!(
        query_param(query_string, "profile"),
        Some("1" | "true" | "yes")
    )
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
            "bench-timing lang=marty-sync summary until_compute_us={:?} startup_us={} compute_us={} total_us={}",
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
            "bench-timing lang=marty-sync summary until_compute_us={:?} startup_us={} compute_us={} total_us={}",
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
    if let Err(e) = resp.write_response_to_stdout() {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
