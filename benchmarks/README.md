# Benchmarks

| Suite | Description |
|-------|-------------|
| **[`complex/`](complex/)** | CPU routes (primes / fibonacci / matrix): Marty sync + Axum, Go, Python, PHP — Docker (prefork + hosting-like) and shared-host upload/outside |
| **[`simple/`](simple/)** | Minimal webhook (POST JSON + secret → JSON ack): Marty vs Go vs Python CGI vs PHP |

Product direction: [`docs/DIRECTION.md`](../docs/DIRECTION.md). CGI stacks: [`docs/CGI-VARIANTS.md`](../docs/CGI-VARIANTS.md).

## Complex (Docker + outside)

```bash
cd benchmarks/complex
just bench-fair              # prefork lab image
just bench-fair-hosting      # event/cgid/suexec mirror
just linux                   # musl upload binaries → dist/
just outside https://…       # low-volume sync/axum/php on a real host
```

Host fingerprint (not a bench): [`tools/hosting-probe/`](../tools/hosting-probe/).

See [`complex/README.md`](complex/README.md).

## Simple

```bash
cd benchmarks/simple
just bench-fair
```

Or from the repo root: `just bench-webhook-fair`. See [`simple/README.md`](simple/README.md).
