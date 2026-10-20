#!/usr/bin/env bash
# Fair complex benchmark: one Apache container, N GETs per (language × route), summary table.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$HERE/docker-compose.yml"
COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-marty-bench-complex}"
BASE="${BENCH_BASE_URL:-http://127.0.0.1:8081}"
PROBE_URL="${BASE}/cgi-bin/marty-complex/primes/10000?salt=0"
N="${BENCH_REQUESTS:-2000}"
C="${BENCH_CONCURRENCY:-10}"
PAUSE="${BENCH_PAUSE_SEC:-1}"
RUNS="${BENCH_RUNS:-1}"
BENCH_TIMING="${BENCH_TIMING:-1}"
WARMUP_N="${BENCH_WARMUP_REQUESTS:-0}"
BENCH_LANGS="${BENCH_LANGS:-marty go python php}"
MARTY_COMPLEX_SPLIT_RESPONSE="${MARTY_COMPLEX_SPLIT_RESPONSE:-0}"
GO_COMPLEX_RAW_CGI="${GO_COMPLEX_RAW_CGI:-0}"
# Dockerfile selection (compose interpolates it): default prefork/mod_cgi/mod_php image,
# or docker/Dockerfile.hosting for the hosting-like mpm_event/mod_cgid/suexec/php-fpm image.
BENCH_DOCKERFILE="${BENCH_DOCKERFILE:-benchmarks/complex/docker/Dockerfile}"
export BENCH_DOCKERFILE

# shellcheck source=../../simple/scripts/ensure-hey.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/../../simple/scripts/ensure-hey.sh"
# shellcheck source=docker-lib.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/docker-lib.sh"
ensure_hey

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker not on PATH" >&2
  exit 1
fi

PRIME_LIMIT="${BENCH_PRIME_LIMIT:-400000}"
FIB_N="${BENCH_FIB_N:-42}"
MATRIX_SIZE="${BENCH_MATRIX_SIZE:-128}"

wait_ready() {
  local url="$1"
  local i
  for i in $(seq 1 60); do
    if curl -sf -o /dev/null -m 5 "$url"; then
      return 0
    fi
    sleep 1
  done
  echo "error: timeout waiting for $url" >&2
  return 1
}

dc() {
  docker compose -p "$COMPOSE_PROJECT_NAME" -f "$COMPOSE_FILE" "$@"
}

start_stack() {
  local port
  port="$(bench_host_port "$BASE")"
  export BENCH_PORT="$port"
  export BENCH_TIMING
  export MARTY_COMPLEX_SPLIT_RESPONSE
  export GO_COMPLEX_RAW_CGI
  stop_docker_on_port "$port"
  dc down --remove-orphans 2>/dev/null || true
  echo ">> docker compose up -d (project ${COMPOSE_PROJECT_NAME}, host :${port}, BENCH_TIMING=${BENCH_TIMING}, MARTY_COMPLEX_SPLIT_RESPONSE=${MARTY_COMPLEX_SPLIT_RESPONSE}, GO_COMPLEX_RAW_CGI=${GO_COMPLEX_RAW_CGI})"
  dc up -d
}

