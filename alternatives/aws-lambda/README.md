# AWS Lambda (alternative to Marty)

**Recommendation (aws-serverless skill):** Rust HTTP handlers map to **Lambda Event Functions · On-Demand** (non-Node.js → Event Functions; no steady-traffic signal → On-Demand). Front door for a minimal public URL: **Lambda Function URL** (or API Gateway if you need its features).

Agent setup used for this tree: [AWS Lambda agent setup](https://docs.aws.amazon.com/lambda/latest/dg/samples/aws-lambda-agent-setup.md).

| Example | Marty twin | What it shows |
|---------|------------|---------------|
| [`01_basic/`](01_basic/) | [`examples/01_basic`](../../examples/01_basic/) | Smallest Rust Lambda: fixed text body over a Function URL |

## Prerequisites

- AWS CLI ≥ 2.32 (`aws login` for short-lived credentials)
- [Cargo Lambda](https://www.cargo-lambda.info/) (`just install-cargo-lambda` or Homebrew)
- Optional: AWS serverless agent skill — `npx skills add https://github.com/aws/agent-toolkit-for-aws --skill aws-serverless --yes --global`

## Just recipes

From `alternatives/aws-lambda/`:

```bash
just login                    # browser sign-in + pin region (default eu-west-1)
just login region=us-east-1   # same Lambda list price, US East
just whoami                   # Account / Arn + region
just build                    # 01_basic → Lambda zip (arm64 / Graviton)
just deploy                   # create/update function + Function URL
just watch                    # local emulator on :9000
```

Lambda **compute/request** list prices match across major US/EU regions; Arm (`--arm64`) is cheaper than x86. Default region is `eu-west-1`.

`just deploy` exports short-lived keys from your `aws login` session into the environment — cargo-lambda’s SDK does not yet read login sessions directly (`credentials-login`).
