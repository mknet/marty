# Complex benchmark — result snapshots

Saved from terminal runs on the complex fair bench (Apache Docker, port 8081, 2000 requests per handler×route, concurrency 10).

Workloads: `primes/400000`, `fibonacci/42`, `matrix/128`.

**Use P50/P95 for comparisons**, not Avg alone — CGI runs often have long tails (Slow >> P95). Go’s **P50 can be much lower than Avg** (bimodal: many fast forks + some slow).

## Run M — Hosting-like Apache: mpm_event + mod_cgid + suexec (2026-10-19)

**The prefork results (Runs K/L) do not describe the target hosting.** The probes
in [`tools/hosting-probe/`](../../tools/hosting-probe/) were run once each on
all-inkl.com and reported: Ubuntu 24.04 (Apache 2.4.58), CGI `stdin`/`stdout`
are **unix sockets** (`mod_cgid`, hence a threaded MPM — not prefork), the CGI
runs as the site user with suexec's cleaned environment
(`PATH=/usr/local/bin:/usr/bin:/bin`, only CGI/1.1 + `HTTP_*` + `SSL_*` vars),
KeepAlive on, responses chunked, HTTP/2 negotiated over TLS. Server-side time
for an empty Perl or Rust CGI was ~40–60 ms either way (TLS + RTT + wrapper
dominate; the language does not).

`docker/Dockerfile.hosting` mirrors that: Ubuntu 24.04, `mpm_event`,
`mod_cgid`, `apache2-suexec-custom` (CGIs owned by and run as user `site`),
PHP via php-fpm, `KeepAlive On`. Run with `just bench-fair-hosting`
(separate compose project `marty-bench-complex-hosting`). suexec strips
`BENCH_TIMING`, so the Startup column is n/a in this mode.

`just bench-fair-hosting` — 2000 requests × 3 runs (median), C=10, all handlers,
`marty-complex` with `apache-cgi-pacing` (crate default):

| Handler          | Avg   | P50   | P95   | P99   | req/s   |
|------------------|-------|-------|-------|-------|---------|
| marty /primes    |   3.2 |   3.0 |   4.4 |   6.3 | 3068.72 |
| marty /fibonacci |   2.4 |   2.2 |   3.8 |   5.0 | 4048.99 |
| marty /matrix    |   5.0 |   4.7 |   7.0 |   9.6 | 1981.45 |
| go /primes       |   4.3 |   4.0 |   6.3 |   8.4 | 2300.41 |
| go /fibonacci    |   3.1 |   2.9 |   4.5 |   6.4 | 3179.67 |
| go /matrix       |   8.0 |   7.4 |  13.5 |  19.1 | 1220.01 |
| python /primes   |  90.3 |  83.2 | 136.2 | 190.9 |  110.02 |
| python /fibonacci|  39.8 |  35.2 |  64.4 | 118.6 |  248.71 |
| python /matrix   | 247.3 | 241.6 | 331.4 | 396.0 |   40.03 |
| php /primes      |  24.5 |  23.1 |  37.8 |  47.0 |  399.51 |
| php /fibonacci   |  11.4 |  10.9 |  18.7 |  24.9 |  842.19 |
| php /matrix      |  44.3 |  42.1 |  63.3 |  79.7 |  222.40 |

(A first run of the same command right after the image build showed a
~20 ms fixed penalty on every compiled handler — Go `/primes` at 345 req/s —
which did not reproduce in any later run; treat it as host noise.)

### A/B: `apache-cgi-pacing` on the hosting-like stack

Back-to-back, same session, `BENCH_LANGS="marty go"`, Go as the control
(2707 → 2763 req/s between the two builds, i.e. ~2 % drift).
`MARTY_CARGO_FLAGS="--no-default-features"` builds `marty-complex` without the
feature:

