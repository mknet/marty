# Linux + musl for `marty-01-basic` (shared zigbuild: root `justfile` `_linux_musl`).
# From this directory: `just linux` (x86_64) or `just linux arm` (aarch64).
# Stages `target/deploy/marty-01-basic/` from the musl binary + sibling `*.htaccess`.

default: (_musl "x86_64-unknown-linux-musl")

arm: (_musl "aarch64-unknown-linux-musl")

[private]
_musl triple:
    #!/usr/bin/env bash
    set -euo pipefail
    HERE="{{ justfile_directory() }}"
    ROOT="$(cd "$HERE/../.." && pwd)"
    just -f "$ROOT/justfile" _linux_musl marty-01-basic "{{ triple }}"
