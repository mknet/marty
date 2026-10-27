# `02_dynamodb` — cloud twin of Marty `examples/05_sqlite`

Marty reference: [`examples/05_sqlite`](../../../examples/05_sqlite/) — one GET creates storage if needed, inserts `Bob`/42, returns `05_sqlite: Bob …`.

This sample does the same with **DynamoDB On-Demand**. There is **no** durable local SQLite on Lambda (unlike CGI’s `./marty_05_sqlite.db`).

| | Marty `05_sqlite` | This sample |
|---|-------------------|-------------|
| Store | SQLite file next to CGI | DynamoDB table |
| Durability | host filesystem | managed NoSQL |
| Cost story | included in shared hosting flat fee | free tier / per-request — see [`docs/COST.md`](../../../docs/COST.md) |

Related: [`03_mysql`](../03_mysql/) ≈ Marty [`examples/07_mysql`](../../../examples/07_mysql/) (real SQL, higher cloud floor).

## What it returns

Plain text like `02_dynamodb: Bob Bob …` (all `name` attributes, sorted).

## Setup + deploy

From `alternatives/aws-lambda/` — usually just:

```bash
just login
just deploy          # includes this sample + DynamoDB table + IAM policy
just destroy         # tears it all down again
```

Single sample only: `just deploy-one 02_dynamodb` (table must exist; full `just deploy` is simpler).

Env on the function: `TABLE_NAME` (default `alt-aws-lambda-02-user`).
