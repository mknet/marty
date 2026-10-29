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
| **netcup EiWoMiSau** | ~6.45 € | **Verified** (Oct 2026): Debian 12, nginx→Apache, **pipe**/mod_cgi-class (unlike Hetzner/all-inkl sockets), Rust musl + PHP; WCP “CGI-Unterstützung” required; cheaper 1000 still probe |
| **netcup Webhosting 1000** | 2.69 € (12 months) | SSH on all plans; CGI not documented: **probe** (do not assume EiWoMiSau parity) |
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

## Findings (Oct 2026, all-inkl + Hetzner S + netcup EiWoMiSau + local hosting mirror)

- **Two real CGI shapes on verified hosts:** all-inkl (Ubuntu 24.04) and **Hetzner Webhosting S** (Debian 13) are **mod_cgid**-class (stdio **unix sockets**) + site/FTP user + cleaned env. **netcup EiWoMiSau** (Debian 12) is nginx front + Apache CGI with stdio **pipes** (mod_cgi-class) — closer to the classic prefork lab stack, still fine for Marty without library hacks.
- **Rust** (static musl) works on all three; **PHP** works; **Go** CGI dies at init on all-inkl (`out of memory allocating heap arena map`) — not re-checked on Hetzner/netcup.
- Outside `just outside` (same client, same `marty-sync` / `marty-axum` / PHP binaries, N=40 C=2): **Hetzner P50 ~33 ms** vs **all-inkl P50 ~50–54 ms** vs **netcup P50 ~72–73 ms** on sync routes; relative pattern identical — sync ≈ axum; **matrix** → Marty ahead of PHP. Tables: [`RESULTS-SNAPSHOTS.md` Run N](../benchmarks/complex/RESULTS-SNAPSHOTS.md#run-n--outside-compare-hetzner-s-vs-all-inkl-2026-10-28), [Run O](../benchmarks/complex/RESULTS-SNAPSHOTS.md#run-o--outside-netcup-eiwomisau-2026-10-29).
- On light routes, TLS/RTT dominates; Marty ≈ PHP (on netcup PHP sometimes edges Marty on P50). On heavy CPU, Marty wins clearly.
- **Sync vs Axum** on the same host/workloads: latency ≈ noise; Axum binary ~2× larger.
- Light-route capacity (same client, `primes/10000`): steady **320/min × 10 min** fine on **Hetzner** (P50 ~31 ms) and **netcup** (P50 ~73 ms, all 200). Burst **C=60** sketched on Hetzner only (~220 req/s, P50 under ~300 ms). Grill **C=100 × ~2 min**: Hetzner ~233 req/s, queues, stays up; **netcup ~72 req/s**, ~8.4k× 200, then **client IP banned** ≥20 min (shared-host filter; not a Marty failure). Do not use aggressive grills as hoster marketing claims. [Run N / Run O](../benchmarks/complex/RESULTS-SNAPSHOTS.md).
- Cost story vs Lambda/Functions/Cloud Functions: for **one stateless function the cloud free tier wins**; the flat plan wins once there is data (SQLite/MySQL included vs DB floor or vendor data API) or steady volume. See [`COST.md`](COST.md).
- **Perf vs clouds (Oct 2026, Run A):** same `marty-bench-shared` complex routes on AWS / Azure / Cloudflare / Scaleway + Hetzner / all-inkl (`alternatives/just outside`, N=40 C=2, ~738 GETs). Warm P50 top tier: **Hetzner S ≈ Cloudflare Workers** (~26–34 ms); mid: Azure / Scaleway (~40–53 ms); slower band: **all-inkl ≈ AWS** (~50–66 ms). Absolute ms are mostly front door + RTT (identical compute). Azure Consumption showed a large cold/P95 spike; CF matrix ran OK on this account. Tables: [`alternatives/RESULTS-SNAPSHOTS.md`](../alternatives/RESULTS-SNAPSHOTS.md).

## Implications for work

1. Optimize for uploadable size, cold start, and sync DX — not for long-lived async servers.
2. Keep `tower`/Axum maintained enough to reuse existing handlers; not the main API.
3. Prefer hosting mirror / outside checks for shared-host claims; keep prefork Docker as a **known variant**, not the truth.
4. Teach the CGI variant map in docs; implement hoster-specific library workarounds only when (1) needs them.
