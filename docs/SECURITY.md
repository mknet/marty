# Security

What each Marty deployment actually isolates, and what stays with the app and the host. Marty has no auth layer, no WAF, and no secret store. This note is the threat picture for the shapes in [`CGI-VARIANTS.md`](CGI-VARIANTS.md) and [`HOSTERS.md`](HOSTERS.md).

Why ship on Marty (performance-first product situations): [`DIRECTION.md` — Why ship on Marty](DIRECTION.md#why-ship-on-marty).

Jurisdiction is a different claim: [`COST.md` — Digital sovereignty](COST.md#digital-sovereignty). A German shared host does not make the app safer against its own bugs or against the hoster.

## Scenarios

### 1. Visitor → your CGI (primary)

One HTTP request starts one process. The binary reads the CGI environment and stdin, writes a document response, and exits. That is the model on verified hosts (all-inkl, Hetzner Webhosting S, netcup EiWoMiSau) and in the hosting Docker mirror.

| Property | What it means |
|----------|----------------|
| Process lifetime | Memory, open DB connections, and in-process state die with the request. A later request is a new process. |
| Identity | With suexec (all-inkl; site user on Hetzner and netcup) the process runs as **your** site/FTP user, with a cleaned CGI environment. `.htaccess` `SetEnv` does **not** arrive in the process ([`CGI-VARIANTS.md`](CGI-VARIANTS.md)). |
| Input | Method, path, query, headers, and body are attacker-controlled. `PATH_INFO` / `{param}` / `{*catch}` are strings the router extracted, not checked paths. |
| Pretty URL | The generated rewrite only changes the public path. `/cgi-bin/<binary>/…` still runs the same binary ([`htaccess` comments](../marty/src/htaccess.rs)). Access control belongs in the handler. |

Other customers on the machine have a different uid when suexec is in effect. They share the kernel. Your other vhosts on the **same** account share the uid and can read files that account can read.

The hoster (root, backups, support, legal process) can read the home directory, the binary, config files, and databases. suexec separates customers from each other.

### 2. Secrets and data outside the docroot

There is no platform secret manager. On suexec hosts the practical store is one file the site user can read, **outside the document root**, mode `600` (directory `700`) when the host allows it. Not in `cgi-bin/` either: whether a `.conf` there is executed (500) or served as text depends on the Apache handler setup and on a front proxy such as netcup's nginx, and none of that is verified per host.

Feature **`config`** ([`marty::Config`](../marty/src/config.rs)) reads env vars first (local runs), then that file. It never searches the cwd or the binary's directory, because on a shared host both are `cgi-bin/`. The file path is compiled in by [`cargo marty`](../tools/cargo-marty/) from the target's `config` (relative to the binary, e.g. `../../private/marty.conf`), and `cargo marty deploy` refuses a `config` inside `docroot` or `cgi_dir`. At runtime `Config` warns on stderr when the file is group/other-readable or lies under `DOCUMENT_ROOT`. Where the login cannot reach anything outside the docroot, there is no safe place for a secret file; deploy stops instead of putting one there.

| Data | Where it lives | Concern |
|------|----------------|---------|
| MySQL password | `MARTY_MYSQL_*` in the config file, or as env vars locally | Env works for local smoke. On all-inkl, `SetEnv` never reaches CGI, so production uses the file. A world-readable conf is readable by other local users if the directory is traversable. |
| Session secret | `MARTY_SESSION_SECRET` in the config file, or as env var locally | [`SessionStore::from_config`](../marty/src/session.rs) refuses a missing secret or one shorter than 32 bytes, so [`examples/03_session`](../examples/03_session/src/main.rs) answers 500 instead of signing with a public default. On suexec hosts the file is the only path, because `SetEnv` does not arrive. |
| SQLite | `./marty_05_sqlite.db` in the CGI cwd ([`examples/05_sqlite`](../examples/05_sqlite/src/main.rs)) | Created as the site user. Check the resulting mode and directory listing (`Options +Indexes`). Concurrent CGI processes share one file; a loose mode exposes the rows. |
| Secret compiled into the ELF | Upload artifact | `strings` on a world-readable binary recovers it. `cgi-bin` binaries are meant to be executed by the web server. |

FTP/SFTP and the panel password replace the binary and read those files. That account is the deployment boundary.

### 3. Signed cookie session

Where state lives at all with one process per request (cookie vs DB session, SQLite, MySQL): [`STATE.md`](STATE.md).

Feature **`session`**: [`SessionStore`](../marty/src/session.rs) puts a percent-encoded map and an HMAC-SHA256 in one cookie so state survives process-per-request. `save` sets `HttpOnly` and `SameSite=Lax`. A bad signature or an expired cookie loads as an empty session.

| Behavior | Consequence |
|----------|-------------|
| Signed, not encrypted | The browser and anyone who sees the cookie can read the values (`count=3`, user ids, …). Keep payloads small and non-secret. |
| Expiry inside the MAC | The value is `v1.<exp>.<payload>.<sig>`; `max_age` (default 600s) sets both `Max-Age` and the signed `exp`, and `load` rejects an expired cookie. A copied value stops working at `exp`, whatever the client does. `max_age(None)` writes `exp = 0` (no expiry), which brings back unlimited reuse of a copied value. |
| No revocation | Before `exp`, a copied cookie still verifies. Rotating the secret invalidates every session; one user cannot be revoked alone. For logout that really ends a session, use a session ID + DB row ([`STATE.md` §2](STATE.md#2-sessions-signed-cookie-or-session-id--db-row)). |
| Replay | Within its lifetime, an old signed cookie restores that map. The visit counter in example 03 can be rolled back by replaying an earlier cookie that has not expired yet. |
| `secure` defaults to `false` | Call [`.secure(true)`](../marty/src/session.rs) when the vhost is HTTPS-only. TLS ends at the hoster; Marty does not infer it. |
| Secret source | Use [`SessionStore::from_config`](../marty/src/session.rs): no secret or fewer than 32 bytes is an error, not a fallback. `SessionStore::new` still accepts any bytes; a short or published secret makes every cookie forgeable. |

### 4. Long-lived process

Uberspace web backends, a VPS, or Axum listening on a port are a different shape: one process serves many requests, memory persists, and you own the socket. [`HOSTERS.md`](HOSTERS.md) calls Uberspace out for that reason. Marty is optional there; the CGI notes above do not describe that process.

`serve_cgi` (feature **`tower`**) is still one CGI process per request. It does not become a long-lived server.

### 5. Cloud function twins

The apps under [`alternatives/`](../alternatives/) run the same kind of HTTP work behind Lambda, Azure Functions, Cloudflare Workers, or Scaleway. Isolation is the platform’s (account, IAM, isolate, or microVM) instead of suexec on a shared kernel. The operator can still read the deployment and its secrets. A public function URL has no Marty auth either. Cost and latency comparisons in [`COST.md`](COST.md) and [`alternatives/RESULTS-SNAPSHOTS.md`](../alternatives/RESULTS-SNAPSHOTS.md) are not security rankings.

## App-level concerns

These show up on every scenario above. Marty leaves them to the handler.

| Concern | Library behavior | App side |
|---------|------------------|----------|
| Request body size | [`CgiRequest::from_env`](../marty/src/cgi.rs) allocates `CONTENT_LENGTH` bytes with no cap, then reads stdin. | Host limits vary. Cap size before adding uploads. |
| Header injection | [`CgiResponse::header`](../marty/src/cgi.rs) and [`Cookie::encode`](../marty/src/cookie.rs) write the strings as given. CR/LF in a name or value breaks the CGI header block. | Keep request data out of header names and values. |
| Paths | Catch-all and path params are decoded route text. | Opening files from that string is a traversal bug in the app. The router does not touch the filesystem. |
| HTML | Default bodies use `text/plain`. | Escape anything you interpolate into HTML or into a URL. |
| SQL | Examples use placeholders (`?`). | Keep that. Errors in example 07 include the driver message in the HTTP body. |
| Logs | `eprintln!` goes to the host’s CGI error log. | Leave secrets and raw bodies out of stderr. |
| Authn / CSRF | None. | Cookies and sessions do not identify a user until the app does. State-changing `GET`s in the examples are demos. |

## Availability

Shared hosts apply their own filters. On netcup EiWoMiSau a calm 320 requests/min succeeded; a hard concurrency grill ended in a client IP ban. Hetzner queued and stayed up. That is the host’s abuse control, documented in [`CGI-VARIANTS.md`](CGI-VARIANTS.md) and [Run O](../benchmarks/complex/RESULTS-SNAPSHOTS.md#run-o--outside-netcup-eiwomisau-2026-10-29). It is not a confidentiality control, and it is not a reason to grill a production account.

Some hosters disable CGI entirely (Infomaniak, see [`HOSTERS.md`](HOSTERS.md)). “CGI” on a tariff page still needs [`tools/hosting-probe`](../tools/hosting-probe/) before you treat the plan as this model.
