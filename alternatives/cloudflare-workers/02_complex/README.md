# `02_complex` — Cloudflare Workers twin of Marty `marty-sync`

Shared compute: [`marty-bench-shared`](../../../benchmarks/complex/shared/). Paths: `/primes/{limit}`, `/fibonacci/{n}`, `/matrix/{size}`.

**Honesty:** free Workers CPU limits may reject heavy `matrix/128`; do not change the algorithm — mark `n/a` in the compare table if the route fails.

## Build / deploy

```bash
cd alternatives/cloudflare-workers
just deploy 02_complex
```

Then set `CF_COMPLEX_URL=https://marty-alt-cf-02-complex.<subdomain>.workers.dev` in `alternatives/targets.env`.
