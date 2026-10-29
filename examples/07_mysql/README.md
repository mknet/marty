# Example `07_mysql`

Sync [`marty::Router`](../../marty/src/router.rs) + **MySQL/MariaDB** (read + write). Target: **all-inkl–style** shared hosting (static musl CGI, DB usually on `localhost`). The `mysql` crate is built **without OpenSSL/aws-lc** (`minimal-rust`) so musl cross-builds stay simple; use `default-rustls` if you need TLS to a remote host.

Deploy basics (`.htaccess`, musl): [`../02_routing/README.md`](../02_routing/README.md) and [root README — Quick start](../../README.md#quick-start).

## What it does

`GET` → create table `marty_user` if needed → `INSERT` one row → `SELECT` names → plain text body like `07_mysql: Bob Bob …`.

## Credentials (all-inkl)

Apache **suexec** on all-inkl does **not** pass `.htaccess` `SetEnv` into the CGI. Put settings in **`marty_mysql.conf`** in the process cwd (typically next to the binary under `cgi-bin/`), or export env vars when you control the environment (local smoke):

| Key (conf file) | Env override |
|-----------------|--------------|
| `host` | `MARTY_MYSQL_HOST` (default `localhost`) |
| `user` | `MARTY_MYSQL_USER` |
| `password` | `MARTY_MYSQL_PASSWORD` |
| `database` | `MARTY_MYSQL_DATABASE` |
| `port` | `MARTY_MYSQL_PORT` (default `3306`) |

```bash
cp marty_mysql.conf.example marty_mysql.conf
# edit user/password/database from KAS → MySQL
```

In KAS: create a database + user, grant access. Host for same-server CGI is almost always `localhost`.

## Build / upload

```bash
cargo zigbuild -p marty-07-mysql --release --target x86_64-unknown-linux-musl
# or: cd examples/07_mysql && just linux
```

Upload `marty-07-mysql` → `cgi-bin/` (`chmod 755`), `.htaccess` mount (`alias = mysql`), and `marty_mysql.conf` beside the binary (mode `600` if possible).

## Local smoke

Needs a reachable MySQL/MariaDB and config/env:

```bash
export MARTY_MYSQL_USER=… MARTY_MYSQL_PASSWORD=… MARTY_MYSQL_DATABASE=…
# optional: MARTY_MYSQL_HOST=127.0.0.1
cd examples/07_mysql && just smoke
# or from repo root: just smoke-07-mysql
```
