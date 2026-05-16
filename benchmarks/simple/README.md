# Simple webhook benchmark

Compare a **minimal payment-provider-style webhook** (POST JSON + shared secret header → `200` JSON ack) across:

| Handler | Runtime | Apache integration |
|--------|---------|-------------------|
| **marty-webhook** | Rust (`marty::serve_cgi` + Axum) | CGI (`/cgi-bin/marty-webhook`) |
| **go-webhook** | Go (`net/http/cgi`) | CGI |
| **python-webhook** | Python 3 **stdlib only** (no Flask/WSGI) | CGI |
| **php/webhook.php** | PHP 8 | **mod_php** (`/php/webhook.php`, in-process) |

Python uses plain CGI on purpose: on typical shared hosting that is the realistic deployment; a WSGI app server (gunicorn/uvicorn) would be a different class of benchmark.

## Contract

- `POST` with `Content-Type: application/json`
- Header `X-Webhook-Secret: bench-secret`
- Body: `{"id":"evt_…","type":"payment.completed"}`
- Success: `200` and `{"received":true,"id":"evt_…"}`

Fixture: [`fixtures/webhook-post.json`](fixtures/webhook-post.json).

## Quick start (CGI only, local)

Uses the repo’s [`_http-cgi-server`](../../examples/_http-cgi-server) and `benchmarks/simple/cgi-bin/` (no PHP).

```bash
cd benchmarks/simple
just smoke          # build + Hurl (marty, go, python)
just run-server     # foreground test server on :8080
```

## Full stack (CGI + mod_php)

Apache in Docker builds Linux CGI binaries and serves PHP via mod_php:

```bash
cd benchmarks/simple
just docker-smoke   # build image, start, Hurl all four
just bench-docker   # load test with hey (install: go install github.com/rakyll/hey@latest)
just docker-down
```

Tune load: `BENCH_REQUESTS=10000 BENCH_CONCURRENCY=50 just bench-docker`.

## Load testing: hey vs drill vs k6

| Tool | Fit for this repo | Notes |
|------|-------------------|--------|
| **[hey](https://github.com/rakyll/hey)** | Quick local compare | Already wired (`just bench`). One binary, minimal config. |
| **[drill](https://github.com/fcsonline/drill)** | **Recommended** for repeatable YAML benchmarks | Rust, Apache-Ansible-style plans, `body.file`, `--stats`. Plans live under [`load/drill/`](load/drill/). Install: `cargo install drill`. |
| **[k6](https://k6.io)** | Optional, heavier | Worth it if you need **ramp-up stages**, SLO thresholds, Grafana/CI export, or JS scenarios. For four static POST endpoints on localhost it is usually more setup than benefit. |

**Important:** Run **one handler per run** (as both scripts do). A single drill plan with four sequential requests would measure a pipeline, not per-language throughput.

### Fair comparison (recommended)

One **Apache** container (CGI + **mod_php**), **2000 POSTs per handler**, summary table. The container is **not** restarted between handlers; rebuild only after code changes.

```bash
cd benchmarks/simple
just bench-fair              # reuse running container if already up
just bench-fair-rebuild      # docker compose build + up, then benchmark
# BENCH_REQUESTS=500 BENCH_CONCURRENCY=20 just bench-fair
```

From `benchmarks/`: `just bench-fair`. From repo root: `just bench-webhook-fair`.

### hey

```bash
cd benchmarks/simple
just bench              # _http-cgi-server + cgi-bin (CGI only)
just bench-docker       # Apache: CGI + mod_php
# BENCH_REQUESTS=10000 BENCH_CONCURRENCY=50 just bench-docker
```

Install: `go install github.com/rakyll/hey@latest`

### drill

```bash
cd benchmarks/simple
just bench-drill
just bench-drill-docker
# BENCH_ITERATIONS=10000 BENCH_CONCURRENCY=50 BENCH_RAMPUP=2 just bench-drill-docker
```

Install: `cargo install drill` (needs OpenSSL dev headers).

### k6 (optional)

If you later want k6, add a `load/k6/webhook.js` with four scenarios or four `k6 run` invocations — same isolation rule as above. We do not ship k6 by default to keep dependencies small.

## Layout

```
benchmarks/simple/
  marty-webhook/     # Rust workspace crate
  go-webhook/
  python-webhook/
  php/
  fixtures/
  docker/            # Apache + multi-stage build
  scripts/
  smoke.hurl         # all four (docker)
  smoke-cgi.hurl     # CGI three (local server)
```

Results depend on hardware, Apache MPM, and whether CGI forks per request; treat numbers as **relative** signals on your machine, not universal rankings.
