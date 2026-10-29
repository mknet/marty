#!/usr/bin/env bash
# Low-volume fair compare: complex CPU routes on cloud alternatives + Marty hosting.
# Same workloads as benchmarks/complex `just outside` (primes/400k, fib/42, matrix/128).
#
# Usage:
#   cp targets.example.env targets.env   # fill URLs after deploy
#   ./compare-outside.sh                 # or: just outside
#
# Env (optional):
#   TARGETS_FILE=targets.env
#   BENCH_REQUESTS=40 BENCH_CONCURRENCY=2 BENCH_PAUSE_SEC=2
#   BENCH_PRIME_LIMIT / BENCH_FIB_N / BENCH_MATRIX_SIZE
#   BENCH_TARGETS="aws azure cf scw hetzner allinkl"   # subset
#   BENCH_PROBE_TIMEOUT=60
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGETS_FILE="${TARGETS_FILE:-$ROOT/targets.env}"

N="${BENCH_REQUESTS:-40}"
C="${BENCH_CONCURRENCY:-2}"
PAUSE="${BENCH_PAUSE_SEC:-2}"
PRIME_LIMIT="${BENCH_PRIME_LIMIT:-400000}"
FIB_N="${BENCH_FIB_N:-42}"
MATRIX_SIZE="${BENCH_MATRIX_SIZE:-128}"
PROBE_TIMEOUT="${BENCH_PROBE_TIMEOUT:-60}"
WANT_TARGETS="${BENCH_TARGETS:-aws azure cf scw hetzner allinkl}"

if [[ ! -f "$TARGETS_FILE" ]]; then
  echo "error: missing $TARGETS_FILE — copy targets.example.env and fill URLs" >&2
  exit 1
fi

# shellcheck disable=SC1090
set -a
# shellcheck source=/dev/null
source "$TARGETS_FILE"
set +a

if ! command -v hey >/dev/null 2>&1; then
  echo "error: hey not on PATH (go install github.com/rakyll/hey@latest)" >&2
  exit 1
fi

want() {
  local t="$1" x
  for x in $WANT_TARGETS; do
    [[ "$x" == "$t" ]] && return 0
  done
  return 1
}

# label|base_url — base has no trailing slash; routes appended as /primes/…
declare -a BASES=()
add_base() {
  local id="$1" url="${2:-}"
  want "$id" || return 0
  if [[ -z "$url" ]]; then
    echo "skip $id (empty URL in $TARGETS_FILE)" >&2
    return 0
  fi
  url="${url%/}"
  BASES+=("${id}|${url}")
}

add_base aws "${AWS_COMPLEX_URL:-}"
add_base azure "${AZURE_COMPLEX_URL:-}"
add_base cf "${CF_COMPLEX_URL:-}"
add_base scw "${SCW_COMPLEX_URL:-}"
add_base hetzner "${HETZNER_SYNC_URL:-}"
add_base allinkl "${ALLINKL_SYNC_URL:-}"

