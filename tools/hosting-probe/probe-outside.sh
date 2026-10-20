#!/usr/bin/env bash
# Outside view of a shared-hosting CGI deployment: protocol, keep-alive policy,
# chunked vs Content-Length, Server header and single-request timing.
# Deliberately low volume: ~8 sequential requests with pauses, no concurrency.
#
# Usage: probe-outside.sh https://example.org/cgi-bin/marty-hosting-probe
set -euo pipefail

URL="${1:-}"
if [[ -z "$URL" ]]; then
  echo "usage: $0 <probe-url>" >&2
  exit 1
fi

PAUSE="${PROBE_PAUSE_SEC:-1}"

hr() { printf '\n== %s ==\n' "$1"; }

hr "response headers (HTTP/1.1, single request)"
curl -sS --http1.1 -o /dev/null -D - "$URL" | tr -d '\r'
sleep "$PAUSE"

hr "keep-alive: two requests on one connection (look for 'Re-using existing connection')"
curl -sS --http1.1 -o /dev/null -o /dev/null -v "$URL" "$URL" 2>&1 \
  | grep -Ei 'connected to|re-using|connection #|closing connection|< connection|< keep-alive|< transfer-encoding|< content-length|< server' || true
sleep "$PAUSE"

hr "http/2 negotiation (TLS only; ignore for plain http)"
curl -sS --http2 -o /dev/null -w 'http_version=%{http_version} code=%{http_code}\n' "$URL" || true
sleep "$PAUSE"

hr "timing, 3 sequential requests (new connection each)"
printf '%-10s %-10s %-10s %-10s %-10s\n' connect appconnect ttfb total size
for _ in 1 2 3; do
  curl -sS --http1.1 -o /dev/null \
    -w '%{time_connect} %{time_appconnect} %{time_starttransfer} %{time_total} %{size_download}\n' "$URL" \
    | awk '{ printf "%-10s %-10s %-10s %-10s %-10s\n", $1, $2, $3, $4, $5 }'
  sleep "$PAUSE"
done

hr "done"
echo "ttfb - appconnect ≈ server-side time incl. CGI fork/exec + compute."
