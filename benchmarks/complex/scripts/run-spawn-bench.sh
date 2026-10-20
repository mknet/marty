#!/usr/bin/env bash
# Direct process-spawn benchmark for complex CGI binaries (no Apache, no TCP).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
BIN_DIR="${BENCH_SPAWN_BIN_DIR:-$HERE/.spawn-bin}"
N="${BENCH_SPAWN_REQUESTS:-2000}"
WARMUP="${BENCH_SPAWN_WARMUP:-20}"
ROUTES="${BENCH_SPAWN_ROUTES:-primes/400000 fibonacci/42 matrix/128}"

build_binaries() {
  mkdir -p "$BIN_DIR"

  cd "$ROOT"
  local rustflags="${MARTY_RUSTFLAGS:-}"
  if [[ -n "${MARTY_TARGET_CPU:-}" ]]; then
    rustflags="${rustflags} -Ctarget-cpu=${MARTY_TARGET_CPU}"
  fi
  if [[ -n "${MARTY_PGO_PROFILE:-}" ]]; then
    rustflags="${rustflags} -Cprofile-use=${MARTY_PGO_PROFILE}"
  fi
  if [[ -n "$rustflags" ]]; then
    echo ">> Rust RUSTFLAGS=${rustflags}"
  fi
  echo ">> building Rust marty-sync"
  CARGO_INCREMENTAL=0 RUSTFLAGS="$rustflags" cargo build -p marty-sync --profile bench

  local target_dir
  target_dir="$(cargo metadata --format-version=1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
  cp -f "$target_dir/release/marty-sync" "$BIN_DIR/marty-sync"
  chmod +x "$BIN_DIR/marty-sync"

  echo ">> building Go go-complex"
  (
    cd "$HERE/go-complex"
    CGO_ENABLED="${GO_CGO_ENABLED:-0}" go build -trimpath -ldflags="-s -w" -o "$BIN_DIR/go-complex" .
  )
  chmod +x "$BIN_DIR/go-complex"
}

run_bench() {
  python3 - "$BIN_DIR" "$N" "$WARMUP" $ROUTES <<'PY'
import os
import statistics
import subprocess
import sys
import time

bin_dir = sys.argv[1]
n = int(sys.argv[2])
warmup = int(sys.argv[3])
routes = sys.argv[4:]

handlers = [
    ("marty", os.path.join(bin_dir, "marty-sync")),
    ("go", os.path.join(bin_dir, "go-complex")),
]

base_env = {
    "REQUEST_METHOD": "GET",
    "SERVER_PROTOCOL": "HTTP/1.1",
    "SERVER_SOFTWARE": "spawn-bench",
    "SERVER_NAME": "localhost",
    "SERVER_PORT": "80",
    "GATEWAY_INTERFACE": "CGI/1.1",
    "REMOTE_ADDR": "127.0.0.1",
    "BENCH_TIMING": "0",
}

def env_for(handler_name, route):
    env = os.environ.copy()
    env.update(base_env)
    env["SCRIPT_NAME"] = f"/cgi-bin/{handler_name}-complex"
    env["PATH_INFO"] = f"/{route}"
    env["QUERY_STRING"] = ""
    return env

def percentile(values, pct):
    if not values:
        return 0.0
    ordered = sorted(values)
    idx = int((len(ordered) - 1) * pct / 100)
    return ordered[idx]

def validate(handler, binary, route):
    completed = subprocess.run(
        [binary],
        env=env_for(handler, route),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        sys.stderr.write(completed.stderr.decode(errors="replace"))
        raise SystemExit(f"{handler} {route} exited with {completed.returncode}")
    if b"Status: 200" not in completed.stdout:
        sample = completed.stdout[:300].decode(errors="replace")
        raise SystemExit(f"{handler} {route} did not emit a 200 CGI response:\n{sample}")

def run_one(handler, binary, route):
    env = env_for(handler, route)
    for _ in range(warmup):
        subprocess.run([binary], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)

    samples = []
    start_total = time.perf_counter_ns()
    for _ in range(n):
        start = time.perf_counter_ns()
        subprocess.run([binary], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
        samples.append((time.perf_counter_ns() - start) / 1_000_000)
    total_ms = (time.perf_counter_ns() - start_total) / 1_000_000
    return {
        "avg": statistics.fmean(samples),
        "p50": statistics.median(samples),
        "p95": percentile(samples, 95),
        "p99": percentile(samples, 99),
        "fast": min(samples),
        "slow": max(samples),
        "total": total_ms,
        "rps": n / (total_ms / 1000),
    }

print()
print("Complex direct spawn benchmark (no Apache, no TCP)")
print(f"  Requests: {n} process invocations per handler x route")
print(f"  Warm-up:  {warmup} process invocations before timing")
print(f"  Routes:   {', '.join(routes)}")
print()
print("══════════════════════════════════════════════════════════════════════════════════════")
print(f"{'Handler':<18} {'Total':>9} {'Avg':>8} {'P50':>8} {'P95':>8} {'P99':>8} {'Fast':>8} {'Slow':>8} {'exec/s':>8}")
print("──────────────────────────────────────────────────────────────────────────────────────")

for route in routes:
    for handler, binary in handlers:
        validate(handler, binary, route)
        result = run_one(handler, binary, route)
        print(
            f"{handler + ' /' + route.split('/')[0]:<18} "
            f"{result['total']:8.1f} "
            f"{result['avg']:7.2f} "
            f"{result['p50']:7.2f} "
            f"{result['p95']:7.2f} "
            f"{result['p99']:7.2f} "
            f"{result['fast']:7.2f} "
            f"{result['slow']:7.2f} "
            f"{result['rps']:7.2f}"
        )

print("══════════════════════════════════════════════════════════════════════════════════════")
print()
print("All latency columns are milliseconds per direct binary invocation.")
PY
}

build_binaries
run_bench
