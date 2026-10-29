# `01_basic` — AWS Lambda twin of Marty `examples/01_basic`

Marty reference: [`examples/01_basic`](../../../examples/01_basic/) — smallest CGI binary, body `88 miles per hour!`.

This sample returns the same plain text from a Rust **Event Function** behind a **Lambda Function URL**.

| | Marty CGI (`01_basic`) | This sample |
|---|------------------------|-------------|
| Entry | `main` → stdout CGI | `lambda_http::run` |
| Deploy | upload musl binary to `cgi-bin/` | `cargo lambda deploy` → `provided.al2023` |
| Cost | flat shared hosting | per request + GB·s — see [`docs/COST.md`](../../../docs/COST.md) |

Next: storage → [`02_dynamodb`](../02_dynamodb/) (≈ Marty [`05_sqlite`](../../../examples/05_sqlite/)), SQL → [`03_mysql`](../03_mysql/) (≈ Marty [`07_mysql`](../../../examples/07_mysql/)).

## Build / deploy

From `alternatives/aws-lambda/`:

```bash
just login
just deploy              # all samples (incl. this one)
just deploy-one 01_basic # only this sample
just destroy             # tear down all managed resources
just watch 01_basic      # local emulator on :9000
```

Then `curl` the printed Function URL.

## Agent setup (from AWS)

See [`../AGENT-SETUP.md`](../AGENT-SETUP.md) — source: [aws-lambda-agent-setup.md](https://docs.aws.amazon.com/lambda/latest/dg/samples/aws-lambda-agent-setup.md).
