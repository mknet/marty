#!/usr/bin/env bash
# Phase timings with shared client reference (X-Bench-Sent-Us).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASE="${BENCH_BASE_URL:-http://127.0.0.1:8081}"
ROUTE="${PROFILE_ROUTE:-fibonacci/42}"

handlers=(
  "marty-complex"
  "go-complex"
  "python-complex"
  "php"
)

render_profile() {
  local name="$1"
  local client_us="$2"
  local body="$3"
  echo "=== ${name} ==="
  if ! echo "$body" | jq -e '.profile' >/dev/null 2>&1; then
    echo "  (no profile — rebuild handlers and ensure ?profile=1)"
    echo
    return
  fi
  echo "$body" | jq -r --argjson client "$client_us" '
    .profile as $p |
    ($client - ($p.total_us // 0)) as $outside |
    "  client_total_us:    \($client)",
    "  until_process_us:   \($p.until_process_us // "—")  (send → main)",
    "  until_compute_us:   \($p.until_compute_us // "—")  (send → compute)",
    "  startup_us:         \($p.startup_us)  (in-process, main → handler)",
    "  handler_setup_us:   \($p.handler_setup_us)",
    "  compute_us:         \($p.compute_us)",
    "  post_compute_us:    \($p.post_compute_us)",
    "  total_us:           \($p.total_us)  (in-process total)",
    "  outside_process_us: \($outside)  (client_total − total_us ≈ fork + net + queue)"
  '
  echo
}

probe() {
  local name="$1"
  local url="${BASE}/cgi-bin/${name}/${ROUTE}?salt=0&profile=1"
  if [[ "$name" == "php" ]]; then
    url="${BASE}/php/complex.php/${ROUTE}?salt=0&profile=1"
  fi

  local sent_us body client_us
  sent_us="$(python3 -c 'import time; print(int(time.time() * 1e6))')"
  local tmp
  tmp="$(mktemp)"
  local curl_secs
  curl_secs="$(curl -fsS -o "$tmp" -w '%{time_total}' \
    -H "X-Bench-Sent-Us: ${sent_us}" \
    "$url")"
  body="$(cat "$tmp")"
  rm -f "$tmp"
  client_us="$(python3 -c "print(int(float('${curl_secs}') * 1_000_000))")"
  render_profile "$name" "$client_us" "$body"
}

echo "Profiling ${ROUTE} with X-Bench-Sent-Us (all times in microseconds)."
echo "Use localhost/Docker on one machine so client and server clocks align."
echo

for h in "${handlers[@]}"; do
  probe "$h" || true
done

echo "--- recent bench-timing stderr (docker) ---"
if docker compose -p marty-bench-complex ps -q bench 2>/dev/null | grep -q .; then
  docker compose -p marty-bench-complex logs bench 2>&1 | tail -60 | grep 'bench-timing' \
    || echo "(no stderr lines — JSON above is authoritative)"
else
  echo "(stack not running)"
fi
