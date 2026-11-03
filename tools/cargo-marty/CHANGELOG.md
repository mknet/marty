# Changelog

All notable changes to the **cargo-marty** crate are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [SemVer](https://semver.org/) (`0.y.z`: `y` may include breaking changes).

## [Unreleased]

## [0.1.1] — 2026-11-01

### Added

- `cargo marty probe <target>` builds the hosting probe (embedded, works
  outside this repo), uploads it with `probe.pl` into `cgi_dir` under a random
  name, records the inside view (CGI env, pipe vs socket, parent chain, httpd
  processes, system) and the outside view (headers, keep-alive, HTTP/2,
  timing), removes both files and saves the report to
  `deploy/<target>/probe-<timestamp>.txt` (600).
- `probe` flags: `--arm` (aarch64 probe), `--keep` (leave the files on the
  server), `--no-perl` (upload the binary only), `--dry-run` (print the remote
  operations without building or connecting).
- `probe` prints a short summary on the terminal (status, pipe/socket, uid, OS,
  HTTP version, keep-alive, server time); the full report goes to the file.
- The probe reports uid and ppid from libc, so they are present even when
  `/proc` is restricted (`hidepid`). Certificates in the report are shortened.

## [0.1.0] — 2026-11-01

### Added

- `cargo marty build [<target>]`: musl build via `cargo zigbuild` (`--arm` for
  aarch64), ELF header check, deploy bundle `target/deploy/<pkg>/{cgi-bin/<bin>, .htaccess}`;
  with a target, its package, arch and config path are compiled in.
- `cargo marty deploy <target>`: upload over SFTP, FTPS or FTP (temp name,
  `chmod 755`, rename), upload the config file outside the docroot (dir 700,
  file 600), merge the marty block into the remote `.htaccess`. `--dry-run`.
- `cargo marty check <target> [<path>]`: one GET against the deployed alias.
- `cargo marty init <target>`: `marty.deploy.toml` skeleton, `deploy/<target>/marty.conf`,
  `.gitignore` entries.
- `cargo marty presets`: embedded hoster presets (`allinkl`, `generic`, `netcup`).
