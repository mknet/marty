# Complex benchmark — result snapshots

Saved from terminal runs on the complex fair bench (Apache Docker, port 8081, 2000 requests per handler×route, concurrency 10).

Workloads: `primes/400000`, `fibonacci/42`, `matrix/128`.

**Use P50/P95 for comparisons**, not Avg alone — CGI runs often have long tails (Slow >> P95). Go’s **P50 can be much lower than Avg** (bimodal: many fast forks + some slow).

## Run F — Rust-only sieve optimization (2026-05-25)

Stack: same as Run E (static-musl `marty-complex`, Apache `KeepAlive Off`). Only Rust's `prime_count` implementation changed: the sieve now uses an explicit `Vec<u8>` composite array and a direct `while` marking loop. Go/Python/PHP were not changed.

Command: `BENCH_REQUESTS=2000 BENCH_TIMING=0 just bench-fair-throughput`

| Handler           | Avg (ms) | P50 (ms) | P95 (ms) | P99 (ms) | req/s    |
|-------------------|----------|----------|----------|----------|----------|
| marty /primes     | 27.70    | 8.90     | 76.70    | 124.00   | 359.86   |
| marty /fibonacci  | 5.80     | 3.50     | 23.00    | 25.60    | 1718.73  |
| marty /matrix     | 6.70     | 6.20     | 10.70    | 14.40    | 1478.60  |
| go /primes        | 4.80     | 4.60     | 7.00     | 10.00    | 2035.11  |
| go /fibonacci     | 3.90     | 3.60     | 5.80     | 8.30     | 2525.52  |
| go /matrix        | 7.80     | 7.20     | 11.50    | 16.00    | 1263.97  |
| python /primes    | 76.80    | 75.00    | 95.30    | 118.20   | 128.34   |
| python /fibonacci | 30.50    | 29.50    | 41.80    | 51.80    | 322.59   |
| python /matrix    | 222.90   | 218.90   | 289.40   | 351.40   | 44.35    |
| php /primes       | 26.10    | 24.60    | 38.30    | 51.70    | 375.96   |
| php /fibonacci    | 12.00    | 11.20    | 20.20    | 28.20    | 796.55   |
| php /matrix       | 41.80    | 40.50    | 57.20    | 67.30    | 234.92   |

### Rust-only primes profile

`PROFILE_ROUTE=primes/400000 bash scripts/profile-cgi.sh`

| Handler       | `compute_us` | `total_us` |
|---------------|--------------|------------|
| marty-complex | ~718         | ~768       |
| go-complex    | ~770         | ~881       |

**Conclusion:** The previous `/primes` gap was largely a Rust implementation detail, not Marty/CGI. Using `Vec<u8>` plus a direct marking loop makes Rust's in-process sieve slightly faster than Go in this single-request profile. Under `hey`, Go still has lower `/primes` P50, but Marty improved from **12.8 ms → 8.9 ms** while preserving the CGI setup and without changing Go/Python/PHP.

Follow-up experiments were rejected:

- `mimalloc` + explicit `std::process::exit(0)` after response write: built and smoke-tested, but worsened medians (`/fibonacci` P50 3.5 ms → 5.2 ms).
- `std::process::exit(0)` only: also worsened medians (`/primes` P50 8.9 ms → 13.5 ms).

Run F remains the best measured CGI setup so far.

## Run E — KeepAlive Off (2026-05-25)

Stack: same standalone static-musl `marty-complex` as Run D. Apache vhost (`docker/apache-bench.conf`) now has **`KeepAlive Off`**. The previous KeepAlive On config triggered Linux client-side TCP delayed-ACK (~40 ms) on every Marty response, because Marty's reply (Content-Length + ~150-byte body in one segment) fit in a single TCP packet and never triggered immediate-ACK heuristics. Go uses `Transfer-Encoding: chunked` (multiple segments) and avoided the delay. With KeepAlive Off, each request closes its connection and there is no delayed-ACK wait.

Command: `BENCH_REQUESTS=2000 BENCH_TIMING=0 just bench-fair-throughput`

| Handler           | Avg (ms) | P50 (ms) | P95 (ms) | P99 (ms) | req/s    |
|-------------------|----------|----------|----------|----------|----------|
| marty /primes     | 28.50    | 12.80    | 97.40    | 122.50   | 350.30   |
| marty /fibonacci  | 7.50     | 4.40     | 24.90    | 49.90    | 1277.38  |
| marty /matrix     | 6.90     | 6.30     | 11.70    | 20.70    | 1419.45  |
| go /primes        | 5.00     | 4.60     | 7.80     | 12.50    | 1951.42  |
| go /fibonacci     | 3.70     | 3.40     | 5.40     | 8.00     | 2691.63  |
| go /matrix        | 7.60     | 7.10     | 11.10    | 14.20    | 1298.94  |
| python /primes    | 75.10    | 72.80    | 97.70    | 140.70   | 131.45   |
| python /fibonacci | 29.80    | 29.00    | 40.40    | 46.70    | 328.10   |
| python /matrix    | 215.20   | 210.40   | 277.40   | 349.20   | 45.89    |
| php /primes       | 25.00    | 24.00    | 36.60    | 44.30    | 396.74   |
| php /fibonacci    | 11.70    | 11.30    | 19.10    | 23.70    | 839.02   |
| php /matrix       | 41.30    | 40.00    | 55.20    | 80.20    | 237.62   |

