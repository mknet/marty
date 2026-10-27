# `01_basic` — AWS Lambda twin of Marty `examples/01_basic`

Returns plain text `88 miles per hour!` from a Rust **Event Function** behind a **Lambda Function URL**.

| | Marty CGI | This sample |
|---|-----------|-------------|
| Entry | `main` → stdout CGI | `lambda_http::run` |
| Deploy | upload musl binary to `cgi-bin/` | `cargo lambda deploy` → `provided.al2023` |
| Cost | flat shared hosting | per request + GB·s — see [`docs/COST.md`](../../../docs/COST.md) |

## Build / local invoke

```bash
# once: cargo install cargo-lambda
cd alternatives/aws-lambda/01_basic
cargo lambda build --release --arm64
cargo lambda watch   # local emulator
# elsewhere: curl http://localhost:9000/
```

## Deploy (needs AWS credentials)

```bash
aws sts get-caller-identity   # must succeed
cargo lambda deploy alt-aws-lambda-01-basic \
  --enable-function-url \
  --arm64
```

Then `curl` the printed Function URL.

## Agent setup (from AWS)

See [`../AGENT-SETUP.md`](../AGENT-SETUP.md) — source: [aws-lambda-agent-setup.md](https://docs.aws.amazon.com/lambda/latest/dg/samples/aws-lambda-agent-setup.md).
