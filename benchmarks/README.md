# Benchmarks

| Suite | Description |
|-------|-------------|
| **[`simple/`](simple/)** | Minimal webhook (POST JSON + secret → JSON ack): Marty vs Go vs Python CGI vs PHP mod_php |
| **[`complex/`](complex/)** | Three CPU-heavy GET routes (primes / fibonacci / matrix) — same four languages |

## Quick start (complex)

```bash
cd benchmarks/complex
just bench-fair    # Apache :8081, 12 rows (4 langs × 3 routes)
```

See [`complex/README.md`](complex/README.md).

## Quick start (simple)

```bash
cd benchmarks/simple
just bench-fair
```

Or from the repo root: `just bench-webhook-fair`.

From `benchmarks/` (forwards to simple):

```bash
cd benchmarks
just simple-bench-fair
```

See [`simple/README.md`](simple/README.md) for details.
