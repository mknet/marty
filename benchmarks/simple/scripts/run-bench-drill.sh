#!/usr/bin/env bash
# Load-test all webhook handlers with drill (https://github.com/fcsonline/drill).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DRILL_DIR="$HERE/load/drill"
export BENCH_BASE_URL="${BENCH_BASE_URL:-http://127.0.0.1:8080}"
CONCURRENCY="${BENCH_CONCURRENCY:-20}"
ITERATIONS="${BENCH_ITERATIONS:-5000}"
RAMPUP="${BENCH_RAMPUP:-0}"

if ! command -v drill >/dev/null 2>&1; then
  echo "error: drill not on PATH (cargo install drill)" >&2
  exit 1
fi

run_drill() {
  local label="$1"
  local file="$2"
  local tmp
  tmp="$(mktemp)"
  sed -e "s/^concurrency: .*/concurrency: $CONCURRENCY/" \
    -e "s/^iterations: .*/iterations: $ITERATIONS/" \
    -e "s/^rampup: .*/rampup: $RAMPUP/" \
    "$file" >"$tmp"
  echo "=== $label (drill: concurrency=$CONCURRENCY iterations=$ITERATIONS rampup=${RAMPUP}s) ==="
  drill -b "$tmp" --stats
  rm -f "$tmp"
  echo
}

for pair in \
  "marty (Rust CGI)|$DRILL_DIR/marty.yml" \
  "go (CGI)|$DRILL_DIR/go.yml" \
  "python (stdlib CGI)|$DRILL_DIR/python.yml"; do
  label="${pair%%|*}"
  file="${pair#*|}"
  run_drill "$label" "$file"
done

PAYLOAD="$HERE/fixtures/webhook-post.json"
if curl -sf -o /dev/null -m 2 -X POST \
  -H "Content-Type: application/json" \
  -H "X-Webhook-Secret: bench-secret" \
  -d @"$PAYLOAD" \
  "$BENCH_BASE_URL/php/webhook.php"; then
  run_drill "php (mod_php)" "$DRILL_DIR/php.yml"
else
  echo "=== php (mod_php) === skipped (start docker compose for mod_php)" >&2
fi
