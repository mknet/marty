# Example `02_routing`

Sync [`marty::Router`](../../marty/src/router.rs) CGI — the recommended shared-hosting path.

Also the **reference** for consumer deploy: package metadata → `.htaccess` next to the binary. Full write-up: [root README — Quick start](../../README.md#quick-start).

## Layout

| File | Role |
|------|------|
| [`Cargo.toml`](Cargo.toml) | `[package].name` = binary; `[package.metadata.marty] alias` / `path_info` |
| [`build.rs`](build.rs) | `marty::htaccess::write_next_to_binary_from_manifest()` |
| [`src/main.rs`](src/main.rs) | Routes (`/`, `/hello/{name}`) — **not** mirrored in `.htaccess` |

```toml
[package.metadata.marty]
alias = "routing"
path_info = true
```

## Build

Host (smoke / local):

```bash
cargo build -p marty-02-routing
# target/debug/marty-02-routing
# target/debug/marty-02-routing.htaccess
```

Linux musl (upload to `cgi-bin/`):

```bash
# once: zig + cargo-zigbuild (see root README)
cargo zigbuild -p marty-02-routing --release --target x86_64-unknown-linux-musl
# or from this directory: just linux  →  target/deploy/marty-02-routing/
```

Smoke: from repo root `just smoke-02-routing`, or here `just` / `just smoke`.
