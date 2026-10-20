#!/usr/bin/env bash
# Low-volume sync / Axum / PHP check on shared hosting (e.g. all-inkl).
# Go is omitted: stock Go CGI dies with heap-arena OOM on typical shared hosts.
#
# Modes (BENCH_MODE):
#   normal  — primes/400k, fib/42, matrix/128; N=40 C=2
#   extreme — primes/1M, matrix/400 only; N=8 C=1 (~heavy O(n³) matrix)
set -euo pipefail

usage() {
  cat <<'EOF' >&2
Usage: compare-outside.sh <base-url>

Env (optional):
  BENCH_MODE=normal|extreme
  BENCH_REQUESTS / BENCH_CONCURRENCY / BENCH_PAUSE_SEC
  BENCH_LANGS="sync axum php"
  PHP_SCRIPT=/php/complex.php
  BENCH_PRIME_LIMIT / BENCH_FIB_N / BENCH_MATRIX_SIZE
  BENCH_PROBE_TIMEOUT=60   (seconds; use higher for extreme)
EOF
  exit 1
}

[[ $# -eq 1 ]] || usage
BASE="${1%/}"

MODE="${BENCH_MODE:-normal}"
LANGS="${BENCH_LANGS:-sync axum php}"
PHP_SCRIPT="${PHP_SCRIPT:-/php/complex.php}"

case "$MODE" in
  normal)
    N="${BENCH_REQUESTS:-40}"
    C="${BENCH_CONCURRENCY:-2}"
    PAUSE="${BENCH_PAUSE_SEC:-2}"
    PRIME_LIMIT="${BENCH_PRIME_LIMIT:-400000}"
    FIB_N="${BENCH_FIB_N:-42}"
    MATRIX_SIZE="${BENCH_MATRIX_SIZE:-128}"
    ROUTES="${BENCH_ROUTES:-primes fibonacci matrix}"
    PROBE_TIMEOUT="${BENCH_PROBE_TIMEOUT:-60}"
    ;;
  extreme)
    # ~8× matrix work vs n=200; keep concurrency 1 and few repeats on shared hosts.
    N="${BENCH_REQUESTS:-8}"
    C="${BENCH_CONCURRENCY:-1}"
    PAUSE="${BENCH_PAUSE_SEC:-3}"
    PRIME_LIMIT="${BENCH_PRIME_LIMIT:-1000000}"
    FIB_N="${BENCH_FIB_N:-42}"
    MATRIX_SIZE="${BENCH_MATRIX_SIZE:-400}"
    ROUTES="${BENCH_ROUTES:-primes matrix}"
    PROBE_TIMEOUT="${BENCH_PROBE_TIMEOUT:-180}"
    ;;
  *)
    echo "error: BENCH_MODE must be normal or extreme" >&2
    exit 1
    ;;
esac

if ! command -v hey >/dev/null 2>&1; then
  echo "error: hey not on PATH (go install github.com/rakyll/hey@latest)" >&2
  exit 1
fi

lang_enabled() {
  local want="$1" l
  for l in $LANGS; do
    [[ "$l" == "$want" ]] && return 0
  done
  return 1
}

route_enabled() {
  local want="$1" r
  for r in $ROUTES; do
    [[ "$r" == "$want" ]] && return 0
  done
  return 1
}

# Map language token → cgi-bin binary name (or empty for php).
cgi_prefix_for() {
  case "$1" in
    sync) echo "${BASE}/cgi-bin/marty-sync" ;;
    axum) echo "${BASE}/cgi-bin/marty-axum" ;;
    php)  echo "${BASE}${PHP_SCRIPT}" ;;
    *) return 1 ;;
  esac
}

n_handlers=0
for l in sync axum php; do
  lang_enabled "$l" && n_handlers=$((n_handlers + 1))
done
n_routes=0
route_enabled primes && n_routes=$((n_routes + 1))
route_enabled fibonacci && n_routes=$((n_routes + 1))
route_enabled matrix && n_routes=$((n_routes + 1))
TOTAL=$((N * n_handlers * n_routes))

echo "Hosting compare (low volume; Go skipped)"
echo "  Mode:         $MODE"
echo "  Base:         $BASE"
echo "  Handlers:     $LANGS"
echo "  PHP script:   ${PHP_SCRIPT}"
echo "  Per target:   $N requests, concurrency $C"
echo "  Routes:       $ROUTES"
echo "  Workloads:    primes/$PRIME_LIMIT, fibonacci/$FIB_N, matrix/$MATRIX_SIZE (?salt=0)"
echo "  Total GETs:   ~$TOTAL (+ probes)"
echo "  Between:      ${PAUSE}s pause"
echo

