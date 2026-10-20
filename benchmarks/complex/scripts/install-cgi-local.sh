#!/usr/bin/env bash
# Install complex CGI binaries into benchmarks/complex/cgi-bin.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
CGI="$HERE/cgi-bin"
mkdir -p "$CGI"

cd "$ROOT"
rustflags="${MARTY_RUSTFLAGS:-}"
if [[ -n "${MARTY_TARGET_CPU:-}" ]]; then
  rustflags="${rustflags} -Ctarget-cpu=${MARTY_TARGET_CPU}"
fi
if [[ -n "${MARTY_PGO_PROFILE:-}" ]]; then
  rustflags="${rustflags} -Cprofile-use=${MARTY_PGO_PROFILE}"
fi
if [[ -n "$rustflags" ]]; then
  echo "RUSTFLAGS=${rustflags}"
fi
CARGO_INCREMENTAL=0 RUSTFLAGS="$rustflags" cargo build -p marty-sync --profile bench
T="$(cargo metadata --format-version=1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"

cp -f "$T/release/marty-sync" "$CGI/marty-sync"
chmod +x "$CGI/marty-sync"

(
  cd "$HERE/go-complex"
  CGO_ENABLED=0 go build -trimpath -ldflags="-s -w" -o "$CGI/go-complex" .
)
chmod +x "$CGI/go-complex"

cp -f "$HERE/python-complex/complex.py" "$CGI/python-complex"
chmod +x "$CGI/python-complex"

echo "installed complex CGI handlers under $CGI"