**Highlights:**

- `/matrix`: **Marty P50 6.3 ms beats Go 7.1 ms** (req/s 1419 vs 1299).
- `/fibonacci`: Marty P50 4.4 ms is within 1.3× of Go's 3.4 ms.
- `/primes`: Marty 12.8 ms vs Go 4.6 ms — the remaining gap is the Rust sieve doing ~3.5× the wall-clock work of the Go sieve in single-shot profiling. Same algorithm in `compute.rs`; the difference is bounds-check elimination / cache behavior, not framework.
- Both Marty and Go now massively exceed Python/PHP throughput.

**Root cause of the previous "Marty 2× slower" reading:** TCP delayed-ACK on `KeepAlive On`, not Marty handler code. Static linking (Run D) and earlier router/JSON rewrites (Run C) were preconditions that made the small Marty response fit into a single segment, but the actual latency was on the **client TCP stack waiting for a second segment** before ACKing.

Run B (`curl` + `?profile=1`) is an experimental profiling harness; **do not compare req/s to hey runs**. With `?salt=0`, fat LTO may constant-fold fib work in-process (`compute_us: 0` in profile JSON).

## Run D — Static musl marty-complex (2026-05-25)

Stack: same standalone `marty-complex` as Run C, but built for the host Linux musl target in Docker. Verification inside the Apache image:

- `marty-complex`: `statically linked`, `ldd`: `not a dynamic executable`, **~452 KiB**
- `go-complex`: `statically linked`, **~3.1 MiB**

Command: `BENCH_REQUESTS=2000 BENCH_TIMING=0 just bench-fair-throughput`

| Handler           | Avg (ms) | P50 (ms) | P95 (ms) | P99 (ms) | req/s    |
|-------------------|----------|----------|----------|----------|----------|
| marty /primes     | 57.40    | 61.10    | 76.10    | 86.50    | 148.94   |
| marty /fibonacci  | 48.80    | 56.50    | 71.00    | 79.10    | 195.68   |
| marty /matrix     | 51.90    | 55.00    | 74.30    | 81.00    | 186.10   |
| go /primes        | 25.00    | 4.80     | 52.60    | 62.20    | 362.92   |
| go /fibonacci     | 23.90    | 3.30     | 52.80    | 63.50    | 381.54   |
| go /matrix        | 20.80    | 5.30     | 53.60    | 56.40    | 446.13   |

### Static-link profile (`profile-cgi.sh`, one curl each)

| Route            | marty `compute_us` | go `compute_us` | marty `total_us` | go `total_us` |
|------------------|--------------------|-----------------|------------------|---------------|
| primes/400000    | ~2042              | ~1665           | ~2108            | ~1801         |
| fibonacci/42     | ~0*                | ~109            | ~28              | ~226          |
| matrix/128       | ~1857              | ~4453           | ~1887            | ~4588         |

\*Rust/fat LTO optimizes the deterministic fib loop extremely aggressively; hey load tests use dynamic salt.

**Conclusion:** Static linking removed the dynamic loader from `marty-complex`, but it **did not close the hey P50 gap**. Marty is now often faster than Go inside the CGI process, while Apache/hey still reports Marty P50 around **55–61 ms** and Go P50 around **3–5 ms**. The earlier dynamic-linking hypothesis was plausible but not sufficient; the remaining gap is in the Apache CGI scheduling/exec distribution rather than Marty handler code alone.

## Run C — Post Go-parity pass (dynamic glibc, 2026-05-25)

Stack: standalone `marty-complex` (PATH_INFO `match`, hand-written JSON, no `marty`/hyper/serde), sieve aligned with Go (`composite[]bool`), workspace profile `bench` (fat LTO). This run used the default glibc dynamic Rust artifact. Docker binaries: marty **~398 KiB**, go-complex **~3.1 MiB**.

Command: `BENCH_REQUESTS=2000 BENCH_TIMING=0 BENCH_REBUILD=1 just bench-fair-rebuild-throughput`

| Handler           | Avg (ms) | P50 (ms) | P95 (ms) | P99 (ms) | req/s    |
|-------------------|----------|----------|----------|----------|----------|
| marty /primes     | 57.40    | 59.90    | 75.40    | 117.70   | 152.34   |
| marty /fibonacci  | 50.40    | 59.30    | 76.50    | 97.00    | 188.12   |
| marty /matrix     | 50.90    | 53.10    | 73.60    | 108.30   | 189.32   |
| go /primes        | 26.50    | 8.80     | 52.50    | 57.60    | 322.06   |
| go /fibonacci     | 24.20    | 3.60     | 52.70    | 62.60    | 372.55   |
| go /matrix        | 20.70    | 5.70     | 53.70    | 57.20    | 437.90   |

