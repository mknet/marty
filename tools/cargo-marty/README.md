# cargo-marty

`cargo marty` builds a [Marty](https://github.com/mknet/marty) CGI binary for Linux musl, stages the deploy bundle, and uploads it to shared hosting over SFTP, FTPS, or FTP. Hosters differ in **data** (paths, protocol), not in code: a target in `marty.deploy.toml` plus an optional preset.

```bash
cargo install cargo-marty
# needs: zig (for cargo-zigbuild, installed on first build), sftp and/or curl
```

## Commands

| Command | What it does |
|---------|--------------|
| `cargo marty build [<target>] [-p pkg] [--arm]` | `cargo zigbuild --release --target x86_64-unknown-linux-musl` (`--arm`: aarch64), checks the ELF header, writes `target/deploy/<pkg>/{cgi-bin/<bin>, .htaccess}`. With `<target>`: its package, arch, and config path compiled in |
| `cargo marty deploy <target> [-p pkg] [--dry-run]` | build for the target, upload the binary (temp name, `chmod 755`, rename over the old one), upload `deploy/<target>/<file>` → `config` (dir `700`, file `600`), merge the marty block into the remote `.htaccess` |
| `cargo marty check <target> [<path>]` | one `GET https://<domain>/<alias>/<path>` (alias from `[package.metadata.marty]`): status, latency, `Server` header |
| `cargo marty init <target> [--preset …] [--host …] [--user …] [--domain …] [--protocol …] [--docroot …] [--config …]` | append a `[target.<name>]` skeleton to `marty.deploy.toml`, create `deploy/<target>/marty.conf` (600, from `marty.conf.example` if present), add both to `.gitignore` |
| `cargo marty presets` | list embedded presets |

`--dry-run` prints the remote operations without building or connecting.

## `marty.deploy.toml`

Looked up in the cwd, then in the workspace root. Keep it out of git. It holds no passwords. One `[target.<name>]` per environment:

```toml
[target.prod]
preset = "netcup"                        # optional defaults: protocol, docroot, cgi_dir
host = "ssh.example.org"                 # or an alias from ~/.ssh/config
user = "hosting123"
config = "private/marty.conf"            # remote, outside the docroot; local: deploy/prod/marty.conf
domain = "example.org"                   # only for `check`: https://example.org/<alias>/
# protocol = "sftp"                      # sftp | ftps | ftp
# port = 22
# docroot = "httpdocs"                   # relative to the login dir; overrides the preset
# cgi_dir = "{docroot}/cgi-bin"
# package = "my-cgi"                     # default: -p, else the package in the cwd
# arch = "aarch64"                       # default x86_64
# url = "https://example.org/health"     # full URL for `check` instead of domain + alias

[target.stage]
preset = "netcup"
host = "ssh.example.org"
user = "hosting123"
docroot = "stage.example.org/httpdocs"   # own docroot: stage and prod must not share cgi-bin/
config = "stage.example.org/private/marty.conf"   # local: deploy/stage/marty.conf
domain = "stage.example.org"
```

Remote paths are relative to the login directory unless they start with `/`. `..` is rejected.

Layout next to `marty.deploy.toml` (both gitignored):

```
marty.deploy.toml
deploy/
├── prod/marty.conf     → uploaded to [target.prod] config
└── stage/marty.conf    → uploaded to [target.stage] config
```

The local file is `deploy/<target>/<file name of config>`, so it has the same name as on the server. `config_source = "…"` overrides that path.

## Config file (secrets)

The app reads one file with [`marty::Config`](https://docs.rs/marty/latest/marty/config/) (keys = env var names, e.g. `MARTY_SESSION_SECRET=…`). Where it lives:

- **Never under the docroot.** `deploy` refuses a `config` inside `docroot` or `cgi_dir`. A login whose root *is* the docroot (`docroot = "."`) has no safe place, and deploy stops instead of guessing.
- **Compiled in, not searched.** `build`/`deploy` set `MARTY_BUILD_CONFIG_PATH` to the path from `cgi_dir` to `config` (netcup: `../../private/marty.conf`), resolved at runtime next to the binary. An absolute `config` (`/…`) is compiled in as is. That is only right when the login sees the same filesystem as the CGI (SSH/SFTP without chroot); FTP logins are usually chrooted, so use relative paths there.
- **No `config`:** the binary reads env vars only. Locally, `MARTY_CONFIG=/path/marty.conf` points it at a file.

Each target is built with its own path, so stage and prod binaries differ; deploy rebuilds per target.

## Credentials

- **SFTP** runs the system `sftp` in batch mode, so `~/.ssh/config`, keys, and the agent apply. Without a key, ssh prompts on the terminal. With `MARTY_DEPLOY_PASSWORD` set, ssh gets it through `SSH_ASKPASS` (OpenSSH 8.4+). Connect once with `sftp` by hand so the host key is known.
- **FTP(S)** runs `curl`. The password comes from `MARTY_DEPLOY_PASSWORD` or a prompt and is passed on curl's stdin, not on its command line. `ftps` means explicit TLS (`--ssl-reqd`). Plain `ftp` works and prints a warning.

## `.htaccess` merge

The bundle's `.htaccess` comes from `marty::htaccess` (feature `htaccess`, `build.rs`) and sits between `# BEGIN marty <binary>` and `# END marty <binary>`. Deploy downloads the remote `.htaccess`, replaces that block in place or **prepends** it when absent (so a CMS catch-all such as WordPress's `RewriteRule . /index.php` cannot swallow the mount), and uploads the result. Other lines stay byte for byte. Packages without `[package.metadata.marty] alias` do not touch `.htaccess`.

## Presets

Presets are TOML files in [`presets/`](presets/) with `protocol`, `docroot`, `cgi_dir`, `notes`. A new hoster is a new file plus one line in `src/target.rs`, no transport code.

| Preset | docroot | cgi_dir | Notes |
|--------|---------|---------|-------|
| `allinkl` | `htdocs` | `{docroot}/cgi-bin` | verified Nov 2026 over FTPS; KAS: domain → `/<domain>/htdocs/`, FTP user restricted to `/<domain>/`, `config = "_config/marty.conf"` |
| `generic` | (set per target) | `{docroot}/cgi-bin` | any Apache host with `cgi-bin` under the docroot |
| `netcup` | `httpdocs` | `{docroot}/cgi-bin` | verified on EiWoMiSau; enable **CGI-Unterstützung** in the WCP |

License: Apache-2.0.
