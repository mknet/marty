# Alternatives to Marty

Side-by-side sketches of **cloud function** stacks that solve a similar problem as Marty CGI (request in → response out, no long-lived VPS process you manage) — but with a different ops and cost model. Product cost sketch: [`docs/COST.md`](../docs/COST.md).

These are **not** Marty features. Each tree is a small, self-contained sample you can build/deploy with that cloud’s tooling so the Marty pitch stays concrete. **Every cloud example names its Marty twin** under `examples/`. Shared business strings live in [`examples/demo_logic`](../examples/demo_logic/) (`marty-demo-logic`).

## Overview — how each provider runs Rust

Marty itself: **static musl ELF → `cgi-bin/`** on shared hosting. The alternatives below keep “no VPS you babysit,” but change the **artifact** and **who builds**.

| Path | Artifact you ship | Who builds | Runtime shape | EU-native vendor? |
|------|-------------------|------------|---------------|-------------------|
| [`aws-lambda/`](aws-lambda/) | Zip with native `bootstrap` | You (`cargo lambda`) | Custom runtime `provided.al2023` | No (US; EU regions OK) |
| [`azure-functions/`](azure-functions/) | Zip with Linux `handler` + `host.json` | You (musl / zigbuild) | Custom handler (HTTP on loopback) | No (US; EU regions OK) |
| [`cloudflare-workers/`](cloudflare-workers/) | WASM + JS shim (`index.js`) | You (`worker-build`) + Wrangler upload | V8 isolate (not Node); Rust via WASM | No (US; global edge) |
| [`scaleway-functions/`](scaleway-functions/) | **Source** zip (`Cargo.toml` + `src`) | **Scaleway** (managed `rust*` runtime) | FaaS on Knative | **Yes (FR)** |
| [`gcp-cloud-functions/`](gcp-cloud-functions/) | *(placeholder)* | — | For Rust: practically **container** / Cloud Run | No |

**Takeaways**

- **Closest to “upload a binary”:** AWS Lambda and Azure custom handler (you compile Linux, zip, deploy).
- **Different model — WASM edge:** Cloudflare Workers. Entry is always JS; Rust is a library inside the isolate (`workers-rs` / `worker-build`, not “just `wasm-pack`”).
- **Managed language runtime (source upload):** Scaleway — no local musl/`bootstrap`; their builders compile Rust. Path deps must be **staged into the zip** (see that justfile).
- **GCP:** no first-class Rust zip runtime like Lambda/Azure; expect containers — hence still a placeholder here.
- **EU FaaS without Docker:** Scaleway is the clear filled-in option. Clever Functions (WASM) exists but was private alpha when we looked; Fastly Compute / Akamai+Spin are strong WASM peers but not EU vendors.

Per-provider detail, storage options, and cost shape: README in each subdirectory.

## Overview — storing data (vs Marty CGI)

Marty on shared hosting can use a **local SQLite file** next to the binary and/or the hoster’s **MySQL**. Serverless runtimes generally **do not** give you a durable local disk like that — state moves to a managed store.

| Need (Marty twin) | AWS | Azure | Cloudflare | Scaleway | GCP (placeholder) |
|-------------------|-----|-------|------------|----------|-------------------|
| Tiny durable rows ≈ [`05_sqlite`](../examples/05_sqlite/) | **DynamoDB** ([`02_dynamodb`](aws-lambda/02_dynamodb/)) | Table Storage / Cosmos Table | **D1** (SQLite-shaped) or **KV** | Object Storage / Redis / Serverless SQL | Firestore / Memorystore |
| Relational MySQL ≈ [`07_mysql`](../examples/07_mysql/) | **RDS** MySQL ([`03_mysql`](aws-lambda/03_mysql/), optional) | Flexible Server MySQL | Atypical — Hyperdrive → external Postgres/MySQL | Managed MySQL/PostgreSQL or Serverless SQL | Cloud SQL |
| KV / cache | DynamoDB, ElastiCache | Redis Cache, Table Storage | **Workers KV**, Durable Objects | Managed Redis | Memorystore |
| Blobs / files | S3 | Blob Storage | **R2** | Object Storage | GCS |