| Handler          | Pacing | Avg | P50 | P95  | P99  | req/s   |
|------------------|--------|-----|-----|------|------|---------|
| marty /primes    | off    | 2.6 | 2.5 |  3.3 |  4.9 | 3877.71 |
| marty /primes    | on     | 3.3 | 3.0 |  4.4 |  6.4 | 3026.92 |
| go /primes       | -      | 3.6 | 3.4 |  5.0 |  6.1 | 2762.61 |
| marty /fibonacci | off    | 1.8 | 1.7 |  2.3 |  3.3 | 5544.36 |
| marty /fibonacci | on     | 2.1 | 1.9 |  3.3 |  4.5 | 4782.64 |
| go /fibonacci    | -      | 2.6 | 2.4 |  3.5 |  4.6 | 3801.20 |
| marty /matrix    | off    | 4.2 | 4.0 |  5.6 |  7.5 | 2359.96 |
| marty /matrix    | on     | 4.4 | 4.2 |  6.0 |  8.1 | 2209.47 |
| go /matrix       | -      | 6.5 | 6.0 | 10.3 | 14.7 | 1502.99 |

**Conclusion:**

- On the hosting-like stack the prefork picture inverts: **Marty beats Go on
  every route**, by 40–60 % in throughput without pacing, with lower P50 and
  P99. The single-`write()` response is simply the cheapest thing a CGI can do
  when the reader is the cgid daemon on a unix socket instead of a prefork
  worker on a pipe.
- **`apache-cgi-pacing` costs 6–22 % here** (most on the shortest responses)
  and worsens P99. The feature is a prefork/`mod_cgi` workaround; do **not**
  enable it for all-inkl-like deployments. The library default (feature off,
  one write) is the right one for this hosting.
- Both stacks together explain the earlier confusion: Go was never "faster at
  CGI", it was accidentally well-paced for prefork. Absolute numbers on the
  event/cgid stack are 2–3× higher for every compiled handler than on prefork.

## Run L — Marty "go-like" output (split header/body with micro-sleep) (2026-05-27)

**Confirms the Apache prefork synchronization hypothesis.** With Marty's normal
single-`write()` response, Apache `mod_cgi` reads the entire CGI document in one
`read()` and delivers it as a single `Content-Length` segment. The library now
exposes `CgiResponse::write_response_to_stdout_paced(delay)`, which splits
the response into header- and body-phase writes separated by `sleep_us`
microseconds, so that Apache observes two distinct reads from the CGI pipe —
mirroring what Go's `bufio.Writer` emits naturally and (in our setup) causing
Apache to switch the HTTP response to `Transfer-Encoding: chunked`.

These measurements were originally captured with a temporary runtime switch
that also spawned 3 short-lived warm-up threads to imitate Go's per-request
`clone()` pattern. That benchmark switch has since been removed: current
`marty-complex` enables the library feature `apache-cgi-pacing`, so the default
`CgiResponse::write_response_to_stdout()` path handles Apache pacing at compile
time and does not spawn diagnostic threads.

Historical diagnostic command used an environment-controlled sleep value; the
current equivalent is `BENCH_REBUILD=1 BENCH_TIMING=0 BENCH_LANGS="marty go" bash scripts/run-bench-fair.sh`
with `marty-complex` compiled against `marty` feature `apache-cgi-pacing`.

| Handler          | Mode          | Avg  | P50  | P95   | P99   | req/s   |
|------------------|---------------|------|------|-------|-------|---------|
| marty /primes    | baseline      | 22.4 |  8.1 |  60.7 |  89.6 |  443.02 |
| marty /primes    | go-like 50 µs | 29.2 |  7.8 |  72.6 | 126.1 |  341.56 |
| marty /primes    | go-like 200µs | 29.1 | 16.2 | 101.9 | 123.4 |  342.97 |
| go /primes       | -             |  5.3 |  5.0 |   8.1 |  10.6 | 1838.82 |
| marty /fibonacci | baseline      | 11.4 |  3.5 |  41.9 |  47.3 |  866.34 |
| marty /fibonacci | go-like 50 µs |  5.7 |  5.2 |  10.2 |  17.5 | 1731.53 |
| marty /fibonacci | go-like 200µs |  5.5 |  4.9 |  10.0 |  15.5 | 1810.31 |
| go /fibonacci    | -             |  4.4 |  4.1 |   7.2 |   9.4 | 2223.91 |
| marty /matrix    | baseline      | 15.3 |  9.8 |  32.8 | 123.5 |  650.78 |
| marty /matrix    | go-like 50 µs |  7.2 |  6.8 |  10.3 |  12.7 | 1374.43 |
| marty /matrix    | go-like 200µs |  7.0 |  6.7 |   9.8 |  12.5 | 1416.27 |
| go /matrix       | -             |  7.7 |  7.2 |  11.2 |  14.4 | 1274.40 |

