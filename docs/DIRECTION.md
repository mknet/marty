# Marty — product direction

Decision source for scope and defaults. Deeper notes:

- CGI stacks & Docker maps → [`CGI-VARIANTS.md`](CGI-VARIANTS.md)
- Cost sketch (shared hosting vs cloud functions) → [`COST.md`](COST.md)
- Hosting measurements → root [README](../README.md#shared-hosting-all-inkl-style), [`benchmarks/complex/`](../benchmarks/complex/)

## Pitch

Rust for web **without** a VPS: build a static CGI binary, upload to shared hosting (`cgi-bin/`), maybe pair with a frontend such as Marko. If you can run your own long-lived server anyway, Marty is optional — use Axum/Tokio (or anything else) directly.

## Defaults

| Choice | Decision |
|--------|----------|
| Primary target | Shared hosting — Apache **event/worker + mod_cgid + suexec** ([variants](CGI-VARIANTS.md)) |
| Recommended stack | **`sync-cgi`** — `marty::Router`, sync handlers, smallest binary |
| Axum / `tower` | **Optional escape hatch** for existing Axum handlers via `serve_cgi` |
| Sync router scope | Thin CGI DX: methods, `{param}` / `{*catch_all}`, fallback, `SCRIPT_NAME` strip — **not** Axum extractor/middleware parity |
| Apache quirks | Document & mirror in Docker; **do not** grow library hacks for prefork-only issues (e.g. pacing was removed) |

New examples/docs prefer sync (`examples/02_routing`). Optional Axum: `examples/06_axum`.

## Target hosters (CGI-friendly)

Marty cares about hosts that allow **customer CGI** (ideally a static Linux binary under `cgi-bin/` or `AddHandler`). Always verify with [`tools/hosting-probe`](../tools/hosting-probe/) — “CGI scripts” in marketing ≠ “arbitrary ELF” on every plan.

| Hoster | Notes |
|--------|--------|
| **all-inkl.com** | **Verified** (Oct 2026): Rust musl CGI works; Go dies (arena OOM); event/cgid/suexec |
| **Hetzner Webhosting** | Documents CGI (Perl/Python/Ruby); binary CGI — probe before claiming |
| **IONOS** | `.htaccess` / `AddHandler cgi-script` documented; probe plan limits |
| **DomainFactory / Host Europe** | Classic DE shared hosting; CGI historically available — probe |
| **netcup** / **dogado** / similar DE/EU shared plans | Often CGI-capable; confirm binary + suexec |
| **DreamHost** / **nearlyfreespeech.net** (US) | Long-standing CGI culture; useful outside DE |

Not a partnership list — a **shortlist to validate**. Prefork-only lab numbers do not prove a hoster.

## Planned features

| Feature | Intent |
|---------|--------|
| **`.htaccess` generation** | From routes / mount points → uploadable rewrite rules (today: hand-maintained `htaccess.basic.in` / `apache-combined-vhost.htaccess.in`) |
| Sync DX (cookies, sessions, …) | Prefer sync-sized deps over pulling full Tower stacks by default |
| More hosting mirrors | Only when a real host differs in a way Docker should capture ([CGI-VARIANTS](CGI-VARIANTS.md)) |

## Findings (Oct 2026, all-inkl + local hosting mirror)

- Host CGI is **mod_cgid** (unix sockets) + suexec-style env, not classic prefork+`mod_cgi` pipes.
- **Rust** (static musl) works; **PHP** works; **Go** CGI dies at init (`out of memory allocating heap arena map`).
- On light routes, TLS/RTT (~40–60 ms) dominates; Marty ≈ PHP. On heavy CPU (e.g. `matrix/400`), Marty wins clearly.
- **Sync vs Axum** on the same host/workloads: latency ≈ noise; Axum binary ~2× larger.
- Cost story vs Lambda/Functions/Cloud Functions: flat shared hosting vs per-invocation — see [`COST.md`](COST.md).

## Implications for work

1. Optimize for uploadable size, cold start, and sync DX — not for long-lived async servers.
2. Keep `tower`/Axum maintained enough to reuse existing handlers; not the main API.
3. Prefer hosting mirror / outside checks for shared-host claims; keep prefork Docker as a **known variant**, not the truth.
4. Teach the CGI variant map in docs; implement hoster-specific library workarounds only when (1) needs them.
