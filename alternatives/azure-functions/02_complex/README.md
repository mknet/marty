# `02_complex` — Azure Functions twin of Marty `marty-sync` (complex CPU)

Marty reference: [`benchmarks/complex/marty-sync`](../../../benchmarks/complex/marty-sync/). Shared compute: [`marty-bench-shared`](../../../benchmarks/complex/shared/).

Paths (Azure `/api` prefix): `/api/primes/{limit}`, `/api/fibonacci/{n}`, `/api/matrix/{size}`.

Fair compare: set `AZURE_COMPLEX_URL=https://<app>.azurewebsites.net/api` in [`../../targets.env`](../../targets.example.env).

## Build / deploy

From `alternatives/azure-functions/`:

```bash
just login tenant=<your-tenant>
just app=marty-alt-az-02-complex storage=martyaltaz02complex deploy 02_complex
```

Uses the same Linux Consumption + custom-handler zip path as [`01_basic`](../01_basic/). See [`../DEPLOY.md`](../DEPLOY.md).

Smoke:

```bash
curl -sS "https://marty-alt-az-02-complex.azurewebsites.net/api/primes/400000?salt=0"
```