**Conclusion:**

- **`/fibonacci` and `/matrix` (short, fairly deterministic compute) close the
  gap completely.** Marty's P95 drops from 41.9 → 10.0 ms (`fibonacci`) and
  32.8 → 9.8 ms (`matrix`); P99 collapses from 47.3 / 123.5 → 15.5 / 12.5 ms;
  throughput more than doubles on `/matrix` (650 → 1416 req/s) and on
  `/fibonacci` Marty (1810 req/s) is now within 18 % of Go (2223 req/s) and
  has the better P99.
- **`/primes` (longer, sieve-dominated compute, ~6-15 ms) gets *worse* with
  go-like.** Throughput drops to ~342 req/s and P95/P99 inflate. The 50 µs and
  200 µs variants are indistinguishable, ruling out the sleep itself; the
  cost is most likely the 3 warm-up threads + Apache's chunked path being
  unfavorable when each request already keeps a worker busy for many ms with
  variance. (A "go-like sleep only, no threads" mode would be needed to
  disentangle these.)
- The mechanism is **Apache prefork worker-pool synchronization, not CGI
  fork/exec cost and not the application code**. Marty's super-compact
  single-segment response causes many workers to free up in the same scheduler
  quantum, which then collide on accept-mutex / `access_log`. Pacing the
  response into two reads spreads them out — exactly what Go does
  unintentionally.

This validates the diagnosis from Run K's follow-ups: Marty isn't slow in
Apache — Marty is **too fast and too synchronous** for the Apache prefork pool
on short routes, and the fix lives in the I/O pacing of the CGI library, not
in the application.

## Run K — Marty single-write stdout (2026-05-26)

**Root cause identified.** `strace` of a single direct invocation revealed that Marty was emitting **5 separate `write()` syscalls** on stdout per CGI response (4 header lines + body), because Rust's `io::stdout()` is wrapped in a `LineWriter` that flushes on every `\n`. Go in comparison emitted only **2** (combined headers, then body) thanks to `bufio.Writer`.

Under Apache `mod_cgi` concurrency every extra `write()` on the child's stdout pipe becomes an extra wake-up / read in the Apache parent that drains it. This widened Marty's tail distribution dramatically (P99 routinely 40-60 ms in Runs G–J).

Fix in `marty-complex/src/cgi_out.rs`: build status + headers + body into one `Vec<u8>`, then write through the raw stdout fd (via `File::from_raw_fd(1)` wrapped in `ManuallyDrop`), bypassing `LineWriter`. Verified with strace that Marty now emits exactly **1 `write()`** syscall per response, less than Go.

Direct hey, warmed steady-state, 3000 req × 3 runs, concurrency 10 (runs 2-3 of each handler, run 1 excluded as cold):

| Handler          | P50 (ms) | P95 (ms) | P99 (ms) | Slowest (ms) |
|------------------|----------|----------|----------|--------------|
| marty /primes    | 4.6-4.7  | 8.1-8.3  | 10.4-11.0| 15-17        |
| go /primes       | 5.4-5.5  | 9.1-9.9  | 12.4-13.8| 21-42        |

Median-over-5-runs summary from `BENCH_REBUILD=1 just bench-fair-rust-go-stable` (5000 req × 5 runs, warm-up 200):

| Handler          | Avg (ms) | P50 (ms) | Fastest | Slowest (ms) | req/s   |
|------------------|----------|----------|---------|--------------|---------|
| marty /primes    | 4.60     | 4.30     | 3.90    | 6.30         | 1537.94 |
| marty /fibonacci | 3.70     | 3.40     | 3.20    | 4.00         | 2448.11 |
| marty /matrix    | 6.60     | 6.40     | 5.80    | 7.90         | 1246.14 |
| go /primes       | 5.50     | 5.60     | 4.80    | 6.20         | 1602.80 |
| go /fibonacci    | 4.60     | 4.40     | 4.00    | 5.20         | 1850.10 |
| go /matrix       | 8.30     | 8.30     | 7.20    | 11.50        | 869.33  |

