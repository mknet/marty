# Azure Functions (alternative to Marty)

Rust runs as an **Azure Functions custom handler**: a small HTTP server binary (`handler`) plus `host.json` — **no container** required (unlike typical GCP/Cloud Run for Rust). Closest DX cousin in this repo: AWS Lambda’s `bootstrap` zip via [`../aws-lambda/`](../aws-lambda/).

| Example | Marty twin | What it shows |
|---------|------------|---------------|
| [`01_basic/`](01_basic/) | [`examples/01_basic`](../../examples/01_basic/) | Fixed text over HTTP (`/api/basic`), Consumption plan |

Planned later (same story as AWS): Table Storage ≈ Marty `05_sqlite`, Azure Database for MySQL ≈ `07_mysql`.

Cost sketch: [`docs/COST.md`](../../docs/COST.md).

## Prerequisites

- [Azure CLI](https://learn.microsoft.com/cli/azure/install-azure-cli) (`az login`)
- Rust + `x86_64-unknown-linux-musl` (preferably [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild))
- `zip`
- Optional local: [Azure Functions Core Tools](https://aka.ms/func-core-tools) (`just watch`)

## Just

```bash
cd alternatives/azure-functions
just login tenant=<GUID-or-yourdir.onmicrosoft.com>   # avoid work tenants (e.g. EWE)
just use-sub <subscription-id>                        # if several subscriptions
just deploy          # RG + storage + Function App + zip (default: northeurope)
just destroy         # delete resource group (async)
just watch           # local Core Tools
```

Defaults: resource group `marty-alt-azure`, app `marty-alt-az-01-basic`, location **`northeurope`**. Override with `just deploy location=…` or env `AZURE_RG` / `AZURE_FUNCTIONAPP` / `AZURE_LOCATION` / `AZURE_STORAGE`.

**Deploy pitfalls** (wrong tenant, Free Trial regions, macOS zip, empty `linuxFxVersion`, …): see **[`DEPLOY.md`](DEPLOY.md)**. The justfile applies the fixes we found; that doc is the narrative.

Smoke check after deploy: `curl -sS https://<app>.azurewebsites.net/api/basic` → `88 miles per hour!`
