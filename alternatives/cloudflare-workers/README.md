# Cloudflare Workers (alternative to Marty)

Rust compiles to **WebAssembly** via [`workers-rs`](https://github.com/cloudflare/workers-rs) and runs inside a Cloudflare Worker (**V8 isolate**). Closest ops cousin for “no long-lived VPS”: [`../aws-lambda/`](../aws-lambda/) / [`../azure-functions/`](../azure-functions/) — but the artifact is **WASM + JS shim**, not a native ELF/`handler` zip.

## How it works (findings)

| | |
|--|--|
| **Artifact** | `build/index.js` + `index_bg.wasm` (Wrangler bundle) |
| **Who builds** | You — `worker-build` (cargo → `wasm-bindgen` → `wasm-opt` → **esbuild**), then Wrangler upload |
| **Entry** | Always **JavaScript**. Cloudflare invokes the JS module’s `fetch`; Rust is a `cdylib` behind `wasm-bindgen` glue |
| **vs Node** | **Not** “Node light” — isolate/workerd + Web/Workers APIs (optional Node compat is separate) |
| **vs `wasm-pack` alone** | Pipeline is **`worker-build`**, not a plain `wasm-pack` publish. Workers import `.wasm` as `WebAssembly.Module` (not a ready `Instance`); the glue must instantiate accordingly |
| **Tooling** | [Wrangler](https://developers.cloudflare.com/workers/wrangler/) = Cloudflare’s CLI (`dev` / `deploy` / `login`), analogous to `cargo-lambda` / `az`+zip |

**Why `index.js` exists:** the platform starts an ES module; JS bootstraps WASM, bridges Platform APIs (`Request`/`Response`/bindings), and exports the handler. Without that shim there is nothing for Workers to run.

| Example | Marty twin | What it shows |
|---------|------------|---------------|
| [`01_basic/`](01_basic/) | [`examples/01_basic`](../../examples/01_basic/) | Fixed text on `*.workers.dev`, Rust → WASM |
| [`02_complex/`](02_complex/) | [`marty-sync`](../../benchmarks/complex/marty-sync/) | Same CPU JSON; free Workers CPU may reject matrix |

## Data storage

Workers are isolates — **no local SQLite file** on disk. Platform stores:

| Store | Role vs Marty | Status here |
|-------|---------------|-------------|
| **D1** | Serverless **SQLite** — closest twin to [`05_sqlite`](../../examples/05_sqlite/) | planned |
| **Workers KV** | Eventually consistent KV (config, session-ish, cache) | not sampled |
| **Durable Objects** | Strongly consistent per-object state (SQLite-backed recommended) | not sampled |
| **R2** | S3-compatible object storage (egress-friendly) | not sampled |
| **Hyperdrive** | Connection pool / cache in front of **external** Postgres (or similar) | path for “real SQL” without hosting MySQL on CF |
| External MySQL | Possible over network; **not** a first-class CF product | atypical vs Marty shared-hosting MySQL |

## Cost shape

| Piece | Shape | Demo ballpark |
|-------|--------|---------------|
| **Workers Free** | 100 k requests **per day** (account), 10 ms CPU per invocation | ~3 M requests / month; enough for demos |
| **Workers Paid** | **$5/mo** minimum, includes **10 M requests + 30 M CPU-ms**; then **$0.30 / M requests + $0.02 / M CPU-ms** | A Rust worker returning a string uses ~1 ms CPU → ~$0.32 / M beyond 10 M. Only if you leave Free |
| **KV** | Free: 100k reads/day, 1k writes/day, 1 GB; Paid: ops + $0.50/GB-mo after 1 GB | Tiny KV demos ≈ **$0** on Free |
| **D1** | Free: 5M rows read/day, 100k written/day, 5 GB; Paid: large included, then rows + **~$0.75/GB-mo** storage | Visit-demo scale usually inside Free |
| **R2** | Storage + Class A/B ops; no egress fees to the Internet (CF’s pitch) | Cheap object demos |
| **Always-on SQL elsewhere** | Hyperdrive → your Postgres/RDS still bills **that** database’s floor | Same RDS surprise as AWS/Azure |

List prices checked Oct 2026 ([Workers pricing](https://developers.cloudflare.com/workers/platform/pricing/)). Overview: [`../README.md`](../README.md). Comparison with the other platforms and shared hosting: [`docs/COST.md`](../../docs/COST.md).

## Prerequisites

- Rust + `wasm32-unknown-unknown` (`rustup target add wasm32-unknown-unknown`)
- Node.js / npm (`npx wrangler`)
- Cloudflare account (`just login`)

## Just

```bash
cd alternatives/cloudflare-workers
just login
just whoami
just watch          # local :8787
just deploy         # → *.workers.dev
just destroy
```

Smoke: `curl -sS https://marty-alt-cf-01-basic.<your-subdomain>.workers.dev` → `88 miles per hour!`
