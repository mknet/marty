#!/usr/bin/env bash
# Fair webhook compare: one Apache (mod_php + CGI), same load per handler, summary table.
# Does NOT restart Docker between handlers — only `docker compose up -d` once per invocation.
# Rebuild image only when BENCH_REBUILD=1 (code changed).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$HERE/docker-compose.yml"
COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-marty-bench-simple}"
BASE="${BENCH_BASE_URL:-http://127.0.0.1:8080}"
PROBE_URL="${BASE}/cgi-bin/marty-webhook"
PAYLOAD="$HERE/fixtures/webhook-post.json"
N="${BENCH_REQUESTS:-2000}"
C="${BENCH_CONCURRENCY:-10}"
PAUSE="${BENCH_PAUSE_SEC:-1}"

# shellcheck source=ensure-hey.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/ensure-hey.sh"
ensure_hey

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker not on PATH" >&2
  exit 1
fi

wait_ready() {
  local url="$1"
  local i
  for i in $(seq 1 60); do
    if curl -sf -o /dev/null -m 2 -X POST \
      -H "Content-Type: application/json" \
      -H "X-Webhook-Secret: bench-secret" \
      -d @"$PAYLOAD" \
      "$url"; then
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

ensure_apache() {
  cd "$HERE"
  if [[ "${BENCH_REBUILD:-0}" == "1" ]]; then
    echo ">> docker compose build (BENCH_REBUILD=1)"
    dc build
  fi
  if [[ -n "$(dc ps -q bench 2>/dev/null)" ]] && curl -sf -o /dev/null -m 5 -X POST \
    -H "Content-Type: application/json" \
    -H "X-Webhook-Secret: bench-secret" \
    -d '{"id":"probe","type":"probe"}' \
    "$PROBE_URL"; then
    echo ">> reusing running simple bench container on ${BASE} (probe ok)"
  else
    if [[ -n "$(dc ps -q bench 2>/dev/null)" ]]; then
      echo ">> container up but probe failed — recreating" >&2
      dc up -d --force-recreate
    else
      echo ">> docker compose up -d (project ${COMPOSE_PROJECT_NAME})"
      dc up -d
    fi
  fi
}

# label|url
TARGETS=(
  "marty (Rust CGI)|${BASE}/cgi-bin/marty-webhook"
  "go (CGI)|${BASE}/cgi-bin/go-webhook"
  "python (stdlib CGI)|${BASE}/cgi-bin/python-webhook"
  "php (mod_php)|${BASE}/php/webhook.php"
)

run_hey() {
  local url="$1"
  hey -n "$N" -c "$C" -m POST \
    -H "Content-Type: application/json" \
    -H "X-Webhook-Secret: bench-secret" \
    -D "$PAYLOAD" \
    "$url" 2>&1
}

# hey summary lines look like: "  Average:	0.0117 secs"
parse_metric() {
  local haystack="$1"
  local key="$2"
  echo "$haystack" | grep -F "${key}:" | head -1 | sed -E 's/.*:[[:space:]]*([0-9.eE+-]+).*/\1/'
}

secs_to_ms() {
  awk -v s="$1" 'BEGIN { if (s == "" || s == "N/A") print "n/a"; else printf "%.2f", s * 1000 }'
}

ensure_apache
echo ">> waiting for handlers…"
for pair in "${TARGETS[@]}"; do
  wait_ready "${pair#*|}"
done

echo
echo "Fair webhook benchmark"
echo "  Server:    Apache in Docker (CGI + mod_php), single container for all runs"
echo "  Requests:  $N per handler (concurrency $C)"
echo "  Between:   ${PAUSE}s pause, no container restart between handlers"
echo

declare -a LABELS=()
declare -a AVGS_MS=()
declare -a FAST_MS=()
declare -a SLOW_MS=()
declare -a RPS=()

for pair in "${TARGETS[@]}"; do
  label="${pair%%|*}"
  url="${pair#*|}"
  echo ">> $label …"
  output="$(run_hey "$url")"
  avg_s="$(parse_metric "$output" "Average")"
  fast_s="$(parse_metric "$output" "Fastest")"
  slow_s="$(parse_metric "$output" "Slowest")"
  rps="$(parse_metric "$output" "Requests/sec")"

  avg_ms="$(secs_to_ms "$avg_s")"
  fast_ms="$(secs_to_ms "$fast_s")"
  slow_ms="$(secs_to_ms "$slow_s")"

  LABELS+=("$label")
  AVGS_MS+=("$avg_ms")
  FAST_MS+=("$fast_ms")
  SLOW_MS+=("$slow_ms")
  RPS+=("${rps:-n/a}")

  if [[ "${BENCH_VERBOSE:-0}" == "1" ]]; then
    echo "$output"
    echo
  fi

  sleep "$PAUSE"
done

echo
echo "════════════════════════════════════════════════════════════════════════"
printf "%-22s %10s %10s %10s %10s\n" "Handler" "Avg (ms)" "Fast (ms)" "Slow (ms)" "req/s"
echo "────────────────────────────────────────────────────────────────────────"
for i in "${!LABELS[@]}"; do
  printf "%-22s %10s %10s %10s %10s\n" \
    "${LABELS[$i]}" "${AVGS_MS[$i]}" "${FAST_MS[$i]}" "${SLOW_MS[$i]}" "${RPS[$i]}"
done
echo "════════════════════════════════════════════════════════════════════════"
echo
echo "Lower average (ms) is faster. Rebuild binaries: BENCH_REBUILD=1 just bench-fair"
