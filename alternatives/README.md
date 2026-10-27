# Alternatives to Marty

Side-by-side sketches of **cloud function** stacks that solve a similar problem as Marty CGI (request in → response out, no long-lived VPS process you manage) — but with a different ops and cost model. Product cost sketch: [`docs/COST.md`](../docs/COST.md).

These are **not** Marty features. Each tree is a small, self-contained sample you can build/deploy with that cloud’s tooling so the Marty pitch stays concrete.

| Path | Status | Notes |
|------|--------|--------|
| [`aws-lambda/`](aws-lambda/) | Started | AWS Lambda (Rust Event Function) |
| [`gcp-cloud-functions/`](gcp-cloud-functions/) | Placeholder | Google Cloud Functions / Cloud Run functions |
| [`azure-functions/`](azure-functions/) | Placeholder | Azure Functions |

Marty default for comparison: [`examples/01_basic`](../examples/01_basic/) + [`examples/02_routing`](../examples/02_routing/).
