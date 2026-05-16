#!/usr/bin/env bash
# Put common Go/Cargo bin dirs on PATH and install hey if missing.
set -euo pipefail

export PATH="${HOME}/go/bin:${GOPATH:-}/bin:${HOME}/.cargo/bin:${PATH}"

ensure_hey() {
  if command -v hey >/dev/null 2>&1; then
    return 0
  fi
  if command -v go >/dev/null 2>&1; then
    echo ">> hey not found; installing with go install …" >&2
    go install github.com/rakyll/hey@latest
    export PATH="${HOME}/go/bin:${GOPATH:-}/bin:${PATH}"
  fi
  if command -v hey >/dev/null 2>&1; then
    return 0
  fi
  echo "error: hey not on PATH." >&2
  echo "  Install Go, then: go install github.com/rakyll/hey@latest" >&2
  echo "  Ensure \$HOME/go/bin is on PATH (e.g. export PATH=\"\$HOME/go/bin:\$PATH\")." >&2
  return 1
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  ensure_hey
fi
