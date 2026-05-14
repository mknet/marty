# Linux + musl for `marty-02-routing` (shared zigbuild: root `justfile` `_linux_musl`).
# From this directory: `just linux` (x86_64) or `just linux arm` (aarch64).
# On success, copies `htaccess.basic.in` → `dist.htaccess`.

default: (_musl "x86_64-unknown-linux-musl")

arm: (_musl "aarch64-unknown-linux-musl")

[private]
_musl triple:
    #!/usr/bin/env bash
    set -euo pipefail
    HERE="{{ justfile_directory() }}"
    ROOT="$(cd "$HERE/../.." && pwd)"
    just -f "$ROOT/justfile" _linux_musl marty-02-routing "{{ triple }}"
    SRC="$HERE/htaccess.basic.in"
    HTA="$HERE/dist.htaccess"
    test -f "$SRC"
    install -m0644 "$SRC" "$HTA"
    echo "wrote $HTA (Rewrite /routing → cgi-bin/marty-02-routing; deploy as .htaccess on Apache)"
