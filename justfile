# Marty workspace — shared CGI smoke (`scripts/cgi-example-smoke.sh`) and musl cross-build (`_linux_musl`).
# From repo root: `just setup`, `just check`, `just smoke-01-basic` … `just smoke-05-sqlite` / `smoke-07-mysql`, or `cd examples/… && just`
# Requires: just, bash, python3 (example `install` recipes). Hurl: `just setup`.

default:
    @just --list

setup:
    cargo install hurl --locked

# Same checks as CI: tests, formatting, clippy (warnings are errors).
check:
    cargo test --workspace --all-features
    cargo fmt --all -- --check
    cargo clippy --workspace --all-features --all-targets -- -D warnings

# Shared musl build + deploy bundle for `package` (workspace member). Used from `examples/*/linux.just`.
# Single source: `tools/cargo-marty` (`cargo marty build`), run from the workspace without installing.
_linux_musl package triple:
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="${HOME}/.cargo/bin:${PATH}"
    cd "{{ justfile_directory() }}"
    cargo run -q -p cargo-marty -- marty build -p "{{package}}" --target "{{triple}}"

smoke-01-basic:
    just -f examples/01_basic/justfile smoke

smoke-02-routing:
    just -f examples/02_routing/justfile smoke

smoke-03-session:
    just -f examples/03_session/justfile smoke

smoke-04-cookies:
    just -f examples/04_cookies/justfile smoke

smoke-05-sqlite:
    just -f examples/05_sqlite/justfile smoke

smoke-07-mysql:
    just -f examples/07_mysql/justfile smoke

build-01-basic:
    just -f examples/01_basic/justfile build

install-01-basic:
    just -f examples/01_basic/justfile install

build-02-routing:
    just -f examples/02_routing/justfile build

install-02-routing:
    just -f examples/02_routing/justfile install

build-03-session:
    just -f examples/03_session/justfile build

install-03-session:
    just -f examples/03_session/justfile install

build-04-cookies:
    just -f examples/04_cookies/justfile build

install-04-cookies:
    just -f examples/04_cookies/justfile install

build-05-sqlite:
    just -f examples/05_sqlite/justfile build

install-05-sqlite:
    just -f examples/05_sqlite/justfile install

build-07-mysql:
    just -f examples/07_mysql/justfile build

install-07-mysql:
    just -f examples/07_mysql/justfile install

# cwd must be the example dir (./cgi-bin). Prefer: `cd examples/01_basic && just run-server`
run-server:
    cd examples/01_basic && just run-server

run-server-02-routing:
    cd examples/02_routing && just run-server

run-server-03-session:
    cd examples/03_session && just run-server

run-server-04-cookies:
    cd examples/04_cookies && just run-server

run-server-05-sqlite:
    cd examples/05_sqlite && just run-server

run-server-07-mysql:
    cd examples/07_mysql && just run-server

# Webhook load benchmarks — benchmarks/simple/ (see benchmarks/README.md)
bench-webhook:
    just -f benchmarks/justfile smoke

bench-webhook-docker:
    just -f benchmarks/justfile docker-smoke

bench-webhook-load:
    just -f benchmarks/justfile bench-docker

bench-webhook-drill:
    just -f benchmarks/justfile bench-drill

bench-webhook-drill-docker:
    just -f benchmarks/justfile bench-drill-docker

# 2000 requests each, Apache docker (mod_php + CGI), results table at end
bench-webhook-fair:
    just -f benchmarks/justfile bench-fair

bench-webhook-fair-rebuild:
    just -f benchmarks/justfile bench-fair-rebuild

bench-complex-fair:
    just -f benchmarks/justfile bench-complex-fair

bench-complex-fair-rebuild:
    just -f benchmarks/justfile bench-complex-fair-rebuild

bench-complex-fair-diagnose:
    just -f benchmarks/justfile complex-bench-fair-diagnose

bench-complex-fair-throughput:
    just -f benchmarks/justfile complex-bench-fair-throughput

bench-complex-fair-stable:
    just -f benchmarks/justfile complex-bench-fair-stable

bench-complex-fair-rust-go-stable:
    just -f benchmarks/justfile complex-bench-fair-rust-go-stable

bench-complex-fair-go-raw-cgi:
    just -f benchmarks/justfile complex-bench-fair-go-raw-cgi

bench-complex-fair-rebuild-diagnose:
    just -f benchmarks/justfile complex-bench-fair-rebuild-diagnose

bench-complex-fair-rebuild-throughput:
    just -f benchmarks/justfile complex-bench-fair-rebuild-throughput

bench-complex-fair-rebuild-native:
    just -f benchmarks/justfile complex-bench-fair-rebuild-native

bench-complex-fair-pgo:
    just -f benchmarks/justfile complex-bench-fair-pgo

bench-complex-spawn:
    just -f benchmarks/justfile complex-bench-spawn

trace-complex-marty-tail:
    just -f benchmarks/justfile complex-trace-marty-tail
