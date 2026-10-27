# `01_basic` — Azure Functions twin of Marty `examples/01_basic`

Marty reference: [`examples/01_basic`](../../../examples/01_basic/) — smallest CGI binary, body `88 miles per hour!`.

This sample is the same text via an **Azure Functions custom handler** (Rust HTTP server + `host.json`). **No container** — Linux musl binary named `handler`, zip-deployed like AWS’s `bootstrap` story.

| | Marty CGI (`01_basic`) | This sample |
|---|------------------------|-------------|
| Entry | `main` → stdout CGI | warp on `FUNCTIONS_CUSTOMHANDLER_PORT` |
| Deploy | musl binary → `cgi-bin/` | musl `handler` + `host.json` → Function App |
| Cost | flat shared hosting | Consumption plan — see [`docs/COST.md`](../../../docs/COST.md) |

Also see AWS twin: [`../../aws-lambda/01_basic`](../../aws-lambda/01_basic/).

## Route

`GET https://<app>.azurewebsites.net/api/basic`

## Just

From `alternatives/azure-functions/`:

```bash
just login tenant=<your.onmicrosoft.com>
just deploy          # resource group + Function App + zip
just destroy         # delete the resource group
just watch           # Core Tools local (needs `func`)
```

Deploy gotchas (region, providers, zip, `linuxFxVersion`): [`../DEPLOY.md`](../DEPLOY.md).
