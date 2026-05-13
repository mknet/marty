# Linux + musl cross-builds for `marty-01-basic` (cargo-zigbuild + Zig).
# From this directory: `just linux` (x86_64) or `just linux arm` (aarch64).
# On success, copies `htaccess.basic.in` → `dist.htaccess` (Apache: /basic → cgi-bin/marty-01-basic).

# `just linux` — x86_64-unknown-linux-musl
default: (_musl "x86_64-unknown-linux-musl")

# `just linux arm` — aarch64-unknown-linux-musl
arm: (_musl "aarch64-unknown-linux-musl")

[private]
_musl triple:
    #!/usr/bin/env bash
    set -euo pipefail
    HERE="{{ justfile_directory() }}"
    ROOT="$(cd "$HERE/../.." && pwd)"
    export PATH="${HOME}/.cargo/bin:${PATH}"

    if ! command -v rustup >/dev/null 2>&1; then
      echo "error: rustup not found" >&2
      exit 1
    fi

    if ! command -v zig >/dev/null 2>&1; then
      echo "error: zig not on PATH (required by cargo-zigbuild). Examples: brew install zig | apt install zig" >&2
      exit 1
    fi

    rustup target add "{{ triple }}"

    if ! command -v cargo-zigbuild >/dev/null 2>&1; then
      echo "Installing cargo-zigbuild into ~/.cargo/bin …"
      cargo install cargo-zigbuild --locked
    fi

    cd "$ROOT"
    cargo zigbuild -p marty-01-basic --release --target "{{ triple }}"

    TRIPLE="{{ triple }}"
    OUT="$ROOT/target/$TRIPLE/release/marty-01-basic"
    if [[ ! -f "$OUT" ]]; then
      OUT="$ROOT/target/release/marty-01-basic"
    fi
    if [[ ! -f "$OUT" ]]; then
      echo "error: binary not found under target/$TRIPLE/release/ or target/release/" >&2
      exit 1
    fi

    info="$(file -b "$OUT")"
    case "$TRIPLE" in
      x86_64-unknown-linux-musl)
        if ! grep -Eiq 'ELF.*x86-64' <<<"$info"; then
          echo "error: expected Linux x86_64 ELF for $TRIPLE, got: $info" >&2
          echo "hint: zig + cargo-zigbuild must cooperate; see https://github.com/rust-cross/cargo-zigbuild." >&2
          echo "hint: Zig 0.16+ can still yield a macOS binary with some toolchains — try Zig 0.13.x–0.15.x if this persists." >&2
          exit 1
        fi
        ;;
      aarch64-unknown-linux-musl)
        if ! grep -Eiq 'ELF.*(aarch64|ARM aarch64)' <<<"$info"; then
          echo "error: expected Linux aarch64 ELF for $TRIPLE, got: $info" >&2
          echo "hint: zig + cargo-zigbuild must cooperate; see https://github.com/rust-cross/cargo-zigbuild." >&2
          echo "hint: Zig 0.16+ can still yield a macOS binary with some toolchains — try Zig 0.13.x–0.15.x if this persists." >&2
          exit 1
        fi
        ;;
      *)
        echo "error: unsupported triple $TRIPLE" >&2
        exit 1
        ;;
    esac

    SRC="$HERE/htaccess.basic.in"
    HTA="$HERE/dist.htaccess"
    test -f "$SRC"
    install -m0644 "$SRC" "$HTA"

    echo "linux musl ok: $OUT ($TRIPLE)"
    echo "wrote $HTA (Rewrite /basic → cgi-bin/marty-01-basic; deploy as .htaccess on Apache)"
