# Cost sketch: Marty on shared hosting vs cloud functions

Illustrative comparison for the pitch “Rust web without a VPS.” **Prices change**; treat numbers as order-of-magnitude (EU-ish, late 2026). Always check current hoster/cloud tariffs.

## Setup A — Marty CGI on shared hosting

| Item | Assumption |
|------|------------|
| Host | Shared web hosting with CGI (e.g. all-inkl Premium class) |
| Price | ~**€8–12 / month** flat (traffic usually included under fair use) |
| App | One static Marty binary in `cgi-bin/` (+ static assets / Marko build) |
| Ops | FTP/SFTP upload, no container, no per-request bill from the hoster |

**Rough monthly cost:** **≈ €10** whether you get 10 k or a few million light hits — until the hoster fair-use or CPU limits kick in. Scaling “up” means a bigger shared plan or a VPS, not a line item per million invocations.

## Setup B — Cloud functions (AWS / Azure / GCP)

Same idea: pay per invocation + GB·seconds, plus free tiers. Ballpark public list prices (region EU, rounded):

| Cloud | Requests | Compute (example) |
|-------|----------|-------------------|
| **AWS Lambda** | ~$0.20 / 1M requests | ~$0.0000167 / GB-s |
| **Azure Functions** (consumption) | similar order (~$0.20 / 1M) | ~$0.000016 / GB-s |
| **GCP Cloud Functions** (2nd gen / Cloud Run–style) | similar order | similar order |

**Worked example** (same assumptions for all three clouds):

- 128 MB memory, **100 ms** billed duration ⇒ 0.0125 GB-s per request  
- Compute ≈ 0.0125 × $0.0000167 ≈ **$0.00000021 / request**  
- Requests ≈ **$0.00000020 / request**  
- **≈ $0.41 / million requests** (compute + requests), before free tier, egress, API Gateway/HTTP front door, logs, etc.

| Monthly volume | Cloud functions (≈) | Shared hosting Marty (≈) |
|----------------|---------------------|---------------------------|
| 100 k | ~$0 (often inside free tier) | ~€10 |
| 1 M | ~$0.40 + front door | ~€10 |
| 10 M | ~$4 + front door | ~€10 |
| 50 M | ~$20 + front door / limits | ~€10 or “please upgrade” |

Front doors (API Gateway, Functions proxy, Cloud Run URL mapping), egress, and observability often **dominate** the pure function line items at scale. Shared hosting usually includes HTTPS and bandwidth in the flat fee.

## How to read this

- **Low / spiky traffic, zero ops:** cloud functions can be cheaper than €10/mo (free tier).  
- **Steady site / app on one domain:** Marty + shared hosting is a **predictable flat** bill and keeps “upload a binary” DX.  
- **Heavy CPU per request:** Marty on a host with fair CPU often wins on both latency and cost vs short-lived functions billed for wall time.  
- **Need scale-out / global:** clouds win; Marty is not trying to replace that.

This is a **product argument**, not a finance spreadsheet. Revisit numbers when publishing marketing pages.
