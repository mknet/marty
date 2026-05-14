# Linux + musl for `marty-05-sqlite` (shared zigbuild: root `justfile` `_linux_musl`).
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
    just -f "$ROOT/justfile" _linux_musl marty-05-sqlite "{{ triple }}"
    SRC="$HERE/htaccess.basic.in"
    HTA="$HERE/dist.htaccess"
    test -f "$SRC"
    install -m0644 "$SRC" "$HTA"
    echo "wrote $HTA (Rewrite /sqlite → cgi-bin/marty-05-sqlite; deploy as .htaccess on Apache)"
