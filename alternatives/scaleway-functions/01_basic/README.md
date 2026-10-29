# `01_basic` — Scaleway Functions twin of Marty `examples/01_basic`

Marty reference: [`examples/01_basic`](../../../examples/01_basic/) — smallest CGI binary, body `88 miles per hour!`.

This sample returns the same plain text from a Scaleway **Serverless Function** (`pub async fn handle`, Axum `Request`/`Response`). You upload **source** (`Cargo.toml` + `src`); Scaleway compiles it.

| | Marty CGI (`01_basic`) | This sample |
|---|------------------------|-------------|
| Entry | `main` → stdout CGI | `pub async fn handle` |
| Artifact | static musl ELF | source zip → managed `rust*` runtime |
| Deploy | upload to `cgi-bin/` | `scw function deploy` |
| Cost | flat shared hosting | per request — see [`docs/COST.md`](../../../docs/COST.md) |

`marty-demo-logic` is a path dependency locally; `just package` / `just deploy` stages a copy into the zip so Scaleway’s build can resolve it.

## Build / deploy

From `alternatives/scaleway-functions/`:

```bash
just login
just package
just deploy
just destroy
```