if [[ ${#BASES[@]} -eq 0 ]]; then
  echo "error: no targets with URLs set" >&2
  exit 1
fi

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

cold_probe() {
  local label="$1" url="$2"
  local code t_total body out
  body="$(mktemp)"
  # One timed GET (best-effort cold after the pause between targets).
  out="$(curl -sS -o "$body" -w '%{http_code} %{time_total}' -m "$PROBE_TIMEOUT" "$url" || echo "fail 0")"
  code="${out%% *}"
  t_total="${out#* }"
  if [[ "$code" != "200" ]]; then
    echo "  cold $label → HTTP $code (see body)" >&2
    head -c 160 "$body" 2>/dev/null | tr '\n' ' ' >&2 || true
    echo >&2
    rm -f "$body"
    echo "n/a"
    return 1
  fi
  if [[ "$url" == *"/matrix/"* ]]; then
    if ! grep -q "\"size\":${MATRIX_SIZE}" "$body" 2>/dev/null; then
      echo "  cold $label: expected JSON size=$MATRIX_SIZE" >&2
      head -c 160 "$body" >&2 || true
      echo >&2
      rm -f "$body"
      echo "n/a"
      return 1
    fi
  fi
  rm -f "$body"
  secs_to_ms "$t_total"
}

run_hey() {
  local label="$1" url="$2"
  local output avg_s p50_s p95_s p99_s rps ok
  echo ">> $label …" >&2
  output="$(hey -n "$N" -c "$C" -m GET -t "$PROBE_TIMEOUT" "$url" 2>&1)" || true
  if ! echo "$output" | grep -q "Requests/sec"; then
    echo ">> error: hey failed for $label" >&2
    echo "$output" >&2
    printf "%-22s %8s %8s %8s %8s %8s %6s %8s\n" \
      "$label" "n/a" "n/a" "n/a" "n/a" "n/a" "0" "n/a"
    return 1
  fi
  avg_s="$(parse_metric "$output" "Average")"
  p50_s="$(parse_latency_pct "$output" 50)"
  p95_s="$(parse_latency_pct "$output" 95)"
  p99_s="$(parse_latency_pct "$output" 99)"
  rps="$(parse_metric "$output" "Requests/sec")"
  ok="$(echo "$output" | grep -E '^\s*\[200\]' | awk '{print $2}' || echo "?")"
  printf "%-22s %8s %8s %8s %8s %8s %6s %8s\n" \
    "$label" \
    "$(secs_to_ms "$avg_s")" \
    "$(secs_to_ms "$p50_s")" \
    "$(secs_to_ms "$p95_s")" \
    "$(secs_to_ms "$p99_s")" \
    "${rps:-n/a}" \
    "${ok:-?}" \
    "${COLD_MS:-n/a}"
}

echo "Alternatives complex compare (low volume; fair protocol)"
echo "  Targets file: $TARGETS_FILE"
echo "  Targets:      $WANT_TARGETS"
echo "  Per route:    $N requests, concurrency $C"
echo "  Workloads:    primes/$PRIME_LIMIT, fibonacci/$FIB_N, matrix/$MATRIX_SIZE (?salt=0)"
echo "  Between:      ${PAUSE}s pause"
echo "  Cold:         one timed GET before each warm hey (best-effort)"
echo

declare -a ROUTE_SPECS=(
  "primes|/primes/${PRIME_LIMIT}?salt=0"
  "fibonacci|/fibonacci/${FIB_N}?salt=0"
  "matrix|/matrix/${MATRIX_SIZE}?salt=0"
)

printf "%-22s %8s %8s %8s %8s %8s %6s %8s\n" \
  "Handler" "Avg" "P50" "P95" "P99" "req/s" "200s" "cold"
echo "----------------------------------------------------------------------------------------"

first=1
for base_pair in "${BASES[@]}"; do
  id="${base_pair%%|*}"
  base="${base_pair#*|}"
  for spec in "${ROUTE_SPECS[@]}"; do
    route="${spec%%|*}"
    suffix="${spec#*|}"
    url="${base}${suffix}"
    label="${id} /${route}"
    if [[ "$first" -eq 0 ]]; then
      sleep "$PAUSE"
    fi
    first=0
    echo ">> cold probe $label …" >&2
    COLD_MS="$(cold_probe "$label" "$url" || true)"
    export COLD_MS
    # Brief pause so warm hey is not the same connection storm as cold.
    sleep 1
    run_hey "$label" "$url" || true
  done
done

echo
echo "Latency columns in ms. Primary columns = warm hey (N=$N C=$C)."
echo "cold = single GET before that route's hey (includes platform cold-start if idle)."
echo "Cloudflare matrix may be n/a on free CPU limits — do not shrink the workload."
echo "Regions (deploy defaults): AWS eu-west-1, Azure northeurope, Scaleway fr-par, CF edge."
