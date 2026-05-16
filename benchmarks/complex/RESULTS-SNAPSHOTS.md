# Complex benchmark — result snapshots

Saved from terminal runs on the complex fair bench (Apache Docker, port 8081, 2000 requests per handler×route, concurrency 10).

Workloads: `primes/400000`, `fibonacci/42`, `matrix/128`.

**Current `just bench-fair` uses Run A (`hey`) only.** Run B was an experimental profiling harness (not comparable for req/s).

## Run A — `hey` load test (no `?profile=1`)

Standard fair bench (`hey` only). Use this table for **latency / throughput** comparisons between languages.

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

Same request count, but each request via shell + `curl` + `jq` with `?profile=1`. **Do not compare req/s to Run A** (client overhead). Marty/Go **Avg** values are suspect vs Run A.

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

**Startup avg** in Run B = mean in-process `startup_us + handler_setup_us` (before CPU work); excludes Apache fork/exec.
