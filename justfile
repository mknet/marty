# Marty workspace — shared CGI smoke (`scripts/cgi-example-smoke.sh`) and musl cross-build (`_linux_musl`).
# From repo root: `just setup`, `just smoke-01-basic` / `just smoke-02-routing`, or `cd examples/… && just`
# Requires: just, bash, python3. Hurl: `just setup`.

default:
    @just --list

setup:
    cargo install hurl --locked

# Shared: `cargo zigbuild` + ELF check for `package` (workspace member). Used from `examples/*/linux.just`.
_linux_musl package triple:
    #!/usr/bin/env bash
    set -euo pipefail
    ROOT="{{ justfile_directory() }}"
    export PATH="${HOME}/.cargo/bin:${PATH}}"
    PKG="{{package}}"
    TRIPLE="{{triple}}"

    if ! command -v rustup >/dev/null 2>&1; then
      echo "error: rustup not found" >&2
      exit 1
    fi

    if ! command -v zig >/dev/null 2>&1; then
      echo "error: zig not on PATH (required by cargo-zigbuild). Examples: brew install zig | apt install zig" >&2
      exit 1
    fi

    rustup target add "$TRIPLE"

    if ! command -v cargo-zigbuild >/dev/null 2>&1; then
      echo "Installing cargo-zigbuild into ~/.cargo/bin …"
      cargo install cargo-zigbuild --locked
    fi

    cd "$ROOT"
    cargo zigbuild -p "$PKG" --release --target "$TRIPLE"

    OUT="$ROOT/target/$TRIPLE/release/$PKG"
    if [[ ! -f "$OUT" ]]; then
      OUT="$ROOT/target/release/$PKG"
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

    echo "linux musl ok: $OUT ($TRIPLE)"

smoke-01-basic:
    just -f examples/01_basic/justfile smoke

smoke-02-routing:
    just -f examples/02_routing/justfile smoke

build-01-basic:
    just -f examples/01_basic/justfile build

install-01-basic:
    just -f examples/01_basic/justfile install

build-02-routing:
    just -f examples/02_routing/justfile build

install-02-routing:
    just -f examples/02_routing/justfile install

# cwd must be the example dir (./cgi-bin). Prefer: `cd examples/01_basic && just run-server`
run-server:
    cd examples/01_basic && just run-server

run-server-02-routing:
    cd examples/02_routing && just run-server
