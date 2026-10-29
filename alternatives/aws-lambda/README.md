# AWS Lambda (alternative to Marty)

**Recommendation (aws-serverless skill):** Rust HTTP handlers map to **Lambda Event Functions · On-Demand** (non-Node.js → Event Functions; no steady-traffic signal → On-Demand). Front door for a minimal public URL: **Lambda Function URL** (or API Gateway if you need its features).

Agent setup used for this tree: [AWS Lambda agent setup](https://docs.aws.amazon.com/lambda/latest/dg/samples/aws-lambda-agent-setup.md) — local notes: [`AGENT-SETUP.md`](AGENT-SETUP.md).

## How it works (findings)

| | |
|--|--|
| **Artifact** | Native Linux binary as `bootstrap` in a zip (`provided.al2023`) |
| **Who builds** | You — typically [`cargo-lambda`](https://www.cargo-lambda.info/) (aarch64 by default here) |
| **Entry** | `lambda_http::run` / `service_fn` (not CGI stdout) |
| **vs Marty** | Same “ship a binary” idea; cold start + per-request billing instead of flat CGI hosting |
| **vs containers** | Zip/custom runtime — **no Docker image required** for these samples |
| **Local** | `cargo lambda watch` (emulator) |

**Credentials:** `aws login` short-lived sessions; the justfile exports keys because cargo-lambda’s SDK did not read login sessions directly when we set this up.

| Example | Marty twin | What it shows |
|---------|------------|---------------|
| [`01_basic/`](01_basic/) | [`examples/01_basic`](../../examples/01_basic/) | Fixed text over a Function URL |
| [`04_complex/`](04_complex/) | [`marty-sync`](../../benchmarks/complex/marty-sync/) | primes/fib/matrix JSON; fair compare vs hosting |
| [`02_dynamodb/`](02_dynamodb/) | [`examples/05_sqlite`](../../examples/05_sqlite/) | Durable “few rows” — DynamoDB |
| [`03_mysql/`](03_mysql/) | [`examples/07_mysql`](../../examples/07_mysql/) | SQL visit pattern via RDS MySQL (optional) |

## Data storage

Lambda has **no durable local disk** like CGI+SQLite. Typical AWS options from a function:

| Store | Role vs Marty | In this tree |
|-------|---------------|--------------|
| **DynamoDB** | KV / document — closest serverless stand-in for “few rows” | [`02_dynamodb`](02_dynamodb/) |
| **RDS MySQL** (or Aurora) | Relational ≈ shared-hosting MySQL / [`07_mysql`](../../examples/07_mysql/) | [`03_mysql`](03_mysql/) via `just deploy with_rds=1` |
| **S3** | Blobs / static assets | not sampled |
| **ElastiCache (Redis)** | Hot KV / cache | not sampled |

## Cost shape

Prices change; treat as order-of-magnitude (EU, late 2026). Compute-only sketch: [`docs/COST.md`](../../docs/COST.md). Overview: [`../README.md`](../README.md).

| Piece | Shape | Demo ballpark |
|-------|--------|---------------|
| **Lambda** | $0.20 / M requests + $0.0000166667 / GB·s; free tier 1 M requests + 400 k GB·s / month (account) | Visit-demo traffic **~$0**; beyond free ~**$0.41 / M** at 128 MB, 100 ms (Rust often bills 10 to 20 ms, cheaper still) |
| **Front door, egress, logs** | HTTP API $1.00 / M (REST $3.50; Function URL $0), egress ~$0.09 / GB, CloudWatch Logs ~$0.50 / GB | ~**$2.56 / M** for the full stack at 10 KB response, 0.5 KB log. Function URL avoids the gateway line |
| **DynamoDB** (on-demand / free tier capacity) | Per request + GB stored; free tier is generous for tiny tables | **~$0** for `02_dynamodb` demos |
| **RDS `db.t4g.micro` MySQL** | **Always-on** instance hours + storage | ~**$14/mo** on demand in `eu-central-1` (Frankfurt, ~$0.019/h), ~**$12–15/mo** in `eu-west-1`, **as long as it exists** — storage/backup extra. This is why `with_rds=1` is opt-in and `just destroy` matters |

**Pitch takeaway:** serverless KV (DynamoDB) preserves “pay almost nothing at low traffic”; classic RDS reintroduces a **monthly floor** that can exceed Marty shared hosting by itself.

## Prerequisites

- AWS CLI ≥ 2.32 (`aws login`)
- [Cargo Lambda](https://www.cargo-lambda.info/) (`just install-cargo-lambda` or Homebrew)
- Default VPC in the target region (for optional RDS)

## Just

```bash
just login
just deploy                 # DynamoDB + all three Lambdas
just deploy with_rds=1      # also db.t4g.micro MySQL (public) + wire MYSQL_*  ⚠️ costs until destroy
just destroy
just deploy-one 02_dynamodb
just watch 01_basic
```

`with_rds=1` creates a **public** `db.t4g.micro` MySQL (demo SG opens 3306 to `0.0.0.0/0`), writes creds to gitignored `.rds-credentials`, and sets them on `alt-aws-lambda-03-mysql`. Prefer `just destroy` soon after experimenting.
