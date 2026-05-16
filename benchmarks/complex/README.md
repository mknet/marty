# Complex CPU benchmark

Same language span as [`simple/`](../simple/) (Marty, Go, Python CGI, PHP mod_php), but **three GET routes** with CPU-heavy work inspired by common web/language benchmarks ([HsBenchMarkSuite](https://github.com/hsaito/HsBenchMarkSuite)-style primes, [fibonacci web benchmarks](https://github.com/startover/fibonacci-webapp-benchmark), dense **matrix multiply**):

| Route | Workload | Default parameter |
|-------|----------|-------------------|
| `GET /primes/{limit}` | Sieve of Eratosthenes — count primes below `limit` | `400000` → **33860** primes |
| `GET /fibonacci/{n}` | `repeats = 3000 + (salt % 7000)` × `fib(42)` (path `{n}` ignored) | `?salt=0` → **803742888000** |
| `GET /matrix/{size}` | Naive `size×size` matrix multiply + checksum | `size=128` |

### Anti-caching (no constant result per URL)

Each response includes a **`salt`** field and `Cache-Control: no-store`:

- **`?salt=0`** — deterministic workload (used in smoke tests).
- **Omitted `salt`** (load tests) — per-request salt from time; primes/matrix/fib **work size** shifts with salt (fib: thousands of `fib(42)` iterations). No stable JSON body to cache.

After `BENCH_REBUILD=1`, the bench script **recreates** the container (old image is not reused).

OPcache still caches PHP **bytecode** between requests; that is not HTTP/result caching.

CGI examples: `/cgi-bin/marty-complex/primes/400000`  
PHP (mod_php): `/php/complex.php/primes/400000`

All four implementations use the **same algorithms** (see `marty-complex/src/compute.rs`).

## Quick start

```bash
cd benchmarks/complex
just docker-smoke      # build + Apache on :8081
just bench-fair        # 2000 GETs per (language × route), hey summary table
```

Local CGI only (port **8080**, `_http-cgi-server`):

```bash
just smoke
```

Docker defaults to **8081** so it can run alongside `benchmarks/simple` on 8080.

If **port 8081 is already allocated**: `just docker-down` (stops compose + any other container on that port), then retry.

If handlers 404 or wrong stack: `just docker-down`, then `BENCH_REBUILD=1 just bench-fair-rebuild`.

Different host port: `BENCH_PORT=8082 BENCH_BASE_URL=http://127.0.0.1:8082 just bench-fair`

```bash
BENCH_REBUILD=1 just bench-fair-rebuild
BENCH_REQUESTS=200 just bench-fair
```

Fair compare: one Apache container, **2000 requests per (handler × route)** with [`hey`](https://github.com/rakyll/hey), summary table (Avg / Fast / Slow / req/s). After each run, **Startup avg** is the mean of per-request lines in `/var/log/bench-timing/requests.log` (`BENCH_TIMING=1`). Each line: `impl<TAB>route<TAB>pre_compute_us` (e.g. `marty-complex	primes	850`). See [`RESULTS-SNAPSHOTS.md`](RESULTS-SNAPSHOTS.md) for example runs.

From `benchmarks/`: `just complex-bench-fair`. From repo root: `just bench-complex-fair`.

## Layout

```
benchmarks/complex/
  marty-complex/     # Rust + serve_cgi + Axum
  go-complex/
  python-complex/
  php/complex.php
  docker/
  scripts/
  smoke.hurl
```
