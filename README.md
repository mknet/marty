# Marty

Rust **CGI** for shared hosting: one static binary, upload to `cgi-bin/`, no VPS. Primary path for request/response HTTP — on Hetzner S, warm latency sits with Cloudflare Workers in the Oct 2026 outside compare. Detail: [`docs/DIRECTION.md`](docs/DIRECTION.md#why-ship-on-marty).

CGI runs **one process per request**—simple isolation, gone when the work is done. Prefer **`sync-cgi`** + `marty::Router` (smallest binary). Axum/`tower` is optional if you already have handlers.

Talk: [Back to the future: building “Cloud Functions Lite” with Rust and CGI](https://rustlab.it/talks/back-to-the-future-building-cloud-functions-lite-with-rust-and-cgi) — RustLab 2026 (Bologna).

## Quick start

```toml
# Cargo.toml
[package]
name = "hello-cgi"
version = "0.1.0"
edition = "2024"

[package.metadata.marty]
alias = "hello"       # pretty URL: /hello/…
path_info = true      # /hello/world → PATH_INFO /world

[dependencies]
marty = { version = "0.2", default-features = false, features = ["sync-cgi"] }

[build-dependencies]
marty = { version = "0.2", default-features = false, features = ["htaccess"] }
```

```rust
// build.rs — writes hello-cgi.htaccess next to the binary
fn main() {
    marty::htaccess::write_next_to_binary_from_manifest().unwrap();
}
```

```rust
// src/main.rs
use marty::Router;

fn main() {
    let mut router = Router::new();
    router.get("/", |_| "Hello from Marty CGI!\n");
    router.get("/hello/{name}", |req| {
        let name = req.param("name").unwrap_or("world");
        format!("Hello, {name}!\n")
    });
    router.fallback(|_| (404u16, "Not found\n"));
    router.serve().unwrap();
}
```

Cross-compile for typical Linux shared hosts:

```bash
rustup target add x86_64-unknown-linux-musl
# once: cargo install cargo-zigbuild --locked  (and a Zig toolchain)
cargo zigbuild --release --target x86_64-unknown-linux-musl
# → target/x86_64-unknown-linux-musl/release/hello-cgi
# → target/x86_64-unknown-linux-musl/release/hello-cgi.htaccess
```

Upload the binary to `cgi-bin/` (`chmod 755`). Put the `.htaccess` rules in the site root (or merge the `RewriteRule` lines). Pretty URLs are a **mount** (alias → binary), not one rewrite per `router.get(…)`.

## Deploy tooling

Feature **`htaccess`** generates the Apache mount file from `[package.metadata.marty]` at build time (see above). [`cargo-marty`](tools/cargo-marty/) does the last mile: musl build, bundle, upload, `.htaccess` merge.

```bash
cargo install cargo-marty
cargo marty init prod --preset netcup --host ssh.example.org --user hosting123 --domain example.org
cargo marty deploy prod --dry-run   # show remote operations
cargo marty deploy prod             # build, upload over SFTP (or FTPS), chmod 755, merge .htaccess
cargo marty check prod              # GET https://example.org/hello/ (alias from Cargo.toml) → status, latency, Server
```

Targets (`prod`, `stage`, …) live in `marty.deploy.toml` (no passwords; keep it out of git). Hosters differ only in data: `docroot`, `cgi_dir`, protocol, optionally from a preset. Transport is the system `sftp` / `curl`, so your ssh config and agent apply. Secrets go into one config file **outside the docroot** (`config = "private/marty.conf"`); `cargo marty` uploads it with mode `600` and compiles its path into the binary, which reads it with `marty::Config`.

## Optional: Axum

Reuse existing Axum apps as CGI (`tower` feature). Larger binary; on shared hosting latency is usually TLS/RTT-bound either way.

```rust
use axum::{routing::get, Router};
use marty::serve_cgi;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));
    serve_cgi(app).await.unwrap();
}
```

## Features

| Feature | Role |
|---------|------|
| **`sync-cgi`** | Recommended: sync `Router`, request/response helpers (no Tokio) |
| **`htaccess`** | Build-time Apache mount generator for `build.rs` |
| **`session`** | HMAC-signed cookie sessions for process-per-request CGI (enables `config`) |
| **`config`** | `marty::Config`: env vars first, then one config file outside the docroot (suexec drops `SetEnv`) |
| **`hyper-cgi`** | Default: CGI ↔ `hyper` types |
| **`tower`** | `serve_cgi` for Axum / Tower services |

**Windows** is not supported (Unix CGI environment).

## More in the repository

| Topic | Where |
|-------|--------|
| Worked examples (routing, cookies, sessions, SQLite, MySQL, Axum) | [`examples/`](examples/) |
| State, sessions, and databases with one process per request (FAQ) | [`docs/STATE.md`](docs/STATE.md) |
| Deploy tool (`cargo marty`) | [`tools/cargo-marty/`](tools/cargo-marty/) |
| Product direction, hosters, cost, CGI variants, security | [`docs/`](docs/) |
| Outside benches (shared hosts + cloud twins) | [`benchmarks/`](benchmarks/), [`alternatives/`](alternatives/) |
| Host fingerprint probe | [`tools/hosting-probe/`](tools/hosting-probe/) |

Verified Rust musl CGI (Oct 2026): **Hetzner Webhosting S**, **all-inkl**, **netcup EiWoMiSau**. Details and numbers live under `docs/` and the benchmark snapshots—not here.

Lineage: [cbgbt/cgi-rs](https://github.com/cbgbt/cgi-rs) → [mknet/cgi-rs](https://github.com/mknet/cgi-rs) → [mknet/marty](https://github.com/mknet/marty).

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md) (crate releases start at **0.2.0** as the first full sync-CGI surface).

## License

Apache License 2.0 — see [`LICENSE-APACHE`](LICENSE-APACHE).