**Conclusion:** A single `write()` per CGI response is what closed the gap — not chunked vs `Content-Length`, not header flush cadence, not JSON encoding, not routing overhead. Marty now beats Go at P50 by ~20-23% on all three routes, beats Go's worst-of-5-runs `Slowest` on `/fibonacci` and `/matrix`, and matches it on `/primes`. Throughput is comparable, with Marty +32% on `/fibonacci`, +43% on `/matrix`, and -4% on `/primes` (essentially tied).

## Run J — Marty Go-like output experiment (2026-05-26)

Stack: same as Run G, but `marty-complex` was run with `MARTY_COMPLEX_GO_LIKE_OUTPUT=1`. In this mode Marty writes CGI headers with CRLF, flushes after the headers, and appends a trailing newline to the JSON body to mimic Go's `net/http/cgi` output cadence more closely. Apache still delivered Marty responses with `Content-Length`; it did not switch Marty to chunked.

Command: `MARTY_COMPLEX_GO_LIKE_OUTPUT=1 BENCH_LANGS="marty go" BENCH_TIMING=0 BENCH_REQUESTS=2000 BENCH_WARMUP_REQUESTS=1000 BENCH_RUNS=1 just bench-fair`

| Handler          | Avg (ms) | P50 (ms) | P95 (ms) | P99 (ms) | req/s   |
|------------------|----------|----------|----------|----------|---------|
| marty /primes    | 19.50    | 6.40     | 52.90    | 60.20    | 510.33  |
| marty /fibonacci | 11.20    | 3.60     | 39.50    | 46.90    | 891.28  |
| marty /matrix    | 11.90    | 9.50     | 20.50    | 30.60    | 836.59  |
| go /primes       | 6.20     | 5.80     | 9.40     | 12.60    | 1605.54 |
| go /fibonacci    | 5.30     | 4.70     | 9.60     | 12.30    | 1874.90 |
| go /matrix       | 9.00     | 8.40     | 14.20    | 19.40    | 1084.91 |

**Conclusion:** Making Marty write more like Go does not close the tail gap. `/matrix` P95 improves versus Run G, but `/primes` gets worse and `/fibonacci` remains tail-heavy. The key difference is therefore not simply newline/CRLF/header-flush cadence.

## Run I — Go raw CGI response experiment (2026-05-26)

Stack: same as Run G, but `go-complex` was run with `GO_COMPLEX_RAW_CGI=1`. In this mode Go bypasses `net/http/cgi`'s `ResponseWriter` for the final response and writes a fixed CGI document directly to stdout from a hand-written `PATH_INFO` dispatcher. Apache still delivered Go responses with `Transfer-Encoding: chunked`; Marty responses still got `Content-Length`.

Command: `GO_COMPLEX_RAW_CGI=1 BENCH_LANGS="marty go" BENCH_TIMING=0 BENCH_REQUESTS=2000 BENCH_WARMUP_REQUESTS=1000 BENCH_RUNS=1 just bench-fair`

| Handler          | Avg (ms) | P50 (ms) | P95 (ms) | P99 (ms) | req/s   |
|------------------|----------|----------|----------|----------|---------|
| marty /primes    | 11.50    | 5.10     | 36.60    | 39.10    | 866.80  |
| marty /fibonacci | 10.30    | 4.30     | 36.80    | 39.20    | 971.80  |
| marty /matrix    | 12.30    | 7.80     | 27.20    | 65.30    | 715.10  |
| go /primes       | 6.50     | 5.90     | 11.10    | 14.90    | 1525.57 |
| go /fibonacci    | 5.90     | 5.30     | 10.90    | 15.20    | 1686.49 |
| go /matrix       | 8.60     | 8.10     | 12.80    | 17.80    | 1140.04 |

**Conclusion:** Forcing Go away from `net/http/cgi`'s response writer did not reproduce Marty's high Apache tails. Apache still framed Go as chunked, and Go's P95/P99 remained close to the normal Go run. The remaining difference is not explained by JSON encoding, chi routing, or Go's `net/http/cgi` response writer alone.

## Run H — Rust PGO + target-cpu native (2026-05-26)

Stack: same Apache CGI setup as Run G, but `marty-complex` was rebuilt with Rust PGO and `-Ctarget-cpu=native`. Training used the three Marty routes, then `llvm-profdata merge` produced `.pgo/merged.profdata`, and the final Docker image used `-Cprofile-use=/repo/benchmarks/complex/.pgo/merged.profdata`. Go was unchanged.

