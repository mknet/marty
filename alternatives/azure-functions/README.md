# Azure Functions (alternative to Marty)

Rust runs as an **Azure Functions custom handler**: a small HTTP server binary (`handler`) plus `host.json` — **no container** required (unlike typical GCP/Cloud Run for Rust). Closest DX cousin in this repo: AWS Lambda’s `bootstrap` zip via [`../aws-lambda/`](../aws-lambda/).

## How it works (findings)

| | |
|--|--|
| **Artifact** | Linux musl binary named `handler` + `host.json` + function folder (`basic/`) in a zip |
| **Who builds** | You — `x86_64-unknown-linux-musl` (prefer `cargo-zigbuild` on macOS) |
| **Entry** | Tiny HTTP server on `FUNCTIONS_CUSTOMHANDLER_PORT` (Warp in `01_basic`); Azure host proxies `/api/basic` |
| **vs Marty** | Still “compile Linux binary, upload”; Functions host + Consumption plan instead of `cgi-bin/` |
| **vs containers** | Custom handler on Linux Consumption — **no container image** for this sample |
| **Local** | Azure Functions Core Tools (`func start`) after a host-OS `handler` build |

**Hard-won deploy lessons** (tenant, Free Trial regions, zip layout, `FUNCTIONS_WORKER_RUNTIME=Custom`, `linuxFxVersion`): see **[`DEPLOY.md`](DEPLOY.md)**. Defaults assume **`northeurope`** (Free Trial often blocked in `westeurope`). Prefer personal `*.onmicrosoft.com` tenant, not a work directory.

| Example | Marty twin | What it shows |
|---------|------------|---------------|
| [`01_basic/`](01_basic/) | [`examples/01_basic`](../../examples/01_basic/) | Fixed text at `/api/basic`, Consumption plan |
| [`02_complex/`](02_complex/) | [`marty-sync`](../../benchmarks/complex/marty-sync/) | `/api/primes|fibonacci|matrix/…`; fair compare |

## Plan lifecycle: Linux Consumption retires

`01_basic` runs on **Linux Consumption**, and that plan is on its way out:

- **Retires Sept 30, 2028.** No new features or language versions until then.
- Removed from the portal, Visual Studio and VS Code for new apps; CLI and IaC still work (this justfile uses `az`).
- **No in-place migration** to Flex Consumption: create a new app and redeploy (`az functionapp flex-migration` helps).
- Functions runtime v3 on Linux Consumption already stopped on Sept 30, 2026.
- **Flex Consumption supports custom handlers** ("Custom handlers 1.0" in its supported stacks table), so a Flex variant of `01_basic` is possible; not built yet.

This is the "vendor lifecycle" part of lock-in: the same Rust binary keeps working, but the plan around it has an end date. See [`docs/COST.md`](../../docs/COST.md#lock-in-and-setup).

## Data storage

Custom handlers have **no durable local SQLite file**. Natural Azure peers:

| Store | Role vs Marty | Status here |
|-------|---------------|-------------|
| **Azure Table Storage** | Cheap KV / entity rows ≈ “few rows” | planned (`05_sqlite` twin) |
| **Cosmos DB** (Table / NoSQL, serverless or provisioned) | Global KV/document; richer than Table Storage | not sampled |
| **Blob Storage** | Files; also **required** as the Functions app’s backing store | created by `just deploy` |
| **Azure Cache for Redis** | Hot KV / cache | not sampled |
| **Azure Database for MySQL Flexible Server** | Relational ≈ [`07_mysql`](../../examples/07_mysql/) | planned |

## Cost shape

| Piece | Shape | Demo ballpark |
|-------|--------|---------------|
| **Functions Consumption** (legacy, `01_basic`) | Per execution + GB·s; 1 M exec + 400 k GB·s free / month (subscription) | Demo traffic often **~$0** compute; retires Sept 30, 2028 |
| **Functions Flex Consumption** | $0.40 / M executions + $0.000026 / GB·s on demand; **minimum 1,000 ms billed per active period, smallest instance 512 MB**; 250 k exec + 100 k GB·s free / month | Sparse traffic: ≥ 0.5 GB·s per request → **~$13.40 / M**, free grant covers **~200 k** requests. Busy instances share the active period, so much cheaper at high traffic |
| **Storage account** (Tables/Blobs) | GB stored + transactions; Tables are cheap for tiny demos | Usually **cents** at visit-demo scale; **not** in the Functions free grant |
| **Cosmos DB serverless** | RU per request + storage | Fine for spiky demos; provisioned RU/s = **hourly floor** if you pick that mode |
| **MySQL Flexible Server** | **Always-on** vCores + storage | Smallest tiers are a **€/$ monthly floor** (same class of surprise as AWS RDS) — destroy when done |

Rates are list prices checked Oct 2026 (Microsoft's own cost example; regional pricing page did not render numbers; another source cited $0.000016 / GB·s and $0.20 / M for East US, so re-check per region). Full comparison: [`docs/COST.md`](../../docs/COST.md). Overview: [`../README.md`](../README.md).

## Prerequisites

- [Azure CLI](https://learn.microsoft.com/cli/azure/install-azure-cli) (`az login`)
- Rust + `x86_64-unknown-linux-musl` (preferably [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild))
- `zip`
- Optional local: [Azure Functions Core Tools](https://aka.ms/func-core-tools) (`just watch`)

## Just

```bash
cd alternatives/azure-functions
just login tenant=<GUID-or-yourdir.onmicrosoft.com>
just use-sub <subscription-id>
just deploy          # RG + storage + Function App + zip (default: northeurope)
just destroy
just watch
```

Defaults: resource group `marty-alt-azure`, app `marty-alt-az-01-basic`, location **`northeurope`**. Override with `just deploy location=…` or env `AZURE_RG` / `AZURE_FUNCTIONAPP` / `AZURE_LOCATION` / `AZURE_STORAGE`.

Smoke: `curl -sS https://<app>.azurewebsites.net/api/basic` → `88 miles per hour!`