ensure_apache() {
  cd "$HERE"
  export BENCH_PORT="$(bench_host_port "$BASE")"
  export BENCH_TIMING
  export MARTY_COMPLEX_SPLIT_RESPONSE
  export GO_COMPLEX_RAW_CGI

  if [[ "${BENCH_REBUILD:-0}" == "1" ]]; then
    echo ">> docker compose build (BENCH_REBUILD=1)"
    dc build
    echo ">> recreating container so new image is used"
    start_stack
    return
  fi

  local need_recreate=0
  if [[ -n "$(dc ps -q bench 2>/dev/null)" ]]; then
    local cur_timing
    local cur_marty_split
    local cur_go_raw_cgi
    cur_timing="$(dc exec -T bench printenv BENCH_TIMING 2>/dev/null || true)"
    cur_marty_split="$(dc exec -T bench printenv MARTY_COMPLEX_SPLIT_RESPONSE 2>/dev/null || true)"
    cur_go_raw_cgi="$(dc exec -T bench printenv GO_COMPLEX_RAW_CGI 2>/dev/null || true)"
    if [[ "${cur_timing:-}" != "${BENCH_TIMING}" ]] || [[ "${cur_marty_split:-}" != "${MARTY_COMPLEX_SPLIT_RESPONSE}" ]] || [[ "${cur_go_raw_cgi:-}" != "${GO_COMPLEX_RAW_CGI}" ]]; then
      need_recreate=1
    fi
  fi

  if [[ "$need_recreate" == "1" ]]; then
    echo ">> container env changed (BENCH_TIMING ${cur_timing:-unset} -> ${BENCH_TIMING}, MARTY_COMPLEX_SPLIT_RESPONSE ${cur_marty_split:-unset} -> ${MARTY_COMPLEX_SPLIT_RESPONSE}, GO_COMPLEX_RAW_CGI ${cur_go_raw_cgi:-unset} -> ${GO_COMPLEX_RAW_CGI}), recreating container"
    start_stack
    return
  fi

  if [[ -n "$(dc ps -q bench 2>/dev/null)" ]] && curl -sf -o /dev/null -m 5 "$PROBE_URL"; then
    echo ">> reusing running complex bench container on ${BASE} (probe ok, BENCH_TIMING=${BENCH_TIMING}, MARTY_COMPLEX_SPLIT_RESPONSE=${MARTY_COMPLEX_SPLIT_RESPONSE}, GO_COMPLEX_RAW_CGI=${GO_COMPLEX_RAW_CGI})"
    return
  fi

  if [[ -n "$(dc ps -q bench 2>/dev/null)" ]]; then
    echo ">> container up but probe failed — recreating" >&2
    start_stack
    return
  fi

  if docker ps -q --filter "publish=${BENCH_PORT}" 2>/dev/null | grep -q .; then
    echo ">> port ${BENCH_PORT} already allocated — stopping other containers" >&2
    start_stack
    return
  fi

  start_stack
}

# label|url
TARGETS=(
  "marty /primes|${BASE}/cgi-bin/marty-complex/primes/${PRIME_LIMIT}"
  "marty /fibonacci|${BASE}/cgi-bin/marty-complex/fibonacci/${FIB_N}"
  "marty /matrix|${BASE}/cgi-bin/marty-complex/matrix/${MATRIX_SIZE}"
  "go /primes|${BASE}/cgi-bin/go-complex/primes/${PRIME_LIMIT}"
  "go /fibonacci|${BASE}/cgi-bin/go-complex/fibonacci/${FIB_N}"
  "go /matrix|${BASE}/cgi-bin/go-complex/matrix/${MATRIX_SIZE}"
  "python /primes|${BASE}/cgi-bin/python-complex/primes/${PRIME_LIMIT}"
  "python /fibonacci|${BASE}/cgi-bin/python-complex/fibonacci/${FIB_N}"
  "python /matrix|${BASE}/cgi-bin/python-complex/matrix/${MATRIX_SIZE}"
  "php /primes|${BASE}/php/complex.php/primes/${PRIME_LIMIT}"
  "php /fibonacci|${BASE}/php/complex.php/fibonacci/${FIB_N}"
  "php /matrix|${BASE}/php/complex.php/matrix/${MATRIX_SIZE}"
)

target_enabled() {
  local label="$1"
  local lang="${label%% *}"
  local wanted
  for wanted in $BENCH_LANGS; do
    if [[ "$lang" == "$wanted" ]]; then
      return 0
    fi
  done
  return 1
}

run_hey() {
  local url="$1"
  local requests="${2:-$N}"
  hey -n "$requests" -c "$C" -m GET "$url" 2>&1
}

