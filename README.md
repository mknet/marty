# Marty

> *“Roads? Where we’re going, we don’t need roads.”*  
> — Doc Brown, *Back to the Future* (and yet: sometimes all you need is **one process per request**—and we call that **CGI**.)

**Marty** helps you write **Rust CGI** for [shared hosting](#shared-hosting-all-inkl-style): one static binary, upload to `cgi-bin/`, no VPS required. Lineage: [**cbgbt/cgi-rs**](https://github.com/cbgbt/cgi-rs) → [**mknet/cgi-rs**](https://github.com/mknet/cgi-rs) → this tree ([**github.com/mknet/marty**](https://github.com/mknet/marty)).

Product defaults: [`docs/DIRECTION.md`](docs/DIRECTION.md) — **sync-first**; Axum/`tower` is optional for existing handlers. CGI stacks → [`docs/CGI-VARIANTS.md`](docs/CGI-VARIANTS.md). Cost sketch → [`docs/COST.md`](docs/COST.md).

## Lineage

| Stage | Repository |
|--------|------------|
| **Original `cgi-rs`** | [**github.com/cbgbt/cgi-rs**](https://github.com/cbgbt/cgi-rs) |
| **Maintained fork** | [**github.com/mknet/cgi-rs**](https://github.com/mknet/cgi-rs) |
| **Crates.io & this tree** | [**github.com/mknet/marty**](https://github.com/mknet/marty) |

---

## Great Scott — what is this?

A CGI server **launches** a script as its **own process** for each request: clean isolation, gone when the work is done. That trades some throughput for a **simple resource story**—ideal when you want Rust on a shared host instead of a long-lived VPS process.

---

## Recommended: sync router (`sync-cgi`)

Smallest binary, no Tokio. Feature **`sync-cgi`** (disable default features if you want to drop hyper):

```rust ignore
// marty = { version = "…", default-features = false, features = ["sync-cgi"] }
use marty::Router;

fn main() {
    let mut api = Router::new();
    api.get("/hello/{name}", |req| {
        let name = req.param("name").unwrap_or("world");
        format!("Hello, {name}!\n")
    });

    let mut router = Router::new();
    router.get("/", |_| "Hello, World!\n");
    router.nest("/api", api);
    router.get("/static/{*path}", |req| {
        format!("file {}\n", req.param("path").unwrap_or(""))
    });
    router.fallback(|_| (404u16, "Not found\n"));
    router.serve().unwrap();
}
```

Full example: [`examples/02_routing`](examples/02_routing/) (`just smoke-02-routing`). Consumer deploy layout (Cargo.toml + `build.rs` + musl): see below and that example’s [README](examples/02_routing/README.md).

### Your app: binary + `.htaccess`

Pretty URLs on Apache are a **mount** (alias → CGI binary), not a dump of every `Router` route. Feature **`htaccess`** (no extra deps).

**`Cargo.toml`** — package name = binary; alias = pretty URL segment:

```toml
[package]
name = "my-cgi"
version = "0.1.0"
edition = "2024"

[package.metadata.marty]
alias = "app"
path_info = true   # optional: /app/foo → cgi-bin/my-cgi/foo

[dependencies]
marty = { version = "0.1", default-features = false, features = ["sync-cgi"] }

[build-dependencies]
marty = { version = "0.1", default-features = false, features = ["htaccess"] }
```

**`build.rs`**

```rust ignore
fn main() {
    marty::htaccess::write_next_to_binary_from_manifest().unwrap();
}
```

**Output** (after `cargo build --release`):

```
target/release/my-cgi
target/release/my-cgi.htaccess
```

Upload the binary to `cgi-bin/` (`chmod 755`) and the `.htaccess` contents to the site root (or merge the `RewriteRule` lines). Changing `router.get(…)` does **not** change the `.htaccess` — only alias / package name / `path_info` do.

**Linux musl (shared hosting):** a normal `cargo build` on macOS produces a macOS binary. For an uploadable static CGI, cross-compile (no Docker required):

```bash
# once: brew install zig && cargo install cargo-zigbuild --locked
rustup target add x86_64-unknown-linux-musl
cargo zigbuild -p my-cgi --release --target x86_64-unknown-linux-musl
# → target/x86_64-unknown-linux-musl/release/my-cgi{,.htaccess}
```

In this repo the same flow is `just linux` under an example (stages `target/deploy/<pkg>/`). Worked example: [`examples/02_routing`](examples/02_routing/). Multi-app sample: [`examples/apache-combined-vhost.htaccess`](examples/apache-combined-vhost.htaccess).

---

## Optional: Axum / Tower (`tower`)

Reuse **existing** Axum handlers as CGI via [`serve_cgi`](https://docs.rs/marty). Larger binary; latency on shared hosting ≈ sync (TLS/RTT dominates). See [`examples/06_axum`](examples/06_axum/) and [`docs/DIRECTION.md`](docs/DIRECTION.md).

```rust ignore
// marty = { version = "…", features = ["tower"] }
use axum::{routing::get, Router};
use marty::serve_cgi;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));
    serve_cgi(app).await.unwrap();
}
```

---

## Workspace map

| Theme | Idea |
|--------|------|
| **Sync CGI** | Feature `sync-cgi`: [`CgiRequest`](marty/src/cgi.rs) / [`CgiResponse`](marty/src/cgi.rs) / [`Router`](marty/src/router.rs) |
| **Hyper CGI** | Default `hyper-cgi`: CGI env → `hyper::Request` / `hyper::Response` |
| **Tower / Axum** | Feature `tower`: `serve_cgi` + `multi_mount_cgi_router*` |
| **Dev server** | `examples/_http-cgi-server` — first `/cgi-bin/` segment is the binary, rest is `PATH_INFO` |
| **Examples** | `01_basic` — fixed response; **[`02_routing`](examples/02_routing/) — sync `Router` + deploy docs**; `03`–`05` — sync session/cookies/sqlite; **`06_axum` — optional Axum**; [`07_mysql`](examples/07_mysql/) — MySQL/MariaDB (shared hosting); `sample-cgi-script` — kitchen sink |
| **`.htaccess`** | Feature `htaccess`: `[package.metadata.marty]` + `build.rs` → `{binary}.htaccess`; musl via `cargo zigbuild` / `just linux` |
| **Benches** | [`benchmarks/complex/`](benchmarks/complex/) (Docker + upload/outside); host probe: [`tools/hosting-probe/`](tools/hosting-probe/) |

Local smoke (build → `cgi-bin/` → `_http-cgi-server` → [Hurl](https://hurl.dev)): `just setup`, then e.g. `just smoke-02-routing`.

**Windows:** not supported (Unix CGI env / `OsStrExt`).

### Meta-variables (RFC 3875)

```rust ignore
fn main() {
    let method = marty::MetaVariableKind::RequestMethod
        .try_from_env()
        .unwrap();
    assert_eq!(method.as_str().unwrap(), "GET");
}
```

---

## Shared hosting (all-inkl– / Hetzner-S–style)

Product defaults: [`docs/DIRECTION.md`](docs/DIRECTION.md). Host shortlist + prices: [`docs/HOSTERS.md`](docs/HOSTERS.md).

Marty targets **one static CGI binary per app** (upload to `cgi-bin/`, `chmod 755`). Findings from probing and light outside load on **[all-inkl.com](https://all-inkl.com)** and **[Hetzner Webhosting S](https://www.hetzner.com/webhosting)** (Oct 2026), mirrored locally in `benchmarks/complex` (`just bench-fair-hosting`) and exercised via `tools/`. Head-to-head numbers: [`RESULTS-SNAPSHOTS.md` Run N](benchmarks/complex/RESULTS-SNAPSHOTS.md#run-n--outside-compare-hetzner-s-vs-all-inkl-2026-10-28).

### What the host looks like

| Fact | Implication |
|------|-------------|
| CGI `stdin`/`stdout` are **unix sockets** (`mod_cgid`-class) | Threaded MPM (event/worker), **not** classic prefork + `mod_cgi` pipes — confirmed on **both** hosts |
| CGI runs as the **FTP/site user** (suexec-style cleaned env) | Only CGI/1.1 + `HTTP_*` + `SSL_*` vars reach the binary; `.htaccess` `SetEnv` toggles do **not** (all-inkl; assume same elsewhere) |
| KeepAlive on, chunked responses, HTTP/2 over TLS | Absolute latency includes TLS + RTT (empty CGI floor depends on client↔host path; ~33 ms Hetzner vs ~50–54 ms all-inkl in one same-client outside run) |
| all-inkl: Ubuntu 24.04 / Apache 2.4.x; Hetzner S: Debian 13 | Local A/B: `benchmarks/complex/docker/Dockerfile.hosting` (Ubuntu hosting-like mirror) |

### Languages on that host

| Runtime | Result |
|---------|--------|
| **Marty / Rust** (static musl) | Works on all-inkl and Hetzner S. Small RSS; fine under suexec-style env. |
| **PHP** (FPM / host PHP) | Works on both. Short routes ≈ Marty (TLS cost dominates); heavy CPU → Marty much faster (matrix on outside; ~10× on `matrix/400` extreme). |
| **Go** (static `CGO_ENABLED=0`) | **Does not run on all-inkl**: runtime dies at init with `out of memory allocating heap arena map` (address-space limit). Not re-probed on Hetzner. Kept in the complex bench as a negative proof. |

So for this CGI class, **Rust is the practical compiled option**; Go is out on all-inkl. Versus PHP, Marty is at least as good on light work and clearly better when the request is compute-heavy — **same relative pattern on Hetzner and all-inkl**.

### Two routing stacks (same workloads)

| Binary | Marty features | Router | Binary size idea |
|--------|----------------|--------|------------------|
| `marty-sync` | `sync-cgi` only | `marty::Router` (sync) | Smallest |
| `marty-axum` | `tower` | Axum `Router` + `serve_cgi` | Larger (Tokio/Axum) |

Same suite: [`benchmarks/complex/`](benchmarks/complex/) (`just linux` / `just outside` / `just bench-fair-hosting`). Probe: [`tools/hosting-probe/`](tools/hosting-probe/). Do **not** treat prefork-only Docker numbers as shared-host truth.

### Latency vs cloud alternatives

Same complex workload on AWS / Azure / Cloudflare / Scaleway + Hetzner / all-inkl (`alternatives/just outside`, Oct 2026): warm P50 **Hetzner S ≈ Cloudflare** (fastest); Azure/Scaleway mid; **all-inkl ≈ AWS** slower. Tables: [`alternatives/RESULTS-SNAPSHOTS.md`](alternatives/RESULTS-SNAPSHOTS.md).

### Cost, honestly

For **one stateless function** with little traffic, a cloud free tier (Lambda, Cloud Run, Workers, Scaleway) is cheaper than any hosting plan: it is free. Marty pays off when there is **data** (SQLite next to the binary or the hoster's MySQL, included in a ~2 to 10 € flat plan, vs an always-on cloud DB floor or a vendor data API) and when you want no lock-in to a vendor entry point. Numbers: [`docs/COST.md`](docs/COST.md). Cheapest verified plan so far: **Hetzner S (1.90 €)**; others still to probe: [`docs/HOSTERS.md`](docs/HOSTERS.md).

---

## License

Apache License, Version 2.0 — see [`LICENSE-APACHE`](LICENSE-APACHE) in this repository (same text as upstream [mknet/cgi-rs](https://github.com/mknet/cgi-rs) / [cbgbt/cgi-rs](https://github.com/cbgbt/cgi-rs)).

---

## Acknowledgements / *Easter eggs*

- **Doc** would yell *“1.21 gigawatts!”*—here you often need fewer watts, but you do need solid **types**.
- **Marty** might forget to `git revert` *Johnny B. Goode*—you should still ship **versions** and a **changelog**.
- If something will not compile: `cargo clean`, then try again—like the **second attempt** at the flux capacitor wiring harness.

---

*“If you put your mind to it, you can accomplish anything.”* — George McFly (and a halfway decent `README.md`).
