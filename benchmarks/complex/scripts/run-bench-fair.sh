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
  stop_docker_on_port "$port"
  dc down --remove-orphans 2>/dev/null || true
  echo ">> docker compose up -d (project ${COMPOSE_PROJECT_NAME}, host :${port})"
  dc up -d
}

ensure_apache() {
  cd "$HERE"
  export BENCH_PORT="$(bench_host_port "$BASE")"

  if [[ "${BENCH_REBUILD:-0}" == "1" ]]; then
    echo ">> docker compose build (BENCH_REBUILD=1)"
    dc build
    echo ">> recreating container so new image is used"
    start_stack
    return
  fi

  if [[ -n "$(dc ps -q bench 2>/dev/null)" ]] && curl -sf -o /dev/null -m 5 "$PROBE_URL"; then
    echo ">> reusing running complex bench container on ${BASE} (probe ok)"
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

run_hey() {
  local url="$1"
  hey -n "$N" -c "$C" -m GET "$url" 2>&1
}

parse_metric() {
  local haystack="$1"
  local key="$2"
  echo "$haystack" | grep -F "${key}:" | head -1 | sed -E 's/.*:[[:space:]]*([0-9.eE+-]+).*/\1/'
}

secs_to_ms() {
  awk -v s="$1" 'BEGIN { if (s == "" || s == "N/A") print "n/a"; else printf "%.2f", s * 1000 }'
}

bench_container_id() {
  dc ps -q bench 2>/dev/null | head -1
}

bench_timing_clear() {
  local label="$1"
  local cid
  cid="$(bench_container_id)"
  [[ -z "$cid" ]] && return 0
  # Header line for humans; data lines: impl<TAB>route<TAB>pre_compute_us
  docker exec -e LABEL="$label" "$cid" sh -c 'printf "# %s\n" "$LABEL" > /var/log/bench-timing/requests.log' \
    2>/dev/null || true
}

bench_timing_avg_ms() {
  local expect_impl="${1:-}"
  local expect_route="${2:-}"
  local cid
  cid="$(bench_container_id)"
  if [[ -z "$cid" ]]; then
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
  [[ -z "$cid" ]] && { echo "0"; return; }
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

# marty /primes -> marty-complex primes
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

ensure_apache
echo ">> waiting for handlers…"
for pair in "${TARGETS[@]}"; do
  wait_ready "${pair#*|}"
done

echo
echo "Fair complex benchmark (CPU routes)"
echo "  Server:     Apache in Docker (CGI + mod_php), port default 8081"
echo "  Workloads:  primes/${PRIME_LIMIT} (sieve), fibonacci/${FIB_N}, matrix/${MATRIX_SIZE}"
echo "  Requests:   $N per (handler × route), concurrency $C (hey)"
echo "  Between:    ${PAUSE}s pause, no container restart"
echo "  Startup:    server log per request (BENCH_TIMING), avg after each hey run"
echo

declare -a LABELS=()
declare -a AVGS_MS=()
declare -a STARTUP_MS=()
declare -a FAST_MS=()
declare -a SLOW_MS=()
declare -a RPS=()

for pair in "${TARGETS[@]}"; do
  label="${pair%%|*}"
  url="${pair#*|}"
  read -r timing_impl timing_route <<< "$(bench_timing_key "$label")"
  bench_timing_clear "$label"
  echo ">> $label …"
  output="$(run_hey "$url")"
  startup_ms="$(bench_timing_avg_ms "$timing_impl" "$timing_route")"
  timing_n="$(bench_timing_count "$timing_impl" "$timing_route")"
  if [[ "$timing_n" -lt $((N / 2)) ]]; then
    echo ">> warning: only ${timing_n}/${N} timing lines in log (rebuild image? BENCH_REBUILD=1)" >&2
  fi
  avg_s="$(parse_metric "$output" "Average")"
  fast_s="$(parse_metric "$output" "Fastest")"
  slow_s="$(parse_metric "$output" "Slowest")"
  rps="$(parse_metric "$output" "Requests/sec")"

  LABELS+=("$label")
  AVGS_MS+=("$(secs_to_ms "$avg_s")")
  STARTUP_MS+=("$startup_ms")
  FAST_MS+=("$(secs_to_ms "$fast_s")")
  SLOW_MS+=("$(secs_to_ms "$slow_s")")
  RPS+=("${rps:-n/a}")

  if [[ "${BENCH_VERBOSE:-0}" == "1" ]]; then
    echo "$output"
    echo
  fi

  sleep "$PAUSE"
done

echo
echo "════════════════════════════════════════════════════════════════════════"
printf "%-18s %10s %14s %10s %10s %10s\n" "Handler" "Avg (ms)" "Startup avg" "Fast (ms)" "Slow (ms)" "req/s"
echo "──────────────────────────────────────────────────────────────────────────────────"
for i in "${!LABELS[@]}"; do
  printf "%-18s %10s %14s %10s %10s %10s\n" \
    "${LABELS[$i]}" "${AVGS_MS[$i]}" "${STARTUP_MS[$i]}" "${FAST_MS[$i]}" "${SLOW_MS[$i]}" "${RPS[$i]}"
done
echo "══════════════════════════════════════════════════════════════════════════════════"
echo
echo "Avg/req/s = hey. Startup avg = mean in-process µs→ms before compute (log in container; excludes Apache fork)."
echo "Lower is faster. Rebuild: BENCH_REBUILD=1 just bench-fair-rebuild"