parse_metric() {
  local haystack="$1"
  local key="$2"
  local line
  line="$(echo "$haystack" | grep -F "${key}:" | head -1 || true)"
  [[ -z "$line" ]] && return 0
  echo "$line" | sed -E 's/.*:[[:space:]]*([0-9.eE+-]+).*/\1/'
}

# hey "Latency distribution:" lines: "  50%% in 0.0420 secs" (template prints %%).
# Missing percentiles (e.g. 95/99 on short runs) must not fail under pipefail.
parse_latency_pct() {
  local haystack="$1"
  local pct="$2"
  local line
  line="$(echo "$haystack" | grep -E "^[[:space:]]*${pct}%+" | grep -F " in " | head -1 || true)"
  [[ -z "$line" ]] && return 0
  echo "$line" | sed -E 's/.*in[[:space:]]+([0-9.eE+-]+)[[:space:]]+secs.*/\1/'
}

secs_to_ms() {
  awk -v s="$1" 'BEGIN { if (s == "" || s == "N/A") print "n/a"; else printf "%.2f", s * 1000 }'
}

# Median of numeric args (for BENCH_RUNS > 1); prints n/a if empty. Portable (no gawk asort).
median_ms() {
  local -a nums=()
  local v sorted n mid
  for v in "$@"; do
    [[ "$v" == "n/a" || -z "$v" ]] && continue
    nums+=("$v")
  done
  n=${#nums[@]}
  if [[ "$n" -eq 0 ]]; then
    echo "n/a"
    return 0
  fi
  if [[ "$n" -eq 1 ]]; then
    printf "%.2f" "${nums[0]}"
    return 0
  fi
  sorted="$(printf '%s\n' "${nums[@]}" | sort -n)"
  if [[ $((n % 2)) -eq 1 ]]; then
    mid=$(( (n + 1) / 2 ))
    printf "%.2f" "$(echo "$sorted" | sed -n "${mid}p")"
  else
    local lo hi
    lo="$(echo "$sorted" | sed -n "$(( n / 2 ))p")"
    hi="$(echo "$sorted" | sed -n "$(( n / 2 + 1 ))p")"
    awk -v a="$lo" -v b="$hi" 'BEGIN { printf "%.2f", (a + b) / 2 }'
  fi
}

bench_container_id() {
  dc ps -q bench 2>/dev/null | head -1
}

bench_timing_clear() {
  local label="$1"
  local cid
  cid="$(bench_container_id)"
  [[ -z "$cid" ]] && return 0
  if [[ "$BENCH_TIMING" != "1" ]]; then
    return 0
  fi
  docker exec -e LABEL="$label" "$cid" sh -c 'printf "# %s\n" "$LABEL" > /var/log/bench-timing/requests.log' \
    2>/dev/null || true
}

bench_timing_avg_ms() {
  local expect_impl="${1:-}"
  local expect_route="${2:-}"
  local cid
  cid="$(bench_container_id)"
  if [[ -z "$cid" ]] || [[ "$BENCH_TIMING" != "1" ]]; then
    echo "n/a"
    return
  fi
  docker exec "$cid" awk -v impl="$expect_impl" -v route="$expect_route" '
    $1 ~ /^#/ { next }
    NF >= 3 && $3+0 > 0 {
      if (impl != "" && $1 != impl) next
      if (route != "" && $2 != route) next
      s += $3+0
      n++
    }
    END {
      if (n > 0) printf "%.2f", s/n/1000
      else print "n/a"
    }
  ' /var/log/bench-timing/requests.log 2>/dev/null || echo "n/a"
}

bench_timing_count() {
  local expect_impl="${1:-}"
  local expect_route="${2:-}"
  local cid
  cid="$(bench_container_id)"
  [[ -z "$cid" ]] || [[ "$BENCH_TIMING" != "1" ]] && { echo "0"; return; }
  docker exec "$cid" awk -v impl="$expect_impl" -v route="$expect_route" '
    $1 ~ /^#/ { next }
    NF >= 3 && $3+0 > 0 {
      if (impl != "" && $1 != impl) next
      if (route != "" && $2 != route) next
      n++
    }
    END { print n+0 }
  ' /var/log/bench-timing/requests.log 2>/dev/null || echo "0"
}

bench_timing_key() {
  local label="$1"
  local lang route
  lang="${label%% *}"
  route="${label#* }"
  route="${route#/}"
  case "$lang" in
    marty) echo "marty-complex ${route}" ;;
    go) echo "go-complex ${route}" ;;
    python) echo "python-complex ${route}" ;;
    php) echo "php-complex ${route}" ;;
    *) echo " ${route}" ;;
  esac
}

