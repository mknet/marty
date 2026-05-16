#!/usr/bin/env bash
# Load-test webhook endpoints with hey (https://github.com/rakyll/hey).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASE="${BENCH_BASE_URL:-http://127.0.0.1:8080}"
PAYLOAD="$HERE/fixtures/webhook-post.json"
N="${BENCH_REQUESTS:-5000}"
C="${BENCH_CONCURRENCY:-20}"

# shellcheck source=ensure-hey.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/ensure-hey.sh"
ensure_hey

hey_common=(
  -n "$N"
  -c "$C"
  -m POST
  -H "Content-Type: application/json"
  -H "X-Webhook-Secret: bench-secret"
  -D "$PAYLOAD"
)

run_one() {
  local label="$1"
  local url="$2"
  echo "=== $label ==="
  hey "${hey_common[@]}" "$url"
  echo
}

# CGI stack (Apache docker or _http-cgi-server + benchmarks/simple/cgi-bin)
run_one "marty (Rust CGI)" "$BASE/cgi-bin/marty-webhook"
run_one "go (CGI)" "$BASE/cgi-bin/go-webhook"
run_one "python (stdlib CGI)" "$BASE/cgi-bin/python-webhook"

# mod_php only when Apache serves /php (docker-compose bench service)
if curl -sf -o /dev/null -m 2 -X POST \
  -H "Content-Type: application/json" \
  -H "X-Webhook-Secret: bench-secret" \
  -d @"$PAYLOAD" \
  "$BASE/php/webhook.php"; then
  run_one "php (mod_php)" "$BASE/php/webhook.php"
else
  echo "=== php (mod_php) === skipped ($BASE/php/webhook.php not reachable; use: docker compose up)" >&2
fi
