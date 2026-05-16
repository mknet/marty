#!/usr/bin/env bash
# Install CGI binaries into benchmarks/simple/cgi-bin for examples/_http-cgi-server (no mod_php).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
CGI="$HERE/cgi-bin"
mkdir -p "$CGI"

cd "$ROOT"
cargo build -p marty-webhook-bench --release
T="$(cargo metadata --format-version=1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"

cp -f "$T/release/marty-webhook-bench" "$CGI/marty-webhook"
chmod +x "$CGI/marty-webhook"

(
  cd "$HERE/go-webhook"
  CGO_ENABLED=0 go build -trimpath -ldflags="-s -w" -o "$CGI/go-webhook" .
)
chmod +x "$CGI/go-webhook"

cp -f "$HERE/python-webhook/webhook.py" "$CGI/python-webhook"
chmod +x "$CGI/python-webhook"

echo "installed CGI handlers under $CGI"