run_one_target() {
  local label="$1"
  local url="$2"
  local -a run_avgs=()
  local -a run_p50s=()
  local -a run_p95s=()
  local -a run_p99s=()
  local -a run_fast=()
  local -a run_slow=()
  local -a run_rps=()
  local -a run_startup=()
  local run r output avg_s p50_s p95_s p99_s fast_s slow_s rps startup_ms timing_n

  read -r timing_impl timing_route <<< "$(bench_timing_key "$label")"

  if [[ "$WARMUP_N" -gt 0 ]]; then
    echo ">> $label (warm-up ${WARMUP_N} requests) …"
    run_hey "$url" "$WARMUP_N" >/dev/null || true
    sleep "$PAUSE"
  fi

  for run in $(seq 1 "$RUNS"); do
    if [[ "$RUNS" -gt 1 ]]; then
      echo ">> $label (run ${run}/${RUNS}) …"
    else
      echo ">> $label …"
    fi
    bench_timing_clear "$label"
    output="$(run_hey "$url")"
    if ! echo "$output" | grep -q "Requests/sec"; then
      echo ">> error: hey failed for $label (run ${run}/${RUNS})" >&2
      echo "$output" >&2
      return 1
    fi
    startup_ms="$(bench_timing_avg_ms "$timing_impl" "$timing_route")"
    timing_n="$(bench_timing_count "$timing_impl" "$timing_route")"
    if [[ "$BENCH_TIMING" == "1" ]] && [[ "$timing_n" -lt $((N / 2)) ]]; then
      echo ">> warning: only ${timing_n}/${N} timing lines in log (rebuild image? BENCH_REBUILD=1)" >&2
    fi
    avg_s="$(parse_metric "$output" "Average")"
    p50_s="$(parse_latency_pct "$output" 50)"
    p95_s="$(parse_latency_pct "$output" 95)"
    p99_s="$(parse_latency_pct "$output" 99)"
    fast_s="$(parse_metric "$output" "Fastest")"
    slow_s="$(parse_metric "$output" "Slowest")"
    rps="$(parse_metric "$output" "Requests/sec")"

    run_avgs+=("$(secs_to_ms "$avg_s")")
    run_p50s+=("$(secs_to_ms "$p50_s")")
    run_p95s+=("$(secs_to_ms "$p95_s")")
    run_p99s+=("$(secs_to_ms "$p99_s")")
    run_fast+=("$(secs_to_ms "$fast_s")")
    run_slow+=("$(secs_to_ms "$slow_s")")
    run_rps+=("${rps:-n/a}")
    run_startup+=("$startup_ms")

    if [[ "${BENCH_VERBOSE:-0}" == "1" ]]; then
      echo "$output"
      echo
    fi

    if [[ "$run" -lt "$RUNS" ]]; then
      sleep "$PAUSE"
    fi
  done

  LABELS+=("$label")
  AVGS_MS+=("$(median_ms "${run_avgs[@]}")")
  P50_MS+=("$(median_ms "${run_p50s[@]}")")
  P95_MS+=("$(median_ms "${run_p95s[@]}")")
  P99_MS+=("$(median_ms "${run_p99s[@]}")")
  STARTUP_MS+=("$(median_ms "${run_startup[@]}")")
  FAST_MS+=("$(median_ms "${run_fast[@]}")")
  SLOW_MS+=("$(median_ms "${run_slow[@]}")")
  RPS+=("$(median_ms "${run_rps[@]}")")
}

