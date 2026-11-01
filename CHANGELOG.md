# Changelog

All notable changes to the **marty** library crate are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [SemVer](https://semver.org/) (`0.y.z`: `y` may include breaking API changes).

## [0.2.1] — 2026-10-31

### Added

- **`config`** feature: `marty::Config` reads env vars first, then one
  `KEY=value` file whose keys are the env var names (`cfg.require("MARTY_MYSQL_USER")`).
  Made for suexec hosts, where `.htaccess` `SetEnv` never reaches the CGI.
  `Config::load()` reads `MARTY_CONFIG` (local), else the path compiled in from
  `MARTY_BUILD_CONFIG_PATH` (set by `cargo marty`, relative to the binary). It
  never searches the cwd or the binary's directory (`cgi-bin/` under the docroot).
  Quoted values (`"s3cr #t"`), `#` comments; malformed lines are errors with a
  line number. Warns on stderr when the file is group/other-readable or under
  `DOCUMENT_ROOT`.
- `SessionStore::from_config`: secret from `MARTY_SESSION_SECRET`; missing or
  shorter than 32 bytes is an error.
- `htaccess::merge_block`: replace the marty block inside an existing
  `.htaccess` or prepend it, leaving CMS/PHP rules untouched.
- [`cargo-marty`](https://github.com/mknet/marty/tree/main/tools/cargo-marty) (separate crate): `cargo marty build | deploy | check | init`.

### Changed

- **`session` now enables `config`.**
- **Session cookie format** is now `v1.<exp>.<payload>.<sig>`: the expiry from
  `max_age` is signed and checked on `load`, so a copied cookie stops working
  at `exp` instead of verifying forever. Cookies issued by 0.2.0 no longer
  verify; users get a fresh, empty session once. `max_age(None)` signs
  `exp = 0` (no expiry).
- `htaccess::render` output is wrapped in `# BEGIN marty <binary>` /
  `# END marty <binary>` marker lines.
- Example `03_session` no longer falls back to a public demo secret; without a
  secret it answers 500.
- Example `07_mysql` uses `marty::Config`: `marty_mysql.conf` with keys
  `host`, `user`, … becomes one config file with `MARTY_MYSQL_HOST`,
  `MARTY_MYSQL_USER`, …, outside the docroot. A conf in `cgi-bin/` is no longer read.

## [0.2.0] — 2026-10-29

First feature release after the thin `0.1.x` crates.io stubs. Sync CGI is the
recommended path; Axum/`tower` remains optional.

### Added

- **`sync-cgi`**: sync `Router` with path params (`{name}`, `{*catch_all}`),
  `nest` / `merge`, fallback, and `serve()` without Tokio.
- Sync **query / form / cookie** helpers on CGI requests.
- **`session`**: HMAC-signed cookie sessions (`SessionStore`) for
  process-per-request CGI.
- **`htaccess`**: build-time Apache mount generator from
  `[package.metadata.marty]` (`write_next_to_binary_from_manifest`, etc.).
- Crates.io-oriented README (quick start, deploy notes, RustLab 2026 talk link).
- `LICENSE-APACHE` shipped inside the published package.

### Changed

- **Default features**: `default = ["hyper-cgi"]` (maps CGI ↔ `hyper` types).
  Prefer `default-features = false, features = ["sync-cgi"]` for the smallest
  shared-hosting binary.
- Crate **description** rewritten for discoverability (was BTTF-only blurb).
- Docs and examples emphasize sync-first DX; several examples no longer require
  `tower` for cookies/sessions/SQLite.

### Migration from 0.1.1

```toml
# Recommended (shared hosting):
marty = { version = "0.2", default-features = false, features = ["sync-cgi"] }

# Previous Axum-style entry:
marty = { version = "0.2", features = ["tower"] }
```

`0.1.1` on crates.io was a minimal surface (`tower` optional, empty default
feature set). Treat `0.2.0` as the first usable library release.

## [0.1.1] — 2026-05-13

- Early crates.io publish (hyper CGI helpers / optional tower).

## [0.1.0] — 2026-05-13

- Initial crates.io placeholder.

[0.2.0]: https://github.com/mknet/marty/releases/tag/v0.2.0
[0.1.1]: https://crates.io/crates/marty/0.1.1
[0.1.0]: https://crates.io/crates/marty/0.1.0
