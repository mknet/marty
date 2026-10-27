# Alternatives to Marty

Side-by-side sketches of **cloud function** stacks that solve a similar problem as Marty CGI (request in → response out, no long-lived VPS process you manage) — but with a different ops and cost model. Product cost sketch: [`docs/COST.md`](../docs/COST.md).

These are **not** Marty features. Each tree is a small, self-contained sample you can build/deploy with that cloud’s tooling so the Marty pitch stays concrete. **Every cloud example names its Marty twin** under `examples/`.

Shared strings / visit formatting live in [`examples/demo_logic`](../examples/demo_logic/) (`marty-demo-logic`) so CGI and cloud twins exercise the **same** business logic.

| Path | Status | Notes |
|------|--------|--------|
| [`aws-lambda/`](aws-lambda/) | `01_basic`, `02_dynamodb`, `03_mysql` | Twins of Marty `01_basic`, `05_sqlite`, `07_mysql` |
| [`azure-functions/`](azure-functions/) | `01_basic` | Custom handler (no container); Marty `01_basic` |
| [`gcp-cloud-functions/`](gcp-cloud-functions/) | Placeholder | Likely Cloud Run + container for Rust |

| Marty example | Idea | AWS | Azure |
|---------------|------|-----|-------|
| [`examples/01_basic`](../examples/01_basic/) | smallest response | [`aws-lambda/01_basic`](aws-lambda/01_basic/) | [`azure-functions/01_basic`](azure-functions/01_basic/) |
| [`examples/05_sqlite`](../examples/05_sqlite/) | little durable state | [`aws-lambda/02_dynamodb`](aws-lambda/02_dynamodb/) | *(planned: Table Storage)* |
| [`examples/07_mysql`](../examples/07_mysql/) | relational MySQL | [`aws-lambda/03_mysql`](aws-lambda/03_mysql/) | *(planned: Flexible Server)* |
