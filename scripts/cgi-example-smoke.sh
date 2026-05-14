#!/usr/bin/env bash
# Run _http-cgi-server from an example directory and execute that directory's smoke.hurl.
# Prerequisites: cgi-bin/ populated, hurl on PATH, workspace built (_http-cgi-server).
# Usage: cgi-example-smoke.sh /absolute/path/to/examples/NN_name
set -euo pipefail

HERE="${1:?usage: $0 /path/to/examples/NN_example}"
test -d "$HERE"
test -f "$HERE/smoke.hurl"

ROOT="$(cd "$HERE/../.." && pwd)"
BASE_URL="${MARTY_CGI_BASE_URL:-http://127.0.0.1:8080}"

if ! command -v hurl >/dev/null 2>&1; then
  echo "error: hurl not on PATH (https://hurl.dev)" >&2
  exit 1
fi

LOG="$(mktemp)"
cleanup() {
  if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -f "$LOG"
}
trap cleanup EXIT

cd "$HERE"
cargo run --manifest-path "$ROOT/Cargo.toml" -p _http-cgi-server >"$LOG" 2>&1 &
SERVER_PID=$!

if ! hurl \
  --no-output \
  --variable "base_url=${BASE_URL}" \
  --retry 50 \
  "$HERE/smoke.hurl"; then
  echo "--- _http-cgi-server log ---" >&2
  cat "$LOG" >&2 || true
  exit 1
fi

echo "smoke ok: Hurl passed (${HERE##*/}) base_url=${BASE_URL}"
