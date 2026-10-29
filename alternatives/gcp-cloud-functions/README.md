# Google Cloud Functions (placeholder)

Counterpart to Marty CGI on Google’s serverless stack (Cloud Functions / Cloud Run functions).

## How it would work (findings so far)

| | |
|--|--|
| **Rust story** | No first-class “upload Rust source / custom `bootstrap` zip” like AWS Lambda or Azure custom handlers |
| **Practical path** | **Container** — typically Cloud Run (or Functions 2nd gen sitting on Cloud Run) |
| **vs this tree** | That makes GCP the odd one out next to AWS/Azure (native zip), Cloudflare (WASM), and Scaleway (managed Rust source) |

## Data storage (when filled in)

Same constraint: no CGI-local SQLite. Typical GCP peers:

| Store | Role vs Marty |
|-------|---------------|
| **Firestore** / Realtime Database | Document / sync’d state ≈ “few rows” |
| **Memorystore (Redis)** | KV / cache (instance floor) |
| **Cloud SQL** (MySQL/Postgres) | Relational ≈ [`07_mysql`](../../examples/07_mysql/) — always-on |
| **Cloud Storage** | Blobs |
| **Spanner** | Global relational (overkill for Marty demos; expensive floor) |

## Cost shape

| Piece | Shape |
|-------|--------|
| **Cloud Run functions** (request-based, tier 1) | Free per billing account: **2 M requests + 180 k vCPU-s + 360 k GiB-s / month**. Then $0.40 / M requests + $0.000024 / vCPU-s + $0.0000025 / GiB-s → **~$0.60 / M** at 128 MB, 100 ms (~0.083 vCPU). Container cold starts |
| **Instance-based billing** | 240 k vCPU-s + 450 k GiB-s free; $0.000018 / vCPU-s, $0.000002 / GiB-s |
| **1st-gen Cloud Functions** | Different model (GB-s + GHz-s); not relevant for a new Rust sample |
| **Firestore** | Ops + storage (free tier for tiny demos) |
| **Cloud SQL** | **Always-on** instance — monthly floor like RDS |

Rates vary by region; list prices checked Oct 2026 ([Cloud Run pricing](https://cloud.google.com/run/pricing)). Full comparison: [`docs/COST.md`](../../docs/COST.md).

Not started yet — see [`../aws-lambda/`](../aws-lambda/), [`../scaleway-functions/`](../scaleway-functions/), [`../../docs/COST.md`](../../docs/COST.md). Overview: [`../README.md`](../README.md).
