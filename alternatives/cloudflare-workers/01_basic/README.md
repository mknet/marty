# `01_basic` — Cloudflare Workers twin of Marty `examples/01_basic`

Marty reference: [`examples/01_basic`](../../../examples/01_basic/) — smallest CGI binary, body `88 miles per hour!`.

This sample returns the same plain text from a **Rust → WASM** Worker (`workers-rs` + Wrangler).

| | Marty CGI (`01_basic`) | This sample |
|---|------------------------|-------------|
| Entry | `main` → stdout CGI | `#[event(fetch)]` in a `cdylib` |
| Artifact | static musl ELF | WASM module + JS shim (`worker-build`) |
| Deploy | upload to `cgi-bin/` | `npx wrangler deploy` → `*.workers.dev` |
| Cost | flat shared hosting | Workers free tier / paid requests — see [`docs/COST.md`](../../../docs/COST.md) |

## Build / deploy

From `alternatives/cloudflare-workers/`:

```bash
just login
just watch           # http://127.0.0.1:8787
just deploy          # print workers.dev URL
just destroy
```

Or from this directory: `npx wrangler dev` / `npx wrangler deploy`.