probe() {
  local url="$1"
  local code
  code="$(curl -sS -o /tmp/complex-outside-probe.body -w '%{http_code}' -m "$PROBE_TIMEOUT" "$url" || echo fail)"
  if [[ "$code" != "200" ]]; then
    echo "error: probe $url → HTTP $code" >&2
    head -c 200 /tmp/complex-outside-probe.body 2>/dev/null | tr '\n' ' ' >&2 || true
    echo >&2
    exit 1
  fi
  # Confirm matrix size was not clamped (old binaries capped at 200).
  if [[ "$url" == *"/matrix/"* ]]; then
    if ! grep -q "\"size\":${MATRIX_SIZE}" /tmp/complex-outside-probe.body 2>/dev/null; then
      echo "error: expected JSON size=$MATRIX_SIZE (re-upload hosting binaries / complex.php?)" >&2
      head -c 200 /tmp/complex-outside-probe.body >&2 || true
      echo >&2
      exit 1
    fi
  fi
}

parse_metric() {
  local haystack="$1" key="$2" line
  line="$(echo "$haystack" | grep -F "${key}:" | head -1 || true)"
  [[ -z "$line" ]] && return 0
  echo "$line" | sed -E 's/.*:[[:space:]]*([0-9.eE+-]+).*/\1/'
}

parse_latency_pct() {
  local haystack="$1" pct="$2" line
  line="$(echo "$haystack" | grep -E "^[[:space:]]*${pct}%+" | grep -F " in " | head -1 || true)"
  [[ -z "$line" ]] && return 0
  echo "$line" | sed -E 's/.*in[[:space:]]+([0-9.eE+-]+)[[:space:]]+secs.*/\1/'
}

secs_to_ms() {
  awk -v s="$1" 'BEGIN { if (s == "" || s == "N/A") print "n/a"; else printf "%.1f", s * 1000 }'
}

run_one() {
  local label="$1" url="$2"
  local output avg_s p50_s p95_s p99_s rps ok
  echo ">> $label …" >&2
  # hey has no per-request timeout; extreme PHP can take many seconds each.
  output="$(hey -n "$N" -c "$C" -m GET -t "$PROBE_TIMEOUT" "$url" 2>&1)"
  if ! echo "$output" | grep -q "Requests/sec"; then
    echo ">> error: hey failed for $label" >&2
    echo "$output" >&2
    return 1
  fi
  avg_s="$(parse_metric "$output" "Average")"
  p50_s="$(parse_latency_pct "$output" 50)"
  p95_s="$(parse_latency_pct "$output" 95)"
  p99_s="$(parse_latency_pct "$output" 99)"
  rps="$(parse_metric "$output" "Requests/sec")"
  ok="$(echo "$output" | grep -E '^\s*\[200\]' | awk '{print $2}' || echo "?")"
  printf "%-18s %8s %8s %8s %8s %8s %6s\n" \
    "$label" \
    "$(secs_to_ms "$avg_s")" \
    "$(secs_to_ms "$p50_s")" \
    "$(secs_to_ms "$p95_s")" \
    "$(secs_to_ms "$p99_s")" \
    "${rps:-n/a}" \
    "${ok:-?}"
}

add_targets() {
  local lang="$1" prefix="$2"
  if route_enabled primes; then
    TARGETS+=("${lang} /primes|${prefix}/primes/${PRIME_LIMIT}?salt=0")
  fi
  if route_enabled fibonacci; then
    TARGETS+=("${lang} /fibonacci|${prefix}/fibonacci/${FIB_N}?salt=0")
  fi
  if route_enabled matrix; then
    TARGETS+=("${lang} /matrix|${prefix}/matrix/${MATRIX_SIZE}?salt=0")
  fi
}

TARGETS=()
for l in sync axum php; do
  if lang_enabled "$l"; then
    add_targets "$l" "$(cgi_prefix_for "$l")"
  fi
done

echo ">> probing …"
for l in sync axum php; do
  if ! lang_enabled "$l"; then
    continue
  fi
  prefix="$(cgi_prefix_for "$l")"
  if route_enabled matrix; then
    probe "${prefix}/matrix/${MATRIX_SIZE}?salt=0"
  else
    probe "${prefix}/primes/10000?salt=0"
  fi
done
echo

printf "%-18s %8s %8s %8s %8s %8s %6s\n" \
  "Handler" "Avg" "P50" "P95" "P99" "req/s" "200s"
echo "--------------------------------------------------------------------------------"

first=1
for pair in "${TARGETS[@]}"; do
  if [[ "$first" -eq 0 ]]; then
    sleep "$PAUSE"
  fi
  first=0
  run_one "${pair%%|*}" "${pair#*|}"
done

echo
echo "Latency columns in ms. Mode=$MODE. Go not compared (arena OOM on shared hosting)."
