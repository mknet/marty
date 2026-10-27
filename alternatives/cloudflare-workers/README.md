# Cloudflare Workers (alternative to Marty)

Rust compiles to **WebAssembly** via [`workers-rs`](https://github.com/cloudflare/workers-rs) and runs inside a Cloudflare Worker (V8 isolate). Closest ops cousin in this repo for “no long-lived VPS”: [`../aws-lambda/`](../aws-lambda/) / [`../azure-functions/`](../azure-functions/) — but the artifact is WASM + a JS shim, not a native ELF/`handler` zip.

| Example | Marty twin | What it shows |
|---------|------------|---------------|
| [`01_basic/`](01_basic/) | [`examples/01_basic`](../../examples/01_basic/) | Fixed text over HTTP (`*.workers.dev`), Rust → WASM |

Planned later: **D1** ≈ Marty `05_sqlite` (SQLite-shaped); relational MySQL is atypical on Workers (Hyperdrive / external DB if needed).

Cost sketch: [`docs/COST.md`](../../docs/COST.md). Workers free tier is often enough for demos; paid is per-request / CPU time after that.

## Prerequisites

- Rust + `wasm32-unknown-unknown` (`rustup target add wasm32-unknown-unknown`)
- Node.js / npm (for `npx wrangler`)
- Cloudflare account (`just login`)

## Just

```bash
cd alternatives/cloudflare-workers
just login          # wrangler OAuth (browser)
just whoami
just watch          # local :8787 (rebuilds on change)
just deploy         # publish 01_basic → *.workers.dev
just destroy        # delete the Worker
```

Smoke check after deploy: `curl -sS https://marty-alt-cf-01-basic.<your-subdomain>.workers.dev` → `88 miles per hour!`
