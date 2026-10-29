# Alternatives complex compare — result snapshots

Outside fair protocol: [`compare-outside.sh`](compare-outside.sh) / `just outside` from this directory.

**Workload:** `primes/400000`, `fibonacci/42`, `matrix/128` (`?salt=0`), N=40 C=2 — same as hosting Run N in [`../benchmarks/complex/RESULTS-SNAPSHOTS.md`](../benchmarks/complex/RESULTS-SNAPSHOTS.md).

**Compute:** only [`marty-bench-shared`](../benchmarks/complex/shared/) (identical algorithms + JSON).

**Deploy defaults:** AWS `eu-west-1` (128 MB / 30 s), Azure `northeurope`, Scaleway `fr-par`, Cloudflare edge.

## Run A — outside compare clouds + hosting (2026-10-28)

Same client laptop. Latency in ms. All routes returned 40× HTTP 200 unless noted.

| Handler | Avg | P50 | P95 | req/s | 200s | cold |
|---------|-----|-----|-----|-------|------|------|
| aws /primes | 61.4 | 55.6 | 139.5 | 31.91 | 40 | 144.3 |
| aws /fibonacci | 54.0 | 49.6 | 134.1 | 36.55 | 40 | 130.0 |
| aws /matrix | 70.1 | 65.9 | 150.8 | 28.23 | 40 | 136.5 |
| azure /primes | 108.1 | 45.1 | 715.3 | 17.55 | 40 | 767.5 |
| azure /fibonacci | 49.3 | 43.4 | 145.3 | 40.46 | 40 | 203.6 |
| azure /matrix | 51.8 | 45.8 | 137.8 | 38.15 | 40 | 184.7 |
| cf /primes | 37.4 | 33.9 | 92.9 | 52.77 | 40 | 94.8 |
| cf /fibonacci | 34.5 | 26.0 | 76.4 | 49.77 | 40 | 81.5 |
| cf /matrix | 37.1 | 30.3 | 83.8 | 51.09 | 40 | 88.7 |
| scw /primes | 52.8 | 45.3 | 121.3 | 37.80 | 40 | 120.8 |
| scw /fibonacci | 44.3 | 39.5 | 118.1 | 44.38 | 40 | 117.1 |
| scw /matrix | 63.2 | 52.9 | 127.7 | 31.58 | 40 | 122.8 |
| hetzner /primes | 34.6 | 31.6 | 86.6 | 57.13 | 40 | 90.2 |
| hetzner /fibonacci | 34.8 | 32.2 | 88.2 | 57.11 | 40 | 99.4 |
| hetzner /matrix | 36.3 | 33.1 | 91.2 | 54.68 | 40 | 89.1 |
| allinkl /primes | 62.4 | 54.0 | 137.8 | 31.87 | 40 | 314.8 |
| allinkl /fibonacci | 59.3 | 51.8 | 142.2 | 33.69 | 40 | 145.6 |
| allinkl /matrix | 58.3 | 52.5 | 143.9 | 33.80 | 40 | 145.2 |

### Head-to-head (warm P50, ms)

| Route | Hetzner | CF | Azure | Scaleway | all-inkl | AWS |
|-------|---------|-----|-------|----------|----------|-----|
| primes | **32** | 34 | 45 | 45 | 54 | 56 |
| fibonacci | 32 | **26** | 43 | 40 | 52 | 50 |
| matrix | 33 | **30** | 46 | 53 | 53 | 66 |

**Takeaways (one client, one session):**

- Relative compute is the same binary logic everywhere; absolute ms are mostly **front door + RTT + cold/warm**.
- **Hetzner S** and **Cloudflare Workers** are in the same ballpark on warm P50 from this laptop; CF fib/matrix slightly ahead.
- **AWS** (eu-west-1 Function URL, 128 MB) and **all-inkl** land in a similar warm band (~50–66 ms P50).
- **Azure** warm P50 is competitive after warm-up; **cold primes ~768 ms** and a fat P95 on the first warm primes run show Consumption cold-start / scale noise.
- Cloudflare **matrix succeeded** on this account (not free-CPU blocked in this run).