**Cost pattern (order of magnitude, list prices checked Oct 2026; always re-check tariffs).** Full numbers and assumptions: [`docs/COST.md`](../docs/COST.md), the single source of truth.

Function compute (128 MB, 100 ms, function only, no gateway/egress/logs):

| Provider | Free per month | Beyond that, per 1 M requests |
|----------|----------------|-------------------------------|
| AWS Lambda | 1 M requests + 400 k GB-s | ~$0.41 |
| Azure Flex Consumption | 250 k executions + 100 k GB-s (≈ 200 k short requests) | ~$13.40 on sparse traffic (billed ≥ 1 s × 512 MB) |
| Azure legacy Consumption | 1 M executions + 400 k GB-s | Linux Consumption retires **Sept 30, 2028** |
| GCP Cloud Run functions | 2 M requests + 180 k vCPU-s + 360 k GiB-s | ~$0.60 |
| Cloudflare Workers | Free: 100 k requests/day, 10 ms CPU | Paid: $5/mo incl. 10 M requests + 30 M CPU-ms, then $0.30/M requests + $0.02/M CPU-ms |
| Scaleway Functions | 1 M requests + 400 k GB-s | ~€0.21 |
| **Shared hosting + Marty** | n/a | **~2 to 10 € / month flat** |

**For one simple, stateless function the cloud is cheaper than any hosting plan, often free.** The real price is lock-in, setup and data.

**Azure caveats.** Flex Consumption bills a **minimum of 1,000 ms per active period on a 512 MB smallest instance**, so a 5 ms Rust function on sparse traffic is billed like a 1 s, 512 MB one. Concurrent requests on a busy instance share the active period, so it gets much cheaper at high traffic. The legacy Linux Consumption plan (what [`azure-functions/01_basic`](azure-functions/01_basic/) uses) retires Sept 30, 2028 with **no in-place migration** to Flex; see [`azure-functions/README.md`](azure-functions/README.md).

Data stores:

| Store class | Billing shape | Demo implication |
|-------------|---------------|------------------|
| **Serverless KV / table / D1 / DynamoDB on-demand** | Pay per request (+ cheap GB stored); generous free tiers | Often **≈ $0** for Marty's visit-demo traffic, but the code now speaks a vendor data API |
| **Always-on RDS / Flexible Server / managed Redis node** | **€/$ floor every hour** the instance exists (RDS `db.t4g.micro` ~**$14/mo** in eu-central-1, ~$12 to 15 in eu-west-1, storage extra) | Alone exceeds the whole shared hosting plan; destroy when done |
| **Serverless SQL** (Scaleway Serverless SQL, Aurora Serverless-ish, Cosmos serverless) | Compute while active + storage; can idle cheaper than RDS but not "free disk like SQLite" | Good middle ground; still not flat shared-hosting MySQL |
| **Front door, egress, logs** (AWS example) | HTTP API $1.00/M, egress ~$0.90/M at 10 KB, logs ~$0.25/M | Function ~$0.41/M becomes ~$2.56/M; no ceiling in the cloud |

Marty CGI + shared hosting MySQL/SQLite stays the **flat ~2 to 10 €/mo** story (hosters: [`docs/HOSTERS.md`](../docs/HOSTERS.md)); cloud "few rows" can be cheap, cloud "always-on SQL" quickly costs more than the hoster.

## Status matrix

