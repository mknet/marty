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

All four implementations use the **same algorithms** (see `marty-complex/src/compute.rs`). Compute code is intentionally shared across languages; micro-optimizing only Rust would break cross-language fairness.

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

## Fair bench output

One Apache container, **2000 requests per (handler × route)** with [`hey`](https://github.com/rakyll/hey), concurrency 10.

Summary columns (all ms except req/s):

| Column | Source |
|--------|--------|
| **Avg, P50, P95, P99, Fast, Slow, req/s** | hey |
| **Startup** | Container log when `BENCH_TIMING=1` (mean in-process µs before compute; **excludes** Apache fork/exec) |

### Measurement modes

Use **just recipes** (do not put `# comments` on the same line as `just …` — some setups pass `#` to just and you get `error: justfile does not contain recipe '#'`).

| Mode | Command | Notes |
|------|---------|--------|
| **Diagnose** (default) | `just bench-fair-diagnose` | Startup column + per-request log |
| **Throughput** | `just bench-fair-throughput` | `BENCH_TIMING=0`, no log I/O |
| **Stability** | `just bench-fair-stable` | 3 hey runs per target, median in table |
| Rebuild + diagnose | `just bench-fair-rebuild-diagnose` | Fresh Docker image |
| Rebuild + throughput | `just bench-fair-rebuild-throughput` | Fresh image, no timing log |

`just bench-fair` keeps env overrides (`BENCH_TIMING`, `BENCH_RUNS`). Recreate the container when changing `BENCH_TIMING` (compose `environment` + `PassEnv`).

Profile single request (not comparable to hey req/s): `?profile=1` on any route.

From `benchmarks/`: `just complex-bench-fair`. From repo root: `just bench-complex-fair`.

## Performance expectations (CGI)

Each request = **new process** (`fork` + `exec`). **marty-complex** is a standalone sync CGI binary (`PATH_INFO` dispatch, hand-written JSON, no hyper/serde in the bench crate). Go uses `net/http/cgi` + chi. PHP uses **mod_php** (no per-request fork) — not apples-to-apples with CGI for latency.

After the Go-parity pass (see **Run F** in [`RESULTS-SNAPSHOTS.md`](RESULTS-SNAPSHOTS.md)), Marty matches or beats Go in-process on `/primes` and beats Go on `/matrix`; under `hey`, Marty is close on `/fibonacci`, faster on `/matrix`, and still trails Go on `/primes` P50 because Go's request distribution has a very fast median. The remaining `/primes` gap is no longer framework-related.

The Rust-only primes optimization keeps the same Sieve of Eratosthenes algorithm but uses an explicit `Vec<u8>` composite array and a direct marking loop, which is a better hot-loop shape for Rust/LLVM. Go/Python/PHP were not changed for Run F.

**Important Apache config:** `docker/apache-bench.conf` sets `KeepAlive Off`. Marty's small responses (Content-Length + ~150 bytes) otherwise fit into a single TCP segment, which triggers Linux client-side TCP delayed-ACK (~40 ms) on every reply — Go's `Transfer-Encoding: chunked` happens to avoid this because it emits multiple segments. Disabling KeepAlive removes that artifact and produces an apples-to-apples bench.

**Build marty-complex locally (native):**

```bash
cargo build -p marty-complex-bench --profile bench
```

Workspace `[profile.bench]` uses fat LTO (`inherits = "release"`); artifact path is `target/release/marty-complex-bench`.

The Docker benchmark builds `marty-complex` for the host Linux musl target (`x86_64-unknown-linux-musl` or `aarch64-unknown-linux-musl`) and copies `target/<triple>/release/marty-complex-bench`. Verify with:

```bash
docker exec marty-bench-complex-bench-1 ldd /usr/lib/cgi-bin/marty-complex
```

Expected: `not a dynamic executable`.

The **`marty`** library exposes feature **`sync-cgi`** (sync [`Router`](../../marty/src/router.rs) without hyper) for other binaries; the complex bench does not link `marty` to keep the CGI executable small (~450 KiB static musl vs ~3 MiB Go in Docker).

## Layout

```
benchmarks/complex/
  marty-complex/     # Rust standalone sync CGI (PATH_INFO match)
  go-complex/
  python-complex/
  php/complex.php
  docker/
  scripts/
  smoke.hurl
```
