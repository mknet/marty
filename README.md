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

Full example: [`examples/02_routing`](examples/02_routing/) (`just smoke-02-routing`).

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
| **Examples** | `01_basic` — fixed response; **`02_routing` — sync `Router`**; `03`–`05` — sessions/cookies/sqlite (Tower); **`06_axum` — optional Axum**; `sample-cgi-script` — kitchen sink |
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

## Shared hosting (all-inkl–style)

Product defaults: [`docs/DIRECTION.md`](docs/DIRECTION.md).

Marty targets **one static CGI binary per app** (upload to `cgi-bin/`, `chmod 755`). Findings from probing and light outside load on [all-inkl.com](https://all-inkl.com) (Oct 2026), mirrored locally in `benchmarks/complex` (`just bench-fair-hosting`) and exercised via `tools/`:

### What the host looks like

| Fact | Implication |
|------|-------------|
| CGI `stdin`/`stdout` are **unix sockets** (`mod_cgid`) | Threaded MPM (event/worker), **not** classic prefork + `mod_cgi` pipes |
| CGI runs as the **FTP/site user** (suexec-style cleaned env) | Only CGI/1.1 + `HTTP_*` + `SSL_*` vars reach the binary; `.htaccess` `SetEnv` toggles do **not** |
| KeepAlive on, chunked responses, HTTP/2 over TLS | Absolute latency includes TLS + RTT (~40–60 ms floor for an empty CGI) |
| Ubuntu 24.04 / Apache 2.4.x | Match with `benchmarks/complex/docker/Dockerfile.hosting` for local A/B |

### Languages on that host

| Runtime | Result |
|---------|--------|
| **Marty / Rust** (static musl) | Works. Small RSS; fine under suexec. |
| **PHP** (FPM / host PHP) | Works. Short routes ≈ Marty (TLS cost dominates); heavy CPU → Marty much faster (~10× on `matrix/400`). |
| **Go** (static `CGO_ENABLED=0`) | **Does not run**: runtime dies at init with `out of memory allocating heap arena map` (address-space limit). Kept in the complex bench as a negative proof. |

So for all-inkl–class CGI, **Rust is the practical compiled option**; Go is out. Versus PHP, Marty is at least as good on light work and clearly better when the request is compute-heavy.

### Two routing stacks (same workloads)

| Binary | Marty features | Router | Binary size idea |
|--------|----------------|--------|------------------|
| `marty-sync` | `sync-cgi` only | `marty::Router` (sync) | Smallest |
| `marty-axum` | `tower` | Axum `Router` + `serve_cgi` | Larger (Tokio/Axum) |

Same suite: [`benchmarks/complex/`](benchmarks/complex/) (`just linux` / `just outside` / `just bench-fair-hosting`). Probe: [`tools/hosting-probe/`](tools/hosting-probe/). Do **not** treat prefork-only Docker numbers as “all-inkl truth”.

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
