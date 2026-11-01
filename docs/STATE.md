# State with one process per request

The question behind most CGI objections: *where does state live if every request is a new process?* Short answer: in the client (cookie), in a file (SQLite), or in a database server (MySQL). Never in RAM. This page shows each option with code that matches the examples in this repo. Trust boundaries for the same pieces: [`SECURITY.md`](SECURITY.md).

## 1. What survives a request

On a verified shared host (all-inkl, Hetzner Webhosting S, netcup EiWoMiSau) Apache starts the binary, feeds the request through env + stdin, reads the response from stdout, and the process exits ([`CGI-VARIANTS.md`](CGI-VARIANTS.md)).

| Survives | Does not survive |
|----------|------------------|
| Cookies (the browser sends them back) | `static` / `OnceLock` / `lazy_static` values |
| Files the site user writes (SQLite, JSON, uploads) | Connection pools, prepared statements |
| Rows in MySQL/MariaDB | In-memory caches, rate-limit counters |
| HTTP cache entries in browsers and proxies | Background threads (they die with the process) |

Treat each request like a cold start of a cloud function, minus the platform. Anything the next request needs goes to one of the left-hand stores before the response is written.

## 2. Sessions: signed cookie or session ID + DB row

**Signed cookie** ([`examples/03_session`](../examples/03_session/src/main.rs), feature `session`). The cookie carries the data plus an HMAC. No server storage, no lookup.

```rust
let cfg = marty::Config::load()?;                        // env MARTY_SESSION_SECRET, else the config file
let store = marty::SessionStore::from_config(&cfg)?.secure(true);
let mut session = store.load(&req);
session.insert("user_id", "42");
let resp = store.save(&session, resp);
```

