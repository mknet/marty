# Marty — product direction

Decision source for scope and defaults. Deeper notes:

- CGI stacks & Docker maps → [`CGI-VARIANTS.md`](CGI-VARIANTS.md)
- Security boundaries (suexec, secrets, sessions) → [`SECURITY.md`](SECURITY.md)
- State with one process per request (sessions, SQLite, MySQL, migrations) → [`STATE.md`](STATE.md)
- Cost sketch (shared hosting vs cloud functions) → [`COST.md`](COST.md)
- EU shared hosters (prices, SSH, CGI status) → [`HOSTERS.md`](HOSTERS.md)
- Hosting measurements → root [README](../README.md#shared-hosting-all-inkl-style), [`benchmarks/complex/`](../benchmarks/complex/)

## Pitch

Rust for web **without** a VPS: build a static CGI binary, upload to shared hosting (`cgi-bin/`), maybe pair with a frontend such as Marko. If you can run your own long-lived server anyway, Marty is optional — use Axum/Tokio (or anything else) directly.

## Why ship on Marty

Marty is a **primary** path for request/response HTTP: one static Rust binary on shared hosting. It is not a consolation prize for people who “cannot do cloud.”

**Performance (Oct 2026, same client, same algorithms):** on Hetzner Webhosting S, warm P50 for the complex CPU routes sits with **Cloudflare Workers** at the top of the compare (~26–34 ms). all-inkl lands near AWS. Azure’s warm band is fine; its cold/P95 spike is the cloud tax Marty does not pay. Tables: [`alternatives/RESULTS-SNAPSHOTS.md`](../alternatives/RESULTS-SNAPSHOTS.md). On the host, Marty beats PHP on heavy CPU; sync ≈ Axum latency, smaller binary ([Findings](#findings-oct-2026-all-inkl--hetzner-s--netcup-eiwomisau--local-hosting-mirror)).

What you choose is **ops and economics**, not a slower runtime. Host status: [`HOSTERS.md`](HOSTERS.md). Money: [`COST.md`](COST.md). Trust boundaries: [`SECURITY.md`](SECURITY.md).

Any row below is enough. They often overlap.

| You want… | Marty ships… | Bounds (not demotions) |
|-----------|--------------|-------------------------|
| **Rust on the public web** | Library or service → musl ELF → `cgi-bin/` → `chmod 755`. Sync `Router` by default; Axum via `serve_cgi` if you already have handlers. | Long-lived Axum on a VPS/Uberspace you already run still wins on persistent connections. For ordinary request/response, CGI is the product. |
| **Cloud-class latency, hosting bill** | Verified floor **Hetzner S at 1.90 €/month**: process + SQLite/MySQL + domain, flat. Warm latency in the Workers band from the laptop compare above. | Global anycast and auto scale-out stay with the clouds. One idle stateless function can be free on a cloud tier — until you add a database. |
| **Independence from hyperscalers** | No Lambda/Azure/Workers/Scaleway entry point, CLI, or vendor URL. Artifact = ELF + `.htaccess`. Exit = FTP the same binary. German hosts keep the process in Germany. | The hoster is still an operator. Jurisdiction ≠ app security. Pick a plan that actually runs customer CGI (probe; Infomaniak does not). |
| **A host you already trust** | Turn existing webspace into a Rust backend next to the site that already pays for domain/FTP/MySQL. Same public origin, measured Rust speed. | Probe first. “CGI” ≠ “arbitrary ELF”. Same-account vhosts share a uid. |
| **A fast API next to a static frontend** | Marko/Astro/HTML for UI; Marty for forms, signup, JSON, signed downloads — at hosting latency that matches the Workers tier on the right plan. | CORS/auth are yours. Fair-use filters exist; do not grill production accounts. |
| **CPU work without a ms meter** | Parsers, hashing, primes-class work, small image/PDF. Process gone when done; wall time is not a line item. Marty leads PHP on the same host for heavy routes. | Host CPU quotas still apply. Light GETs are mostly TLS/RTT everywhere. |
| **PHP for content, Rust for the hot path** | Keep the CMS/shop; put the memory-safe or math-heavy endpoint in `cgi-bin/`. One domain, optional shared MySQL, Rust where it counts. | Two deployables. Shared uid: treat the account as one trust domain ([`SECURITY.md`](SECURITY.md)). |
| **A deliverable the customer can own** | Agency/Verein/client gets a binary on hosting they already understand — cloud-tier warm latency, no AWS org transfer, no surprise function bill. | Probe their plan. Ops model is panel + FTP/SFTP. |
| **A real app without a cloud résumé** | Leaderboard, RSVP, license check, webhook sink, small SaaS-shaped tool. SQLite/MySQL included; signed cookie sessions across process-per-request. Idle cost is the flat plan. | No WebSockets / cross-request in-memory queues. Signature checks and idempotency are app code. |
| **A fair baseline before (or instead of) cloud** | Same workloads run as twins under [`alternatives/`](../alternatives/). Hosting often **ties or beats** warm cloud P50; you move to a vendor when you need *their* scale or edge map — not because CGI was slow. | Entry points differ. Growing into Lambda is a port. Growing into Axum on a VPS reuses the handlers you already have. |
| **Teaching request → process → response** | One ELF, FTP, browser. Students measure real outside latency without IAM theatre — and see that “old” CGI on a modern host is not nostalgia. | Use Docker mirrors when the room has no host. Probe before claiming a tariff. |

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
