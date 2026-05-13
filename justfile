# Marty workspace — smoke for 01_basic lives next to the example.
# From repo root: `just setup` installs hurl; `just smoke-01-basic` or `cd examples/01_basic && just`
# Requires: just, bash, python3 (see examples/01_basic/justfile). Hurl: `just setup`.

default:
    @just --list

# Install the Hurl CLI into Cargo’s bin directory (~/.cargo/bin by default).
setup:
    cargo install hurl --locked

smoke-01-basic:
    just -f examples/01_basic/justfile smoke

build-01-basic:
    just -f examples/01_basic/justfile build

install-01-basic:
    just -f examples/01_basic/justfile install

# Same server as example smoke; must run with cwd examples/01_basic (./cgi-bin). Prefer: cd examples/01_basic && just run-server
run-server:
    cd examples/01_basic && just run-server