Limits ([`SECURITY.md` §3](SECURITY.md#3-signed-cookie-session)): the values are readable by the client, the server cannot revoke a single session, and a copied cookie stays valid until its signed expiry (`max_age`, default 600 s). Good for a visit counter, a flash message, a CSRF token, a "remember the last tab".

**Session ID in the cookie, data in the DB.** The cookie holds a random token. The server looks it up per request. Logout deletes the row, and then the session is gone everywhere. This is the shape for logins.

```rust
use marty::{Cookie, SameSite};
use rusqlite::{Connection, OptionalExtension, params};
use std::io::Read;

// Migration (see §5):
// CREATE TABLE session (token TEXT PRIMARY KEY, user_id INTEGER NOT NULL, expires INTEGER NOT NULL);

fn new_token() -> std::io::Result<String> {
    let mut buf = [0u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut buf)?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

fn login(conn: &Connection, user_id: i64) -> rusqlite::Result<Cookie> {
    let token = new_token().expect("/dev/urandom");
    conn.execute(
        "INSERT INTO session (token, user_id, expires) VALUES (?1, ?2, unixepoch() + 86400)",
        params![token, user_id],
    )?;
    Ok(Cookie::new("sid", token).http_only(true).secure(true).same_site(SameSite::Lax))
}

fn current_user(conn: &Connection, req: &marty::CgiRequest) -> rusqlite::Result<Option<i64>> {
    let Some(token) = req.cookies().get("sid").map(str::to_string) else { return Ok(None) };
    conn.query_row(
        "SELECT user_id FROM session WHERE token = ?1 AND expires > unixepoch()",
        [token],
        |r| r.get(0),
    )
    .optional()
}

fn logout(conn: &Connection, token: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM session WHERE token = ?1", [token]).map(|_| ())
}
```

The same table works in MySQL (`token CHAR(64) PRIMARY KEY`, `expires` as `INT` or `DATETIME`). Expired rows can be deleted opportunistically (`DELETE … WHERE expires < unixepoch()` on, say, every 100th login). The cost is one indexed lookup per request.

| | Signed cookie | Session ID + DB |
|---|---|---|
| Server storage | none | one row per session |
| Logout / revoke one user | no (only rotate the secret for everyone) | `DELETE` the row |
| Data visible to the client | yes | no (only the token) |
| Per-request cost | one HMAC | one DB lookup |

## 3. SQLite on shared hosting

SQLite fits CGI well: no server, one file, and the "connection" is opening that file. Concurrent CGI processes are the case to configure for.

```rust
use std::time::Duration;

let conn = rusqlite::Connection::open(&db_path)?;
conn.busy_timeout(Duration::from_secs(5))?;               // wait for a writer instead of SQLITE_BUSY
conn.pragma_update(None, "journal_mode", "WAL")?;         // readers do not block the writer
conn.pragma_update(None, "synchronous", "NORMAL")?;       // the usual pairing with WAL
```

- **Location:** outside the docroot, e.g. `~/data/app.db`, with the path from `marty::Config` (`MARTY_DB_PATH=…`). [`examples/05_sqlite`](../examples/05_sqlite/src/main.rs) writes `./marty_05_sqlite.db` into the CGI cwd, which is fine for a demo and wrong for real data: a file under the docroot can end up downloadable.
- **Permissions:** the CGI runs as the site user (suexec), so the file is created with that owner. The **directory** must be writable too: SQLite creates `-wal`, `-shm`, and journal files next to the DB. `chmod 700` on the directory, `600` on the file where the host allows it.
- **Concurrency:** many readers plus one writer at a time. `busy_timeout` makes a second writer wait instead of failing. Keep write transactions short; do not hold one open while rendering HTML.
- **Network filesystems:** WAL needs shared memory between processes and does not work on NFS. If the host's home directory is on NFS (ask, or check `mount` over SSH), keep the default rollback journal (`journal_mode = DELETE`) and rely on `busy_timeout`.
- **Backups:** `sqlite3 app.db ".backup backup.db"` over SSH, or `VACUUM INTO` from an admin route. Copying the file while a writer is active can produce a torn copy.

## 4. MySQL / MariaDB: one connection per request

The shared host's MySQL usually runs on the same machine (`localhost`). Opening a connection per request is the normal CGI pattern and costs little next to the TLS handshake and network round trip of the request itself (outside P50 is 30–70 ms on the verified hosts, [`HOSTERS.md`](HOSTERS.md#reading-the-table)).

```rust
let opts = db_opts()?;                                    // marty::Config → OptsBuilder, see examples/07_mysql
let mut conn = mysql::Conn::new(opts)?;                   // exactly one connection, closed on drop
```

[`examples/07_mysql`](../examples/07_mysql/src/main.rs) builds a `mysql::Pool` and takes one connection from it. A pool exists to reuse connections across requests, which a process that lives for one request cannot do, so `Conn::new` is enough. The credentials (`MARTY_MYSQL_*`) come from the config file via `marty::Config` (§8).

## 5. Schema and migrations

The demos run `CREATE TABLE IF NOT EXISTS` on every request. That is a schema write per request and does not cover the second version of a table. Store a schema version and run pending steps once.

**SQLite:** `PRAGMA user_version` is a free integer in the file header.

```rust
use rusqlite::{Connection, TransactionBehavior};

const MIGRATIONS: &[&str] = &[
    "CREATE TABLE note (id INTEGER PRIMARY KEY, text TEXT NOT NULL, created INTEGER NOT NULL)",
    "CREATE TABLE session (token TEXT PRIMARY KEY, user_id INTEGER NOT NULL, expires INTEGER NOT NULL)",
];

fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    let version = |c: &Connection| c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0));
    if version(conn)? as usize >= MIGRATIONS.len() {
        return Ok(()); // common case: one cheap read, no lock
    }
    // Two CGI processes may race here; IMMEDIATE takes the write lock, then re-check.
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version(&tx)? as usize) {
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
    }
    tx.commit()
}
```

**MySQL:** a one-row table `schema_version (version INT NOT NULL)` plus `SELECT GET_LOCK('app_migrate', 10)` around the steps. DDL commits implicitly in MySQL, so write each step so that re-running it after a crash is harmless.

**When to run:** either at the start of each request (the version check above is one read), or only from a protected admin route such as `GET /admin/migrate?token=…` with the token from `marty::Config`. The second keeps schema changes out of the visitor path and makes them visible in your deploy checklist.

## 6. Writes: POST, then redirect

The examples insert on `GET` to keep the demo to one URL. Real writes go through `POST` and answer with `303 See Other` (Post/Redirect/Get), so a reload does not resubmit.

```rust
router.post("/notes", |req| {
    let Some(text) = req.form_value("text") else {
        return marty::CgiResponse::new(400, "missing text\n");
    };
    // insert `text` …
    marty::CgiResponse::new(303, "").header("Location", "/app/notes")
});
```

`req.form()` / `req.form_value()` parse `application/x-www-form-urlencoded` bodies synchronously ([`marty/src/form.rs`](../marty/src/form.rs)); the body is already in memory when the handler runs.

**CSRF:** Marty has no CSRF layer. Session cookies set with `SameSite=Lax` (the `SessionStore` default, and the cookie in §2) are not sent on cross-site `POST`s in current browsers, which covers the classic form attack. For more, put a random token in the session and in a hidden form field and compare them, or reject `POST`s whose `HTTP_ORIGIN` env var is a foreign origin.

## 7. Why no Redis or in-memory cache?

Shared hosting gives you no daemon: nothing keeps running between requests, and you cannot start a Redis next to Apache. Use what is there instead:

| Need | Use |
|------|-----|
| Cache an expensive result | a row in SQLite/MySQL with a timestamp, or a file in a private directory |
| Cache a whole page | write the rendered HTML into the docroot and let Apache serve it as a static file; regenerate on change |
| Let clients cache | `Cache-Control`, `ETag` / `Last-Modified` headers via `CgiResponse::header` (a `304` still starts the process, but skips the work and the body) |
| Rate limiting | a counter row keyed by IP + time bucket; the hoster's own filter already blocks abusive IPs ([`SECURITY.md` — Availability](SECURITY.md#availability)) |

If the app needs a long-lived process (websockets, background jobs, a hot in-memory cache), that is the Uberspace/VPS shape from [`HOSTERS.md`](HOSTERS.md): run Axum directly there.

## 8. Secrets and settings

`.htaccess` `SetEnv` does not reach a suexec CGI, so secrets go into one file **outside the docroot**. Not into `cgi-bin/`: on a shared host that is under the docroot, and whether Apache (or a front nginx) serves a `.conf` from there is not something to rely on. Feature `config` (included in `session`):

```text
# ~/private/marty.conf (netcup: next to httpdocs/), mode 600 — keys are the env var names
MARTY_SESSION_SECRET="9c1f…"
MARTY_DB_PATH=/home/hosting123/data/app.db
MARTY_ADMIN_TOKEN="…"
```

```rust
let cfg = marty::Config::load()?;                        // env vars, then the compiled-in file
let db_path = cfg.get("MARTY_DB_PATH").unwrap_or_else(|| "../../data/app.db".into());
let admin_token = cfg.require("MARTY_ADMIN_TOKEN")?;     // error names the key and the file
```

- **Which file:** the path is compiled in by `cargo marty build/deploy` from the target's `config` (relative to the binary, e.g. `../../private/marty.conf`). `Config` never searches the cwd or the binary's directory. Locally, `MARTY_CONFIG=/path/marty.conf` or plain env vars.
- **Verified on all-inkl** (Nov 2026): `_config/marty.conf` next to `htdocs/`, uploaded over FTPS, is readable by the CGI; the playground's `/visits` answers 200 only when it is.
- **Upload:** `cargo marty deploy` copies `deploy/<target>/marty.conf` (gitignored, created by `cargo marty init`) to `config` (directory `700`, file `600`) and refuses a `config` inside `docroot` or `cgi_dir` ([`tools/cargo-marty`](../tools/cargo-marty/)).
- **Format:** `KEY=value`, `#` comments, `"…"` for values with spaces or ` #`. A malformed line is an error with its line number, not a silently truncated password.
- **Warnings** on stderr (the host's CGI error log): file readable by group/other, or file under `DOCUMENT_ROOT`.
