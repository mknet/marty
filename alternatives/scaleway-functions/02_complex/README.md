# `02_complex` — Scaleway Functions twin of Marty `marty-sync`

Shared compute staged into the source zip (remote `cargo` cannot see path deps outside the archive).

Paths: `/primes/{limit}`, `/fibonacci/{n}`, `/matrix/{size}`.

## Build / deploy

```bash
cd alternatives/scaleway-functions
just deploy 02_complex
```

Function name defaults to `marty-alt-scw-02-complex`. `just package` vendors `benchmarks/complex/shared` as `./shared` in the zip.

Set `SCW_COMPLEX_URL=https://<domain>` from the deploy output into `alternatives/targets.env`.