### Profile baseline (`profile-cgi.sh`, `?salt=0&profile=1`, one curl each)

| Route            | marty `compute_us` | go `compute_us` | marty binary | go binary (Docker) |
|------------------|--------------------|-----------------|--------------|-------------------|
| primes/400000    | ~2787              | ~784            | 398 KiB      | 3.1 MiB           |
| fibonacci/42     | ~0*                | ~199            | —            | —                 |
| matrix/128       | ~1936              | ~2411           | —            | —                 |

\*Fib at `salt=0` may be compile-time folded under fat LTO; hey load tests use dynamic salt.

**Conclusion:** In-process setup is small; **primes compute was still ~3× Go** on this one-shot profile. Under `hey`, Marty **P50 ≈ 54–60 ms** vs Go **P50 ≈ 4–9 ms** (Go tail pulls Avg up). Run D later removed dynamic linking but did not materially change P50.

## Run A — `hey` load test (legacy table: Avg / Fast / Slow)

Standard fair bench before P50/P95 columns were added to `run-bench-fair.sh`.

| Handler           | Avg (ms) | Fast (ms) | Slow (ms) | req/s    |
|-------------------|----------|-----------|-----------|----------|
| marty /primes     | 47.60    | 2.40      | 2566.30   | 171.0390 |
| marty /fibonacci  | 38.00    | 1.40      | 88.80     | 246.7024 |
| marty /matrix     | 43.00    | 3.10      | 72.10     | 216.1702 |
| go /primes        | 25.10    | 1.80      | 69.30     | 355.1442 |
| go /fibonacci     | 23.10    | 0.90      | 66.80     | 372.8236 |
| go /matrix        | 20.30    | 3.40      | 79.10     | 451.4712 |
| python /primes    | 101.00   | 54.50     | 133.80    | 97.3721  |
| python /fibonacci | 64.90    | 17.30     | 214.90    | 152.4647 |
| python /matrix    | 229.60   | 140.50    | 504.20    | 43.1895  |
| php /primes       | 22.90    | 10.30     | 68.20     | 427.2581 |
| php /fibonacci    | 11.00    | 3.90      | 45.20     | 870.1278 |
| php /matrix       | 39.90    | 26.10     | 78.60     | 248.6282 |

## Run B — `curl` + `?profile=1` (profiling harness)

| Handler           | Avg (ms) | Startup avg | Fast (ms) | Slow (ms) | req/s    |
|-------------------|----------|-------------|-----------|-----------|----------|
| marty /primes     | 6.42     | 0.10        | 3.33      | 64.80     | 224.1427 |
| marty /fibonacci  | 3.75     | 0.10        | 1.94      | 15.75     | 248.9221 |
| marty /matrix     | 6.83     | 0.10        | 4.25      | 134.95    | 222.0316 |
| go /primes        | 4.64     | 0.04        | 2.91      | 15.95     | 250.3639 |
| go /fibonacci     | 3.95     | 0.04        | 1.90      | 14.41     | 246.5899 |
| go /matrix        | 6.83     | 0.04        | 4.79      | 13.74     | 233.0512 |
| python /primes    | 69.83    | 0.07        | 57.29     | 126.90    | 91.2327  |
| python /fibonacci | 28.59    | 0.06        | 19.87     | 283.67    | 150.9518 |
| python /matrix    | 218.57   | 0.08        | 152.43    | 581.04    | 39.4663  |
| php /primes       | 25.17    | 0.09        | 13.23     | 48.43     | 172.0759 |
| php /fibonacci    | 11.53    | 0.08        | 4.63      | 25.05     | 220.7286 |
| php /matrix       | 42.24    | 0.11        | 27.97     | 204.97    | 122.7301 |

**Startup avg** = mean in-process time before compute; excludes Apache fork/exec.

## Current script output format

`just bench-fair` prints (ms): **Avg, P50, P95, P99, Startup, Fast, Slow, req/s**.

### Throughput vs diagnose

| `BENCH_TIMING` | Startup column | Per-request log |
|----------------|----------------|-----------------|
| `1` (default)  | yes            | yes             |
| `0`            | n/a            | no              |

Compare throughput runs: `BENCH_TIMING=0 BENCH_REBUILD=1 just bench-fair-rebuild` (recreate container so Apache sees `BENCH_TIMING=0`).

### marty-complex stack (Go-parity pass)

- **Routing:** `PATH_INFO` first-segment `match` (no `Router`, no `dyn` dispatch).
- **JSON:** hand-written in `json.rs` (no `serde_json`).
- **CGI write:** `cgi_out.rs` (no `marty` crate in the bench binary).
- **Docker build:** host-architecture musl target (`x86_64-unknown-linux-musl` or `aarch64-unknown-linux-musl`) via `cargo build -p marty-complex-bench --profile bench --target …`; copied from `target/<triple>/release/`.
- **Local build:** `cargo build -p marty-complex-bench --profile bench` (native artifact under `target/release/`).
- **Library:** `marty` feature `sync-cgi` remains for embedders who want a slim sync router without hyper.
- **Not compared here:** FastCGI / persistent worker (no fork per request).