ensure_apache
echo ">> waiting for handlers…"
for pair in "${TARGETS[@]}"; do
  target_enabled "${pair%%|*}" || continue
  wait_ready "${pair#*|}"
done

timing_mode="off (throughput)"
if [[ "$BENCH_TIMING" == "1" ]]; then
  timing_mode="on (startup log)"
fi

echo
echo "Fair complex benchmark (CPU routes)"
if [[ "$BENCH_DOCKERFILE" == *hosting* ]]; then
  echo "  Server:     Apache in Docker, hosting-like (mpm_event + mod_cgid + suexec + php-fpm)"
else
  echo "  Server:     Apache in Docker (prefork + mod_cgi + mod_php), port default 8081"
fi
echo "  Dockerfile: ${BENCH_DOCKERFILE} (project ${COMPOSE_PROJECT_NAME})"
echo "  Workloads:  primes/${PRIME_LIMIT} (sieve), fibonacci/${FIB_N}, matrix/${MATRIX_SIZE}"
echo "  Handlers:   ${BENCH_LANGS}"
echo "  Requests:   $N per (handler × route), concurrency $C (hey)"
if [[ "$WARMUP_N" -gt 0 ]]; then
  echo "  Warm-up:    $WARMUP_N requests per target before measured runs"
fi
echo "  Runs:       $RUNS per target (median when > 1)"
echo "  Between:    ${PAUSE}s pause, no container restart"
echo "  BENCH_TIMING: ${timing_mode}"
echo "  MARTY_COMPLEX_SPLIT_RESPONSE: ${MARTY_COMPLEX_SPLIT_RESPONSE}"
echo "  GO_COMPLEX_RAW_CGI: ${GO_COMPLEX_RAW_CGI}"
echo

declare -a LABELS=()
declare -a AVGS_MS=()
declare -a P50_MS=()
declare -a P95_MS=()
declare -a P99_MS=()
declare -a STARTUP_MS=()
declare -a FAST_MS=()
declare -a SLOW_MS=()
declare -a RPS=()

for pair in "${TARGETS[@]}"; do
  target_enabled "${pair%%|*}" || continue
  run_one_target "${pair%%|*}" "${pair#*|}"
  sleep "$PAUSE"
done

echo
echo "════════════════════════════════════════════════════════════════════════════════════"
printf "%-18s %8s %8s %8s %8s %10s %8s %8s %8s\n" \
  "Handler" "Avg" "P50" "P95" "P99" "Startup" "Fast" "Slow" "req/s"
echo "────────────────────────────────────────────────────────────────────────────────────"
for i in "${!LABELS[@]}"; do
  printf "%-18s %7s %7s %7s %7s %10s %7s %7s %7s\n" \
    "${LABELS[$i]}" "${AVGS_MS[$i]}" "${P50_MS[$i]}" "${P95_MS[$i]}" "${P99_MS[$i]}" \
    "${STARTUP_MS[$i]}" "${FAST_MS[$i]}" "${SLOW_MS[$i]}" "${RPS[$i]}"
done
echo "════════════════════════════════════════════════════════════════════════════════════"
echo
echo "All latency columns in ms. Avg/P50/P95/P99/Fast/Slow/req/s from hey."
if [[ "$BENCH_TIMING" == "1" ]]; then
  echo "Startup = mean in-process µs→ms before compute (container log; excludes Apache fork)."
else
  echo "Startup n/a — set BENCH_TIMING=1 for in-process startup column (adds per-request log I/O)."
fi
echo "Throughput mode: BENCH_TIMING=0 just bench-fair  |  Rebuild: BENCH_REBUILD=1 just bench-fair-rebuild"
