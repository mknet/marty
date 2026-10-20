# CGI deployment variants (Apache-first)

CGI is just “one process per request.” In practice almost everyone meets it behind **Apache**. There are several common stacks; Marty aims to **know and mirror** them (Docker + probes), not to paper over every quirk with library workarounds.

Walk **most common for our target → historical / lab** below. Problems are called out so claims stay honest; Marty does **not** need a special fix for every one (e.g. prefork pacing was measured and dropped).

## 1. Shared hosting today — `mpm_event`/`worker` + `mod_cgid` + suexec

**What it is:** Threaded MPM; CGI stdin/stdout are **unix sockets** (`mod_cgid`). The binary runs as the **site/FTP user** through suexec with a **cleaned environment** (CGI/1.1 + `HTTP_*` / `SSL_*`; `.htaccess` `SetEnv` does **not** reach the CGI). Often KeepAlive, chunked CGI, HTTP/2 + TLS in front.

**Why it matters:** This is the **primary Marty target** (all-inkl–class). Upload a static musl binary → `cgi-bin/` → `chmod 755`.

**Local mirror:** [`benchmarks/complex/docker/Dockerfile.hosting`](../benchmarks/complex/docker/Dockerfile.hosting)  
`just bench-fair-hosting` / `just docker-smoke-hosting` in `benchmarks/complex/`.

**Outside:** [`tools/hosting-probe/`](../tools/hosting-probe/) (fingerprint); `just outside` in [`benchmarks/complex/`](../benchmarks/complex/) (low-volume sync/axum/php).

**Known issues (document, don’t over-engineer):**

| Issue | Note |
|-------|------|
| Go CGI OOM at arena init | Address-space limits on shared hosts — negative proof in complex bench |
| No `SetEnv` into CGI | Configure via query/path/files, not Apache env toggles |
| TLS/RTT floor (~40–60 ms) | Dominates light routes; fair compares need this in mind |
| Fair-use / load | Keep outside benches low-volume |

## 2. Classic lab / many tutorials — `mpm_prefork` + `mod_cgi` (+ often `mod_php`)

**What it is:** One Apache process per connection; CGI stdio are **pipes**. Common in Docker PHP images and older docs. Easy to stand up; **not** what all-inkl-style hosts run.

**Local mirror:** [`benchmarks/complex/docker/Dockerfile`](../benchmarks/complex/docker/Dockerfile) (default fair bench).  
`just bench-fair` in `benchmarks/complex/`.

**Known issues (historical):**

| Issue | Note |
|-------|------|
| Parent drain of CGI stdout | Extra `write()` syscalls on the pipe widen tails under concurrency |
| “Pacing” experiments | Measured for prefork; **hurt** on the hosting-like stack → **removed** from Marty. Library keeps a single-write response path only |
| Prefork ≠ hosting truth | Do not quote prefork-only numbers as shared-host performance |

Snapshots: [`benchmarks/complex/RESULTS-SNAPSHOTS.md`](../benchmarks/complex/RESULTS-SNAPSHOTS.md).

## 3. Short URLs via `.htaccess` / `mod_rewrite`

**What it is:** Same CGI binary; pretty paths (`/routing/hello/Doc` → `cgi-bin/…/hello/Doc`) with `RewriteRule` + `[PT]`. Needs `AllowOverride` / rewrite enabled (typical on shared hosts that allow `.htaccess`).

**Examples today:** `examples/*/htaccess.basic.in`, `examples/apache-combined-vhost.htaccess.in` (copied to `dist.htaccess` on `just linux`).

**Planned DX:** generate these from the app’s routes (see [DIRECTION](DIRECTION.md#planned-features)).

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
