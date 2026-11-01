# Cost sketch: Marty on shared hosting vs cloud functions

Illustrative comparison for the pitch "Rust web without a VPS". **Prices change**; treat every number as order of magnitude. List prices were **checked Oct 2026** from public price pages (sources at the bottom). Always re-check current hoster and cloud tariffs before quoting.

This page is the **single source of truth** for cost numbers in this repo. Per-provider READMEs under [`alternatives/`](../alternatives/) repeat the same values or link here. Hoster prices: [`HOSTERS.md`](HOSTERS.md).

## Short version

1. **For one simple, stateless function the cloud is cheaper than any hosting plan, often free.** A webhook with a few thousand calls a month costs nothing on Lambda, Cloud Run, Workers or Scaleway. A shared hosting plan costs ~2 to 10 € whether you use it or not.
2. **The real price of the cloud is lock-in, setup and data**, not the per-request rate (see [Lock-in and setup](#lock-in-and-setup)).
3. **As soon as there is data**, the cloud bill gets a floor (always-on DB) or the code gets a vendor API (DynamoDB, D1, Table Storage). Marty keeps a SQLite file next to the binary or uses the hoster's MySQL, included in the flat plan.
4. **Rule of thumb:** one stateless function, low traffic, no data: take the cloud free tier. A small app with data on a host you already pay for: Marty + `cgi-bin/`.

## Setup A: Marty CGI on shared hosting

| Item | Assumption |
|------|------------|
| Host | Shared web hosting with customer CGI (**all-inkl**, **Hetzner Webhosting S**, **netcup EiWoMiSau** verified Oct 2026; others: probe first, see [`HOSTERS.md`](HOSTERS.md)) |
| Price | ~**2 to 10 € / month** flat incl. VAT (**Hetzner S 1.90 €** cheapest verified, netcup 1000 2.69 € still probe, netcup EiWoMiSau ~6.45 € verified, all-inkl Privat 4.95 € / Premium 9.95 €, manitu M 5.99 €) |
| Included | Domain, TLS, mail, webspace, usually MySQL; traffic under fair use |
| App | One static Marty binary in `cgi-bin/` (+ static assets / frontend build) |
| Ops | FTP/SFTP upload, `chmod 755`, no container, no per-request bill |

**Rough monthly cost:** the plan price, whether you get 10 k or a few million light hits, until fair-use or CPU limits kick in. Scaling "up" means a bigger plan or a VPS, not a line item per million invocations. **The plan price is also the ceiling.**

**Latency (not cost):** on the shared complex outside bench (Oct 2026), **Hetzner S** matched **Cloudflare Workers** for warm P50 and beat AWS / Azure / Scaleway / all-inkl from the same client — end-to-end, same algorithms. **netcup EiWoMiSau** outside ~72 ms sync P50 (~2× Hetzner); fine under steady 320/min, fragile under hard parallel grill (IP ban). See [`../alternatives/RESULTS-SNAPSHOTS.md`](../alternatives/RESULTS-SNAPSHOTS.md) and [`../benchmarks/complex/RESULTS-SNAPSHOTS.md`](../benchmarks/complex/RESULTS-SNAPSHOTS.md) Run O.

## Setup B: cloud functions

Assumptions everywhere below unless stated: **128 MB memory, 100 ms billed duration**, function only (no gateway, egress, logs), EUR ≈ USD treated 1:1.

### Same function, five platforms

The business logic is one line everywhere (`marty_demo_logic::basic::BODY`, "88 miles per hour!"). The entry point, build tool, CLI and front door differ per vendor. That column is the lock-in.

| Platform | You ship | Who builds | Entry point | EU vendor |
|---|---|---|---|---|
| Marty CGI | static musl ELF | you, `cargo zigbuild` | `fn main()`, env + stdin → stdout (RFC 3875) | depends on hoster |
| [AWS Lambda](../alternatives/aws-lambda/) | zip with `bootstrap` (`provided.al2023`) | you, `cargo lambda` | `lambda_http::run(service_fn(..))` | no (EU regions) |
| [Azure Functions](../alternatives/azure-functions/) | zip: `handler` + `host.json` + function folder | you, musl / zigbuild | HTTP server on `FUNCTIONS_CUSTOMHANDLER_PORT` | no (EU regions) |
| [Cloudflare Workers](../alternatives/cloudflare-workers/) | WASM + JS shim (`index.js`) | you, `worker-build` + Wrangler | `#[event(fetch)] async fn fetch(..)` | no (global edge) |
| [Scaleway Functions](../alternatives/scaleway-functions/) | **source** zip (`Cargo.toml` + `src`) | Scaleway, managed `rust196` runtime | `pub async fn handle(Request) -> Response` | **yes (FR)** |
| [GCP Cloud Run functions](../alternatives/gcp-cloud-functions/) | container image | you, Docker | your own HTTP server | no |

### Free tiers (per month)

| Provider | Free per month | Effective free requests for a short function* |
|---|---|---|
| AWS Lambda | 1 M requests + 400,000 GB-s | **1 M** (requests are the limit) |
| Azure Functions Flex Consumption | 250,000 executions + 100,000 GB-s | **~200,000** (GB-s are the limit, see below) |
| Azure Functions legacy Consumption | 1 M executions + 400,000 GB-s | 1 M, but Linux Consumption retires **Sept 30, 2028** |
| GCP Cloud Run functions (request-based, tier 1) | 2 M requests + 180,000 vCPU-s + 360,000 GiB-s | **2 M** |
| Cloudflare Workers Free | 100,000 requests **per day**, 10 ms CPU per invocation | **~3 M** |
| Scaleway Serverless Functions | 1 M requests + 400,000 GB-s | **1 M** |

\* Short function = 128 MB, 100 ms. For AWS, GCP and Scaleway the compute grant covers many times the request grant, so the request count is the limit. Azure Flex is the reverse.

**Scope:** AWS and Scaleway per account, Azure per subscription, GCP per billing account (aggregated across projects). Multiple functions share one grant.

### Prices beyond the free tier

Free tier not applied.

| Provider | Rates | Per 1 M requests |
|---|---|---|
| AWS Lambda | $0.20 per M requests + $0.0000166667 per GB-s; 0.0125 GB-s per request → $0.21 compute | **~$0.41** |
| Azure Flex (on demand) | $0.40 per M executions + $0.000026 per GB-s; **minimum billable period 1,000 ms, smallest instance 512 MB** → ≥ 0.5 GB-s per sparse request | **~$13.40** |
| GCP Cloud Run functions | $0.40 per M requests + $0.000024 per vCPU-s + $0.0000025 per GiB-s (128 MB gets ~0.083 vCPU) | **~$0.60** |
| Cloudflare Workers Paid | $5/month minimum, includes 10 M requests and 30 M CPU-ms; then $0.30 per M requests + $0.02 per M CPU-ms. A Rust worker returning a string uses ~1 ms CPU | $5 covers 10 M, then **~$0.32** |
| Scaleway Functions | €0.15 per M requests + €0.000005 per GB-s (€0.50 per 100 k GB-s); 12,500 GB-s per M → €0.06 | **~€0.21** |

**Worked example (AWS):**

- 128 MB, **100 ms** billed ⇒ 0.0125 GB-s per request
- Compute ≈ 0.0125 × $0.0000166667 ≈ **$0.00000021 / request**
- Requests ≈ **$0.00000020 / request**
- **≈ $0.41 / million requests** before free tier, egress, front door, logs

**Caveats**

- **Azure:** $0.000026/GB-s and $0.40/M are the on-demand rates Microsoft uses in its own cost example ("Estimating consumption-based costs"); the regional pricing page did not render numbers when fetched. Another source (MS Docs GitHub) cited $0.000016/GB-s and $0.20/M for East US: re-check per region. The 1 s minimum and 512 MB smallest instance come from the Flex Consumption docs. Concurrent requests on a busy instance share the active period, so **at high traffic Flex gets much cheaper per request**. The ~$13 is a sparse-traffic number, a billing rule rather than a price.
- **Scaleway:** FAQ and pricing page disagreed on the GB-s rate (FAQ: €1.20 per 100 k GB-s, provisioned €0.36; pricing page: €0.50 per 100 k GB-s on demand, €1.70 per 100 k GB-s provisioned). Pricing page values used.
- **GCP:** rates vary by region. Instance-based billing uses different numbers (240 k vCPU-s and 450 k GiB-s free; $0.000018 per vCPU-s, $0.000002 per GiB-s). 1st-gen Cloud Functions used a different model (GB-s + GHz-s).
- **Rust on Lambda** often bills 10 to 20 ms, not 100 ms. That makes compute cheaper still but does not touch the request, gateway, egress and log lines.

### The stack around a function (AWS, per 1 M requests)

A public HTTP endpoint is more than the function:

| Item | Cost |
|---|---|
| Lambda requests + compute | ~$0.41 |
| API Gateway HTTP API | $1.00 (REST API $3.50; Lambda function URL $0, but no custom domain, throttling or WAF) |
| Egress, 10 KB response × 1 M ≈ 10 GB × ~$0.09/GB | ~$0.90 |
| CloudWatch Logs, 0.5 KB × 1 M ≈ 0.5 GB × ~$0.50/GB | ~$0.25 |
| **Sum** | **~$2.56** |

Monthly view, no free tier:

| Requests / month | Lambda only | Lambda + HTTP API + egress + logs | Shared hosting + Marty |
|---|---|---|---|
| 100 k | ~$0.04 | ~$0.26 | ~2 to 10 € flat |
| 1 M | ~$0.41 | ~$2.60 | same |
| 10 M | ~$4 | ~$26 | same |
| 50 M | ~$21 | ~$128 | same, or "please upgrade" |
| **Ceiling** | none | none | **the plan price** |

**No ceiling in the cloud:** a bug, a retry storm or an attacker (denial of wallet) scales the invoice, not the plan. Shared hosting includes domain, TLS, mail, webspace and MySQL in the flat fee.

### Requests per minute (free tiers in perspective)

Month = 30 days = 43,200 minutes, perfectly even load.

| Monthly volume | per minute | per second |
|---|---|---|
| 200 k (Azure Flex effective free) | ~4.6 | ~0.08 |
| 1 M (Lambda, Scaleway free) | ~23 | ~0.4 |
| 2 M (Cloud Run free) | ~46 | ~0.8 |
| ~3 M (Cloudflare free, 100 k/day) | ~69 | ~1.2 |

Real traffic is bursty (daytime peaks, quiet nights). A small company's webhook averaging 46 requests per minute around the clock is already a lot; **most small endpoints stay deep inside the free tier**. Load-testing a shared host at this rate for a month would be a fair-use issue; the repo's outside benches deliberately stay low-volume.

### What a fixed budget buys

Example budget: **€2.66 / month**, roughly the cheapest shared hosting plans (netcup 1000 class). Function only, 128 MB, 100 ms, free tier included, EUR ≈ USD 1:1 (since EUR > USD the dollar providers are actually slightly better than shown).

| Provider | Price per 1 M | Paid with 2.66 | + free tier | **Total / month** | ≈ per minute |
|---|---|---|---|---|---|
| Scaleway | ~€0.21 | ~12.7 M | 1 M | **~13.7 M** | ~320 |
| AWS Lambda | ~$0.41 | ~6.5 M | 1 M | **~7.5 M** | ~170 |
| GCP Cloud Run functions | ~$0.60 | ~4.4 M | 2 M | **~6.4 M** | ~150 |
| Cloudflare Workers | $5 minimum | not possible (2.66 < 5) | ~3 M | **~3 M** (10 M for $5) | ~70 |
| Azure Flex | ~$13.40 | ~0.2 M | ~0.2 M | **~0.4 M** | ~9 |

With the full AWS stack (~$2.56 per M incl. HTTP API, egress, logs) the same budget buys about 1 M paid + 1 M free ≈ **2 M** requests (~46/min). With a free function URL instead of the gateway it stays closer to 7.5 M.

**Interpretation:** for the function alone, the flat plan only wins above roughly 7 to 14 M requests per month at even load; with the stack around it, from about 2 M. A typical small webhook with a few thousand calls is cheaper in the cloud (free). Adding a relational database (RDS ~$14/month) breaks the budget before the first request.

## Setup C: data stores

Function compute is only half the story. Serverless runtimes have no durable local disk, so state moves to a managed store.

| Need (Marty twin) | Marty on shared hosting | AWS | Azure | Cloudflare | Scaleway | GCP |
|---|---|---|---|---|---|---|
| Few rows ≈ [`05_sqlite`](../examples/05_sqlite/) | SQLite file next to the binary | DynamoDB ([`02_dynamodb`](../alternatives/aws-lambda/02_dynamodb/)) | Table Storage / Cosmos Table | D1 (SQLite-shaped) or KV | Object Storage / Redis / Serverless SQL | Firestore |
| Relational ≈ [`07_mysql`](../examples/07_mysql/) | hoster MySQL, **included in plan** | RDS MySQL ([`03_mysql`](../alternatives/aws-lambda/03_mysql/), `with_rds=1`) | MySQL Flexible Server | no MySQL; Hyperdrive to external DB | Managed MySQL/PostgreSQL, Serverless SQL | Cloud SQL |
| Files | webspace | S3 | Blob Storage | R2 | Object Storage | GCS |
| Local disk | yes | no / ephemeral | no / ephemeral | no | no | no / ephemeral |

**Numbers**

| Store | Billing shape | Ballpark |
|---|---|---|
| **RDS `db.t4g.micro`** | Always on as long as it exists | ~**$14/month** on demand in eu-central-1 (Frankfurt), ~$0.019/h; ~$12 to 15/month in eu-west-1; US baseline ~$0.016/h. Storage and backups extra. (Aggregator bytebase.com, July 2026; may be the generic/PostgreSQL row, MySQL should be similar.) |
| **Azure MySQL Flexible Server** | Always on | Same class: monthly floor |
| **Cloudflare D1** | Rows read/written + storage | Free: 5 M rows read/day, 100 k rows written/day, 5 GB; paid storage ~$0.75/GB-month |
| **Cloudflare KV** | Ops + storage | Free: 100 k reads/day, 1 k writes/day, 1 GB |
| **DynamoDB on demand** | Per request + GB stored; 25 GB free storage | ≈ $0 for demo traffic. US about $0.625 per M writes, $0.125 per M reads; Frankfurt post-Nov-2024 price **not verified**, check before quoting |

**Message:** serverless KV keeps money near zero but puts a **vendor data API into the code** (the SQLite file does not move with you). A real relational DB brings back a **monthly floor that alone exceeds the whole shared hosting plan**.

Per-provider stores: [`../alternatives/README.md`](../alternatives/README.md) (section *storing data*) and each provider README.

## Lock-in and setup

**Lock-in dimensions:** vendor entry point (event/handler signature), vendor tooling (`cargo lambda`, `az` + `func`, Wrangler, `scw`), vendor front door (function URL / API Gateway, `*.azurewebsites.net`, `*.workers.dev`), vendor lifecycle.

**Lifecycle example:** Azure Functions on Linux in the **Consumption plan retires on Sept 30, 2028**. It gets no new features or language versions, was removed from the portal, VS and VS Code (CLI/IaC still work), and there is **no in-place migration** to Flex Consumption: create a new app and redeploy (`az functionapp flex-migration` helps). The repo's [`alternatives/azure-functions/01_basic`](../alternatives/azure-functions/01_basic/) runs on exactly this plan. Runtime v3 on Linux Consumption already stopped on Sept 30, 2026. Details: [`azure-functions/README.md`](../alternatives/azure-functions/README.md).

**Setup friction (from this repo):**

| Path | Friction |
|---|---|
| AWS | Smoothest. cargo-lambda did not read `aws login` sessions, so the justfile exports keys |
| Azure | Wrong tenant on `az login`; `Microsoft.Storage`/`Microsoft.Web` not registered; `westeurope` not accepting new Free Trial customers (`northeurope` worked); broken zip via `mktemp` on macOS; `FUNCTIONS_WORKER_RUNTIME=Custom` (capital C) and `linuxFxVersion=DOTNET\|8.0` needed. See [`DEPLOY.md`](../alternatives/azure-functions/DEPLOY.md) |
| Cloudflare | Entry is always JS; Rust is a WASM library behind a shim |
| Scaleway | Source upload; path deps must be staged into the zip |
| Marty | `cargo zigbuild`, FTP/SFTP upload, `chmod 755` |

## Digital sovereignty

- German shared hosters with own servers in Germany (all-inkl states this for all plans; netcup, Hetzner, manitu are German) avoid the US CLOUD Act question.
- US providers remain subject to US law even in EU regions; AWS runs a separate European Sovereign Cloud (legal adequacy is per customer).
- EU FaaS exists: **Scaleway** (FR; regions fr-par, nl-ams, pl-waw) with a managed Rust runtime, cheapest per request in the tables above, but still its own entry point and CLI.
- OVH, STACKIT, IONOS, Hetzner: no Lambda-like FaaS. Clever Cloud functions (WASM) were not GA when checked.
- Do not overclaim: sovereignty is about **jurisdiction and exit**. Threat boundaries for the same hosts are in [`SECURITY.md`](SECURITY.md).

Product situations (latency, flat bill, independence): [`DIRECTION.md` — Why ship on Marty](DIRECTION.md#why-ship-on-marty).

## How to read this

- **One stateless function, low or spiky traffic, no data:** take the cloud free tier. It is cheaper than any hosting plan.
- **Small app with data, on a host you already pay for:** Marty + shared hosting is a predictable flat bill, SQLite or MySQL included, and keeps the "upload a binary" DX.
- **Heavy CPU per request:** Marty on a host with fair CPU often wins on latency and cost vs functions billed for wall time.
- **Need scale-out / global:** clouds win; Marty is not trying to replace that.
- **Need a SQL server left running:** cloud relational usually loses the cost pitch vs shared-hosting MySQL, unless you use serverless SQL/KV and accept the vendor API.

This is a **product argument**, not a finance spreadsheet. Revisit numbers when publishing marketing pages.

## Sources (checked Oct 27/28, 2026)

- AWS Lambda pricing: <https://aws.amazon.com/lambda/pricing/>
- Azure Functions pricing (free grants): <https://azure.microsoft.com/en-us/pricing/details/functions/>
- Azure cost estimation: <https://learn.microsoft.com/en-us/azure/azure-functions/functions-consumption-costs>
- Azure Flex Consumption: <https://learn.microsoft.com/en-us/azure/azure-functions/flex-consumption-plan>
- Azure Linux Consumption retirement: <https://learn.microsoft.com/en-us/Azure/azure-functions/functions-scale>
- Cloud Run pricing: <https://cloud.google.com/run/pricing>
- Cloudflare Workers pricing: <https://developers.cloudflare.com/workers/platform/pricing/>
- Scaleway serverless pricing: <https://www.scaleway.com/en/pricing/serverless/> (403 to bots); FAQ: <https://www.scaleway.com/en/docs/serverless-functions/faq>
- RDS `db.t4g.micro`: <https://www.bytebase.com/dbcost/rds/instance/db.t4g.micro/>
- Hoster sources: [`HOSTERS.md`](HOSTERS.md#sources)
