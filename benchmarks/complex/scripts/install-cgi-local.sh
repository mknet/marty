#!/usr/bin/env bash
# Install complex CGI binaries into benchmarks/complex/cgi-bin.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
CGI="$HERE/cgi-bin"
mkdir -p "$CGI"

cd "$ROOT"
cargo build -p marty-complex-bench --release
T="$(cargo metadata --format-version=1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"

cp -f "$T/release/marty-complex-bench" "$CGI/marty-complex"
chmod +x "$CGI/marty-complex"

(
  cd "$HERE/go-complex"
  CGO_ENABLED=0 go build -trimpath -ldflags="-s -w" -o "$CGI/go-complex" .
)
chmod +x "$CGI/go-complex"

cp -f "$HERE/python-complex/complex.py" "$CGI/python-complex"
chmod +x "$CGI/python-complex"

echo "installed complex CGI handlers under $CGI"