Command: `COMPOSE_PROJECT_NAME=marty-bench-complex-pgo BENCH_PORT=8082 BENCH_BASE_URL=http://127.0.0.1:8082 BENCH_LANGS="marty go" BENCH_TIMING=0 BENCH_REQUESTS=2000 BENCH_WARMUP_REQUESTS=1000 BENCH_RUNS=1 just bench-fair`

| Handler          | Avg (ms) | P50 (ms) | P95 (ms) | P99 (ms) | req/s   |
|------------------|----------|----------|----------|----------|---------|
| marty /primes    | 12.40    | 6.00     | 34.90    | 51.30    | 800.12  |
| marty /fibonacci | 10.60    | 3.60     | 38.30    | 45.30    | 935.91  |
| marty /matrix    | 10.80    | 8.40     | 23.80    | 29.00    | 919.69  |
| go /primes       | 5.80     | 5.40     | 9.40     | 12.40    | 1690.06 |
| go /fibonacci    | 5.00     | 4.60     | 8.10     | 13.30    | 1985.93 |
| go /matrix       | 8.90     | 8.30     | 13.80    | 19.10    | 1099.30 |

**Conclusion:** Rust PGO + `target-cpu=native` improves Marty throughput versus Run G (e.g. `/fibonacci` ~760 → ~936 req/s, `/matrix` ~758 → ~920 req/s) and lowers some tails, especially `/matrix` P95. It does not eliminate the CGI tail gap: Go still has much lower P95/P99 on all three routes.

## Run G — Warmed Rust-vs-Go pass after hot-path cleanup (2026-05-26)

Stack: same as Run F (static-musl `marty-complex`, Apache `KeepAlive Off`). Rust-only cleanup kept timing/profiling out of the normal path and reads CGI env vars as bytes. A single-write CGI response experiment was rejected because it brought the ~50-60 ms Marty distribution back in short concurrent runs; the final code keeps the previous multi-write CGI response shape.

Command: `BENCH_LANGS="marty go" BENCH_TIMING=0 BENCH_REQUESTS=2000 BENCH_WARMUP_REQUESTS=1000 BENCH_RUNS=1 just bench-fair`

| Handler          | Avg (ms) | P50 (ms) | P95 (ms) | P99 (ms) | req/s   |
|------------------|----------|----------|----------|----------|---------|
| marty /primes    | 16.50    | 5.80     | 42.60    | 45.70    | 600.26  |
| marty /fibonacci | 13.10    | 4.70     | 40.90    | 47.70    | 759.79  |
| marty /matrix    | 13.10    | 7.20     | 35.10    | 36.80    | 758.48  |
| go /primes       | 6.30     | 5.90     | 10.50    | 12.70    | 1570.29 |
| go /fibonacci    | 5.60     | 4.70     | 11.90    | 15.80    | 1765.22 |
| go /matrix       | 8.60     | 8.00     | 13.70    | 17.70    | 1150.69 |

**Conclusion:** with warm-up, Marty is aligned with Go on P50 for `/primes` and `/fibonacci`, and slightly faster on `/matrix` P50. The remaining difference is tail latency/throughput: Marty P95/P99 are still much higher under concurrent Apache CGI.

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

### marty-complex stack (library-parity pass)

- **Routing:** `marty::Router` / `CgiRequest` / `CgiResponse` via the `sync-cgi` feature.
- **JSON:** hand-written in `json.rs` (no `serde_json`).
- **CGI write:** `marty::CgiResponse` writes the complete CGI document through the library response path.
- **Docker build:** host-architecture musl target (`x86_64-unknown-linux-musl` or `aarch64-unknown-linux-musl`) via `cargo build -p marty-complex-bench --profile bench --target …`; copied from `target/<triple>/release/`.
- **Local build:** `cargo build -p marty-complex-bench --profile bench` (native artifact under `target/release/`).
- **Library:** the complex bench links `marty` with `default-features = false, features = ["sync-cgi"]` so results represent the lightweight library router rather than a hand-rolled benchmark-only CGI path.
- **Not compared here:** FastCGI / persistent worker (no fork per request).
