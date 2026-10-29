# Marty — product direction

Decision source for scope and defaults. Deeper notes:

- CGI stacks & Docker maps → [`CGI-VARIANTS.md`](CGI-VARIANTS.md)
- Cost sketch (shared hosting vs cloud functions) → [`COST.md`](COST.md)
- EU shared hosters (prices, SSH, CGI status) → [`HOSTERS.md`](HOSTERS.md)
- Hosting measurements → root [README](../README.md#shared-hosting-all-inkl-style), [`benchmarks/complex/`](../benchmarks/complex/)

## Pitch

Rust for web **without** a VPS: build a static CGI binary, upload to shared hosting (`cgi-bin/`), maybe pair with a frontend such as Marko. If you can run your own long-lived server anyway, Marty is optional — use Axum/Tokio (or anything else) directly.

## Defaults

| Choice | Decision |
|--------|----------|
| Primary target | Shared hosting — Apache **event/worker + mod_cgid + suexec** ([variants](CGI-VARIANTS.md)) |
| Recommended stack | **`sync-cgi`** — `marty::Router`, sync handlers, smallest binary |
| Axum / `tower` | **Optional escape hatch** for existing Axum handlers via `serve_cgi` |
| Sync router scope | Thin CGI DX: methods, `{param}` / `{*catch_all}`, `nest`/`merge`, fallback, `SCRIPT_NAME` strip — **not** Axum extractor/middleware parity |
| `.htaccess` | Feature **`htaccess`**: alias via `[package.metadata.marty]`, binary = package name; `build.rs` → `{binary}.htaccess` (not per-`Router` pattern) |
| Apache quirks | Document & mirror in Docker; **do not** grow library hacks for prefork-only issues (e.g. pacing was removed) |

New examples/docs prefer sync (`examples/02_routing`). Optional Axum: `examples/06_axum`.

## Target hosters (CGI-friendly)

Marty cares about hosts that allow **customer CGI** (ideally a static Linux binary under `cgi-bin/` or `AddHandler`). Always verify with [`tools/hosting-probe`](../tools/hosting-probe/): "CGI scripts" in marketing ≠ "arbitrary ELF" on every plan. **Probe before claiming.**

Full table with prices (incl. VAT, Oct 2026), terms, SSH and sources: [`HOSTERS.md`](HOSTERS.md). Shortlist:

| Hoster | Price / month | Status |
|--------|---------------|--------|
| **all-inkl.com** | 4.95 to 24.95 € | **Verified** (Oct 2026): Rust musl CGI works; Go dies (arena OOM); event/cgid/suexec; MySQL via [`examples/07_mysql`](../examples/07_mysql/) (config file, no `SetEnv`) |
| **Hetzner Webhosting S** | 1.90 € | **Verified** (Oct 2026): Debian 13, cgid-class sockets, Rust musl + PHP; same outside pattern as all-inkl (sync ≈ axum; Marty beats PHP on heavy CPU); lower absolute latency in one head-to-head from the same client |
| **netcup Webhosting 1000** | 2.69 € (12 months) | SSH on all plans; CGI not documented: probe |
| **manitu Webhosting M** | 5.99 € | Documents "CGIs for your own applications": **best unverified candidate**, probe |
| **lima-city Mini** | 3.50 to 4.50 € | SSH; CGI not documented: probe |
| **IONOS**, **DomainFactory / Host Europe**, **dogado**, **Strato** | various | CGI historically / `AddHandler cgi-script` (IONOS): probe plan limits |
| **Uberspace** | pay what you want (ref. 6 / 9 / 12 €) | **Different model:** web backends run your own long-lived HTTP process, so Axum directly, no CGI needed |
| **Infomaniak** (CH) | n/a | **Negative example:** CGI module switched off on shared hosting; marketing vs reality drift |
| **DreamHost** / **nearlyfreespeech.net** (US) | | Long-standing CGI culture; useful outside the EU |

Not a partnership list: a **shortlist to validate**. Prefork-only lab numbers do not prove a hoster.

## Planned features

| Feature | Intent |
|---------|--------|
| Sync DX (cookies, sessions, …) | Prefer sync-sized deps over pulling full Tower stacks by default |
| More hosting mirrors | Only when a real host differs in a way Docker should capture ([CGI-VARIANTS](CGI-VARIANTS.md)) |

## Findings (Oct 2026, all-inkl + Hetzner S + local hosting mirror)

- Host CGI is **mod_cgid**-class (stdio **unix sockets**) + site/FTP user + cleaned env, not classic prefork+`mod_cgi` pipes — on **both** all-inkl (Ubuntu 24.04) and **Hetzner Webhosting S** (Debian 13).
- **Rust** (static musl) works on both; **PHP** works; **Go** CGI dies at init on all-inkl (`out of memory allocating heap arena map`) — not re-checked on Hetzner.
- Outside `just outside` (same client, same `marty-sync` / `marty-axum` / PHP binaries, N=40 C=2): **Hetzner P50 ~33 ms** vs **all-inkl P50 ~50–54 ms** on sync routes; relative pattern identical — sync ≈ axum; **matrix** → Marty clearly ahead of PHP. Full table: [`RESULTS-SNAPSHOTS.md` Run N](../benchmarks/complex/RESULTS-SNAPSHOTS.md#run-n--outside-compare-hetzner-s-vs-all-inkl-2026-10-28).
- On light routes, TLS/RTT dominates; Marty ≈ PHP. On heavy CPU, Marty wins clearly.
- **Sync vs Axum** on the same host/workloads: latency ≈ noise; Axum binary ~2× larger.
- Light-route capacity sketch (Hetzner only): P50 stays under ~300 ms up to ~**C=60** / ~220 req/s burst; higher concurrency mostly queues (C=100 → ~420 ms P50). Not a SLA — one client, fair-use load.
- Cost story vs Lambda/Functions/Cloud Functions: for **one stateless function the cloud free tier wins**; the flat plan wins once there is data (SQLite/MySQL included vs DB floor or vendor data API) or steady volume. See [`COST.md`](COST.md).
- **Perf vs clouds (Oct 2026, Run A):** same `marty-bench-shared` complex routes on AWS / Azure / Cloudflare / Scaleway + Hetzner / all-inkl (`alternatives/just outside`, N=40 C=2, ~738 GETs). Warm P50 top tier: **Hetzner S ≈ Cloudflare Workers** (~26–34 ms); mid: Azure / Scaleway (~40–53 ms); slower band: **all-inkl ≈ AWS** (~50–66 ms). Absolute ms are mostly front door + RTT (identical compute). Azure Consumption showed a large cold/P95 spike; CF matrix ran OK on this account. Tables: [`alternatives/RESULTS-SNAPSHOTS.md`](../alternatives/RESULTS-SNAPSHOTS.md).

## Implications for work

1. Optimize for uploadable size, cold start, and sync DX — not for long-lived async servers.
2. Keep `tower`/Axum maintained enough to reuse existing handlers; not the main API.
3. Prefer hosting mirror / outside checks for shared-host claims; keep prefork Docker as a **known variant**, not the truth.
4. Teach the CGI variant map in docs; implement hoster-specific library workarounds only when (1) needs them.