| Path | Status | Notes |
|------|--------|--------|
| [`aws-lambda/`](aws-lambda/) | `01_basic`, `02_dynamodb`, `03_mysql`, **`04_complex`** | Twins of Marty `01_basic`, `05_sqlite`, `07_mysql`, `marty-sync` |
| [`azure-functions/`](azure-functions/) | `01_basic`, **`02_complex`** | Custom handler; see [`DEPLOY.md`](azure-functions/DEPLOY.md) |
| [`cloudflare-workers/`](cloudflare-workers/) | `01_basic`, **`02_complex`** | Rust → WASM (`workers-rs` + Wrangler); matrix may hit free CPU limits |
| [`scaleway-functions/`](scaleway-functions/) | `01_basic`, **`02_complex`** | Source zip → managed Rust (`scw`); stages `marty-bench-shared` |
| [`gcp-cloud-functions/`](gcp-cloud-functions/) | Placeholder | Likely Cloud Run + container for Rust |

| Marty example | Idea | AWS | Azure | Cloudflare | Scaleway |
|---------------|------|-----|-------|------------|----------|
| [`examples/01_basic`](../examples/01_basic/) | smallest response | [`01_basic`](aws-lambda/01_basic/) | [`01_basic`](azure-functions/01_basic/) | [`01_basic`](cloudflare-workers/01_basic/) | [`01_basic`](scaleway-functions/01_basic/) |
| [`benchmarks/complex/marty-sync`](../benchmarks/complex/marty-sync/) | CPU primes/fib/matrix | [`04_complex`](aws-lambda/04_complex/) | [`02_complex`](azure-functions/02_complex/) | [`02_complex`](cloudflare-workers/02_complex/) | [`02_complex`](scaleway-functions/02_complex/) |
| [`examples/05_sqlite`](../examples/05_sqlite/) | little durable state | [`02_dynamodb`](aws-lambda/02_dynamodb/) | *(planned: Table Storage)* | *(planned: D1)* | *(planned)* |
| [`examples/07_mysql`](../examples/07_mysql/) | relational MySQL | [`03_mysql`](aws-lambda/03_mysql/) | *(planned: Flexible Server)* | *(atypical)* | *(planned)* |

## Fair complex compare (clouds + hosting)

Same algorithms as hosting Run N (`marty-bench-shared`), same routes and load (N=40 C=2 + 1 cold GET per route), warm hey + cold footnote:

```bash
# Deploy each complex twin (see provider READMEs), then:
cp targets.example.env targets.env   # fill *_COMPLEX_URL / *_SYNC_URL
just outside                         # → table; paste into RESULTS-SNAPSHOTS.md
```

Details: [`compare-outside.sh`](compare-outside.sh), snapshots: [`RESULTS-SNAPSHOTS.md`](RESULTS-SNAPSHOTS.md).

### Finding (Run A, 2026-10-28)

Same client laptop; identical primes/fib/matrix JSON. Warm **P50** (ms):

| Route | Hetzner | Cloudflare | Azure | Scaleway | all-inkl | AWS |
|-------|---------|------------|-------|----------|----------|-----|
| primes | **32** | 34 | 45 | 45 | 54 | 56 |
| fibonacci | 32 | **26** | 43 | 40 | 52 | 50 |
| matrix | 33 | **30** | 46 | 53 | 53 | 66 |

**Hetzner Webhosting S** and **Cloudflare Workers** are the top tier end-to-end; Azure/Scaleway mid; all-inkl and AWS Lambda (eu-west-1, 128 MB Function URL) in a slower band. This is front-door/RTT-dominated, not a claim that CGI compute beats every cloud CPU. Azure Consumption cold primes ~768 ms. Full numbers + cold column: [Run A](RESULTS-SNAPSHOTS.md#run-a--outside-compare-clouds--hosting-2026-10-28).

## Not in this tree (yet)

| Option | Why it might appear later |
|--------|---------------------------|
| **Fastly Compute** | Rust-first WASM edge (Wasmtime); strong peer to Cloudflare |
| **Akamai Functions / Spin** | WASM/WASI serverless (Spin/CNCF) |
| **Clever Functions** | EU (FR) WASM FaaS — watch for GA |
| **OVH / STACKIT / IONOS / Hetzner** | No Lambda-like FaaS product; VMs/K8s or Marty CGI on shared hosting instead |
