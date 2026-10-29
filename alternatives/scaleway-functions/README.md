# Scaleway Serverless Functions (alternative to Marty)

**European** FaaS (France): upload a **source zip**; Scaleway builds with a managed **Rust** runtime (Knative under the hood). No container image and no local musl/`bootstrap` — unlike [`../aws-lambda/`](../aws-lambda/) / [`../azure-functions/`](../azure-functions/). Closest DX cousin: managed Go/Python/Node functions on the same product.

## How it works (findings)

| | |
|--|--|
| **Artifact** | Source zip: `Cargo.toml` + `src/` (+ vendored crates) |
| **Who builds** | **Scaleway** — managed runtime e.g. `rust196` |
| **Entry** | `pub async fn handle(Request) -> Response` (Axum-style types in their examples) |
| **vs Marty / AWS / Azure** | You do **not** ship a prebuilt Linux binary; their CI-like builder compiles Rust |
| **vs Cloudflare** | No WASM/Workers isolate story — language FaaS with source upload |
| **CLI** | [`scw`](https://github.com/scaleway/scaleway-cli) — Homebrew: `brew install scw` |

**`demo_logic`:** path deps outside the zip break remote `cargo`. `just package` stages a copy of [`examples/demo_logic`](../../examples/demo_logic/) beside a `Cargo.toml.stage` so the uploaded archive is self-contained.

**Regions:** `fr-par` (default), `nl-ams`, `pl-waw`. For EU vendor + Rust without Docker, Scaleway is the filled-in option in this tree (OVH/STACKIT/IONOS/Hetzner have no Lambda-like FaaS product).

| Example | Marty twin | What it shows |
|---------|------------|---------------|
| [`01_basic/`](01_basic/) | [`examples/01_basic`](../../examples/01_basic/) | Fixed text over HTTP, runtime `rust196` |
| [`02_complex/`](02_complex/) | [`marty-sync`](../../benchmarks/complex/marty-sync/) | primes/fib/matrix; stages `marty-bench-shared` in zip |

## Data storage

No durable local disk on the function. Scaleway ecosystem options:

| Store | Role vs Marty | Status here |
|-------|---------------|-------------|
| **Object Storage** | Blobs / simple persisted files | not sampled |
| **Managed Redis** | KV / cache | not sampled (always-on node pricing) |
| **Serverless SQL Database** | Usage-based SQL (vCPU while active + GB stored) | planned-ish middle ground vs RDS |
| **Managed Database** (PostgreSQL / MySQL) | Classic relational ≈ [`07_mysql`](../../examples/07_mysql/) | planned; **hourly instance** floor |
| **Queues / NATS** | Messaging, not primary “visit rows” store | not sampled |

Functions can attach to Private Networks / VPC to reach managed DBs securely.

## Cost shape

List prices before tax (Scaleway public tariffs, late 2026 — re-check):

| Piece | Shape | Demo ballpark |
|-------|--------|---------------|
| **Serverless Functions** | €0.15 / 1 M requests + €0.000005 / GB-s (€0.50 per 100 k GB-s) after free tier (**1 M requests** + **400 k GB-s** / mo, per account) | Demo traffic **€0**. Beyond free: 128 MB × 100 ms = 12,500 GB-s per M → €0.06, so **~€0.21 / M requests**, the cheapest in [`docs/COST.md`](../../docs/COST.md) |
| **min_scale > 0** | Extra **provisioned** GB·s while warm | Avoid for demos (idle cost) |
| **Serverless SQL** | ~€0.14/vCPU-hour while active (min billing window) + ~€0.000272/GB-hour storage | Can stay low if mostly idle; not free like CGI SQLite |
| **Managed Redis** `RED1-MICRO` | From ~**€0.05/hour** main node | ~**€35+/mo** floor if left up |
| **Managed MySQL/Postgres** | Instance size €/hour + storage | Same class as AWS RDS — **destroy when done** |

**Rate caveat:** the FAQ and the pricing page disagreed on the GB-s rate when checked (Oct 2026). FAQ: €1.20 per 100 k GB-s, provisioned €0.36; pricing page: €0.50 per 100 k GB-s on demand, €1.70 per 100 k GB-s provisioned. The pricing page values are used here.

**Sovereignty:** Scaleway is a French vendor (regions fr-par, nl-ams, pl-waw), so no US CLOUD Act question, but still its own entry point (`handle`) and CLI (`scw`), so leaving still means a rewrite of the entry point. Sovereignty is about jurisdiction and exit, not security.

Overview: [`../README.md`](../README.md). Compute comparison: [`docs/COST.md`](../../docs/COST.md).

## Prerequisites

- [Scaleway account](https://console.scaleway.com/) + API key
- Scaleway CLI: `brew install scw` (or [install docs](https://www.scaleway.com/en/docs/scaleway-cli/quickstart/))
- `zip`

## Just

```bash
cd alternatives/scaleway-functions
just login          # scw init
just whoami
just package        # stage demo_logic into zip
just deploy         # namespace + function + upload
just destroy
```

Defaults: namespace `marty-alt-scw`, function `marty-alt-scw-01-basic`, region **`fr-par`**, runtime **`rust196`**. Override with `SCW_DEFAULT_REGION` / `SCW_NAMESPACE` / `SCW_FUNCTION` / `SCW_RUNTIME`.

Smoke: `curl -sS https://<endpoint>` → `88 miles per hour!`
