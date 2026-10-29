# CGI deployment variants (Apache-first)

CGI is just “one process per request.” In practice almost everyone meets it behind **Apache**. There are several common stacks; Marty aims to **know and mirror** them (Docker + probes), not to paper over every quirk with library workarounds.

Walk **most common for our target → historical / lab** below. Problems are called out so claims stay honest; Marty does **not** need a special fix for every one (e.g. prefork pacing was measured and dropped).

## 1. Shared hosting today — `mpm_event`/`worker` + `mod_cgid` + suexec

**What it is:** Threaded MPM; CGI stdin/stdout are **unix sockets** (`mod_cgid`). The binary runs as the **site/FTP user** through suexec with a **cleaned environment** (CGI/1.1 + `HTTP_*` / `SSL_*`; `.htaccess` `SetEnv` does **not** reach the CGI). Often KeepAlive, chunked CGI, HTTP/2 + TLS in front.

**Why it matters:** This is the **primary Marty target** (all-inkl– / Hetzner-S–class). Upload a static musl binary → `cgi-bin/` → `chmod 755`.

**Verified hosts (Oct 2026):** all-inkl (Ubuntu 24.04) and Hetzner Webhosting S (Debian 13) — both socket/cgid-class, site user, KeepAlive + HTTP/2. **netcup EiWoMiSau** is verified too but is **pipe**/mod_cgi-class (see §2). See [`HOSTERS.md`](HOSTERS.md) and outside Run N / Run O in [`RESULTS-SNAPSHOTS.md`](../benchmarks/complex/RESULTS-SNAPSHOTS.md).

**Local mirror:** [`benchmarks/complex/docker/Dockerfile.hosting`](../benchmarks/complex/docker/Dockerfile.hosting)  
`just bench-fair-hosting` / `just docker-smoke-hosting` in `benchmarks/complex/`.

**Outside:** [`tools/hosting-probe/`](../tools/hosting-probe/) (fingerprint); `just outside` in [`benchmarks/complex/`](../benchmarks/complex/) (low-volume sync/axum/php).

**Known issues (document, don’t over-engineer):**

| Issue | Note |
|-------|------|
| Go CGI OOM at arena init | Address-space limits on shared hosts — negative proof on all-inkl; keep in complex bench |
| No `SetEnv` into CGI | Configure via query/path/files, not Apache env toggles (confirmed all-inkl; assume same on suexec-style hosts) |
| TLS/RTT floor | Dominates light routes; absolute ms differ by client↔host path (Hetzner vs all-inkl outside from one laptop) |
| Fair-use / load | Keep outside benches low-volume |

## 2. Classic / pipe CGI — `mpm_prefork` + `mod_cgi` (lab + some real hosts)

**What it is:** CGI stdio are **pipes** (`mod_cgi`). Common in Docker PHP images and older docs. Easy to stand up. **Not** the primary all-inkl– / Hetzner-S shape — but **netcup EiWoMiSau** (Oct 2026 probe) matches this pipe fingerprint behind an nginx TLS front (Apache `SERVER_SOFTWARE`, site user, KeepAlive + HTTP/2). Still upload musl → `httpdocs/cgi-bin/` after enabling WCP **CGI-Unterstützung**; do not use Passenger Python/Node/Ruby for Marty.

**Local mirror:** [`benchmarks/complex/docker/Dockerfile`](../benchmarks/complex/docker/Dockerfile) (default fair bench).  
`just bench-fair` in `benchmarks/complex/`.

**Known issues (historical):**

| Issue | Note |
|-------|------|
| Parent drain of CGI stdout | Extra `write()` syscalls on the pipe widen tails under concurrency |
| “Pacing” experiments | Measured for prefork; **hurt** on the cgid hosting-like stack → **removed** from Marty. Library keeps a single-write response path only — also fine on netcup without reintroducing pacing |
| Prefork-only Docker ≠ every host | Prefer outside numbers for claims; pipe hosts exist (netcup), socket hosts exist (all-inkl / Hetzner) |
| Shared-host fair-use (netcup) | Calm **320/min** fine; aggressive **C=100** grill ~72 req/s then upstream **IP ban** (Hetzner queues instead). Not a library issue — document limits, don’t grill production accounts for marketing numbers |

Snapshots: [`benchmarks/complex/RESULTS-SNAPSHOTS.md`](../benchmarks/complex/RESULTS-SNAPSHOTS.md) (Run O = netcup outside + capacity).

## 3. Short URLs via `.htaccess` / `mod_rewrite`

**What it is:** Same CGI binary; pretty paths (`/routing/hello/Doc` → `cgi-bin/…/hello/Doc`) with `RewriteRule` + `[PT]`. Needs `AllowOverride` / rewrite enabled (typical on shared hosts that allow `.htaccess`).

**DX:** feature **`htaccess`** — alias in `[package.metadata.marty]`, binary = `[package].name`; `build.rs` calls `write_next_to_binary_from_manifest()` → **`{binary}.htaccess`** beside the binary. Consumer walkthrough: [README — Quick start](../README.md#quick-start) and [`examples/02_routing/README.md`](../examples/02_routing/README.md). App `Router` patterns are **not** mirrored as rewrite rules (they use `PATH_INFO`).

## 4. Less common (aware, not first-class)

| Variant | Status |
|---------|--------|
| CGI without `ScriptAlias` (`AddHandler cgi-script`, `.cgi` suffix) | Mentioned in hoster docs (e.g. IONOS); probe per host |
| LiteSpeed / nginx + CGI/fcgiwrap | Possible elsewhere; **not** mirrored yet — out of primary scope |
| Windows CGI | Unsupported (Unix CGI env) |
| Long-lived app server (Axum on a VPS) | Outside Marty pitch — run the server yourself |

## How to use this map

1. **Ship for (1)** — sync binary + hosting tools.  
2. **Regress on (1) and (2)** — Docker recipes above.  
3. **Don’t invent library features** for (2)-only pathologies unless they also help (1).  
4. **Prove a new hoster** with `hosting-probe` once, then optional outside smoke.
