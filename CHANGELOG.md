# Changelog

All notable changes to the **marty** library crate are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [SemVer](https://semver.org/) (`0.y.z`: `y` may include breaking API changes).

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
