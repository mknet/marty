//! Phase timings for `?profile=1` (stderr + JSON).
//!
//! In-process phases use a monotonic clock from `process_start()`.
//! With `X-Bench-Sent-Us` (Unix µs from the client), also reports wall-clock
//! deltas from request send → process entry / compute start.

use serde::Serialize;
use std::io::Write;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

static PROCESS_START: OnceLock<Instant> = OnceLock::new();
static UNTIL_PROCESS_US: OnceLock<Option<u128>> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
pub struct PhaseProfile {
    /// Client `X-Bench-Sent-Us` → first line in `main` (fork/exec + network + queue).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until_process_us: Option<u128>,
    /// Client send → just before compute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until_compute_us: Option<u128>,
    /// `main` → handler entered (CGI parse, routing, runtime).
    pub startup_us: u128,
    /// Handler entered → just before compute.
    pub handler_setup_us: u128,
    /// Pure compute (primes / fib / matrix).
    pub compute_us: u128,
    /// After compute → JSON response ready.
    pub post_compute_us: u128,
    /// Process start → response ready.
    pub total_us: u128,
}

pub fn bench_sent_us() -> Option<u64> {
    std::env::var("HTTP_X_BENCH_SENT_US")
        .ok()
        .and_then(|s| s.parse().ok())
}

pub fn since_sent_us(sent_us: u64) -> u128 {
    wall_now_us().saturating_sub(sent_us as u128)
}

fn wall_now_us() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros()
}

pub fn process_start() {
    let now = Instant::now();
    let _ = PROCESS_START.set(now);
    let until = bench_sent_us().map(since_sent_us);
    let _ = UNTIL_PROCESS_US.set(until);
}

fn t0() -> Instant {
    *PROCESS_START.get().unwrap_or(&Instant::now())
}

pub fn elapsed_us() -> u128 {
    t0().elapsed().as_micros()
}

const BENCH_TIMING_LOG: &str = "/var/log/bench-timing/requests.log";

/// When `BENCH_TIMING=1` (bench Docker), append one line: `impl<TAB>route<TAB>pre_compute_us`.
pub fn log_bench_pre_compute(route: &str) {
    if std::env::var("BENCH_TIMING").as_deref() != Ok("1") {
        return;
    }
    use std::fs::OpenOptions;
    use std::io::Write;
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(BENCH_TIMING_LOG)
    {
        let _ = writeln!(f, "marty-complex\t{route}\t{}", elapsed_us());
    }
}

pub fn log_phase(profile: bool, phase: &str) {
    if !profile {
        return;
    }
    let _ = writeln!(
        std::io::stderr(),
        "bench-timing lang=marty-complex phase={phase} elapsed_us={}",
        elapsed_us()
    );
}

pub fn build_profile(
    handler_enter_us: u128,
    pre_compute_us: u128,
    compute_us: u128,
    post_compute_us: u128,
    until_compute_us: Option<u128>,
) -> PhaseProfile {
    PhaseProfile {
        until_process_us: *UNTIL_PROCESS_US.get().unwrap_or(&None),
        until_compute_us,
        startup_us: handler_enter_us,
        handler_setup_us: pre_compute_us.saturating_sub(handler_enter_us),
        compute_us,
        post_compute_us,
        total_us: pre_compute_us + compute_us + post_compute_us,
    }
}
