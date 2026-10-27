# `03_mysql` — cloud twin of Marty `examples/07_mysql`

Marty reference: [`examples/07_mysql`](../../../examples/07_mysql/) — CGI + MySQL/MariaDB on shared hosting (`localhost`, credentials in `marty_mysql.conf`), response like `07_mysql: Bob …`.

This sample is the same visit logic on **Lambda + MySQL**. MySQL-for-a-few-rows is often overkill; the cheap “little state” twin is [`02_dynamodb`](../02_dynamodb/) ↔ Marty [`examples/05_sqlite`](../../../examples/05_sqlite/).

| | Marty `07_mysql` | This sample |
|---|------------------|-------------|
| Engine | hoster MySQL/MariaDB | Amazon RDS MySQL (`db.t4g.micro` via `with_rds=1`) |
| Network | `localhost` in CGI | public RDS endpoint (demo) or your own engine |
| Cost | typically inside shared-hosting flat fee | **RDS monthly floor** + Lambda — see [`docs/COST.md`](../../../docs/COST.md) |

## What it returns

`03_mysql: Bob Bob …` (same shape as Marty).

## Credentials (env on the function)

| Env | Marty analogue |
|-----|----------------|
| `MYSQL_HOST` | `host` / `MARTY_MYSQL_HOST` |
| `MYSQL_USER` | `user` / `MARTY_MYSQL_USER` |
| `MYSQL_PASSWORD` | `password` / `MARTY_MYSQL_PASSWORD` |
| `MYSQL_DATABASE` | `database` / `MARTY_MYSQL_DATABASE` |
| `MYSQL_PORT` | `port` / `MARTY_MYSQL_PORT` (default `3306`) |

## Deploy

```bash
cd alternatives/aws-lambda
just login
just deploy with_rds=1    # Lambdas + DynamoDB + RDS MySQL + MYSQL_* wired
# … experiment …
just destroy              # deletes Lambdas, DynamoDB, and this RDS instance
```

Without RDS: `just deploy` only uploads the function; then set `MYSQL_*` yourself.

`with_rds=1` uses a public demo security group (3306 open). Creds land in gitignored `.rds-credentials`.
