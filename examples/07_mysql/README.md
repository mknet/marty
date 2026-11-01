# Example `07_mysql`

Sync [`marty::Router`](../../marty/src/router.rs) + **MySQL/MariaDB** (read + write). Target: **all-inkl–style** shared hosting (static musl CGI, DB usually on `localhost`). The `mysql` crate is built **without OpenSSL/aws-lc** (`minimal-rust`) so musl cross-builds stay simple; use `default-rustls` if you need TLS to a remote host.

Deploy basics (`.htaccess`, musl): [`../02_routing/README.md`](../02_routing/README.md) and [root README — Quick start](../../README.md#quick-start).

## What it does

`GET` → create table `marty_user` if needed → `INSERT` one row → `SELECT` names → plain text body like `07_mysql: Bob Bob …`.

## Credentials (all-inkl)

Apache **suexec** on all-inkl does **not** pass `.htaccess` `SetEnv` into the CGI, so production settings come from one config file **outside the docroot**. The keys in the file are the env var names; an env var with the same name wins (local smoke). Lookup is [`marty::Config`](../../marty/src/config.rs) (feature `config`).

| Key (env var and file) | Default |
|------------------------|---------|
| `MARTY_MYSQL_HOST` | `localhost` |
| `MARTY_MYSQL_USER` | required |
| `MARTY_MYSQL_PASSWORD` | empty |
| `MARTY_MYSQL_DATABASE` | required |
| `MARTY_MYSQL_PORT` | `3306` |

```bash
cargo marty init prod --host … --user … --domain example.de --protocol ftps \
  --docroot example.de --config _config/example.de/marty.conf   # also creates deploy/prod/marty.conf (600)
# copy the keys from examples/07_mysql/marty.conf.example and fill in user/password/database from KAS → MySQL
# (quote the password if it contains spaces or " #")
```

In KAS: create a database + user, grant access. Host for same-server CGI is almost always `localhost`.

The binary never reads a config from its own directory or the cwd: on the host both are `cgi-bin/` under the docroot, where a `.conf` may be downloadable. Where it does read is compiled in by `cargo marty` (next section).

## Build / upload

With [`cargo marty`](../../tools/cargo-marty/), in `marty.deploy.toml` (gitignored):

```toml
[target.prod]
preset = "generic"
host = "…"
user = "…"
docroot = "example.de"                      # all-inkl: the domain's folder in the account
config = "_config/example.de/marty.conf"    # outside the docroot; not a folder a domain points to
                                            # local file: deploy/prod/marty.conf
package = "marty-07-mysql"
domain = "example.de"                       # check → https://example.de/mysql/
```

```bash
cargo marty deploy prod      # binary → example.de/cgi-bin (755), conf → _config/example.de (700/600), .htaccess merge
cargo marty check prod
```

`cargo marty` compiles the path from `cgi_dir` to `config` into the binary (`../../_config/example.de/marty.conf`). A plain `cargo zigbuild` / `just linux` build has no compiled-in path and reads env vars only.

## Local smoke

Needs a reachable MySQL/MariaDB and env vars, or `examples/07_mysql/marty.conf` (the justfile passes it via `MARTY_CONFIG`):

```bash
export MARTY_MYSQL_USER=… MARTY_MYSQL_PASSWORD=… MARTY_MYSQL_DATABASE=…
# optional: MARTY_MYSQL_HOST=127.0.0.1
cd examples/07_mysql && just smoke
# or from repo root: just smoke-07-mysql
```
