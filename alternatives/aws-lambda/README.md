# AWS Lambda (alternative to Marty)

**Recommendation (aws-serverless skill):** Rust HTTP handlers map to **Lambda Event Functions · On-Demand** (non-Node.js → Event Functions; no steady-traffic signal → On-Demand). Front door for a minimal public URL: **Lambda Function URL** (or API Gateway if you need its features).

Agent setup used for this tree: [AWS Lambda agent setup](https://docs.aws.amazon.com/lambda/latest/dg/samples/aws-lambda-agent-setup.md).

| Example | Marty twin | What it shows |
|---------|------------|---------------|
| [`01_basic/`](01_basic/) | [`examples/01_basic`](../../examples/01_basic/) | Smallest Rust Lambda: fixed text body over a Function URL |
| [`02_dynamodb/`](02_dynamodb/) | [`examples/05_sqlite`](../../examples/05_sqlite/) | Durable “few rows” store — DynamoDB, because Lambda has no CGI-style local SQLite file |
| [`03_mysql/`](03_mysql/) | [`examples/07_mysql`](../../examples/07_mysql/) | Same SQL visit pattern via RDS MySQL (optional; paid floor) |

Cost sketch: [`docs/COST.md`](../../docs/COST.md).

## Prerequisites

- AWS CLI ≥ 2.32 (`aws login` for short-lived credentials)
- [Cargo Lambda](https://www.cargo-lambda.info/) (`just install-cargo-lambda` or Homebrew)
- Default VPC in the target region (for optional RDS)

## Just recipes

```bash
just login
just deploy                 # DynamoDB + all three Lambdas
just deploy with_rds=1      # also db.t4g.micro MySQL (public) + wire MYSQL_*  ⚠️ costs until destroy
just destroy                # Lambdas + DynamoDB + our RDS/SG if present
just deploy-one 02_dynamodb
just watch 01_basic
```

`with_rds=1` creates a **public** `db.t4g.micro` MySQL (demo SG opens 3306 to `0.0.0.0/0`), writes creds to gitignored `.rds-credentials`, and sets them on `alt-aws-lambda-03-mysql`. Prefer `just destroy` soon after experimenting.

`just deploy` exports short-lived keys from your `aws login` session — cargo-lambda’s SDK does not yet read login sessions directly (`credentials-login`).
