//! Generate Apache `.htaccess` rewrite rules for CGI **mounts** (pretty URL → binary).
//!
//! This covers the public alias (`/routing/…` → `cgi-bin/myapp/…`), not individual
//! [`crate::Router`] patterns — those arrive as `PATH_INFO` after the rewrite.
//!
//! Enable Cargo feature **`htaccess`**. Full consumer `Cargo.toml` + musl upload notes:
//! crate README *Quick start*, and `examples/02_routing/README.md`.
//!
//! ```toml
//! [package.metadata.marty]
//! alias = "app"
//! path_info = true   # optional; default false
//! ```
//!
//! ```rust,ignore
//! // build-dependencies: marty = { …, default-features = false, features = ["htaccess"] }
//! fn main() {
//!     marty::htaccess::write_next_to_binary_from_manifest().unwrap();
//! }
//! ```
//!
//! Reads **`CARGO_PKG_NAME`** (binary) and **`[package.metadata.marty]`** (alias), writes
//! `{package}.htaccess` beside the linked binary.
//!
//! [`render`] wraps its output in `# BEGIN marty <binary>` / `# END marty <binary>` lines.
//! [`merge_block`] uses them to replace that block inside an existing `.htaccess`
//! (CMS / PHP rules elsewhere in the file stay as they are).

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// One pretty-URL mount of a CGI binary under the site document root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mount<'a> {
    /// Public path segment without slashes (e.g. `"routing"` → `/routing`, `/routing/…`).
    pub alias: &'a str,
    /// Binary file name under [`Self::cgi_dir`] (e.g. `"marty-02-routing"`).
    pub binary: &'a str,
    /// CGI directory relative to the `.htaccess` location (default `"cgi-bin"`).
    pub cgi_dir: &'a str,
    /// When true, emit a subpath rule so `/alias/foo` becomes `cgi-dir/binary/foo` (`PATH_INFO`).
    pub path_info: bool,
}

impl<'a> Mount<'a> {
    /// Mount `alias` → `{cgi-bin}/{binary}` with default `cgi_dir` and no subpath rule.
    pub fn new(alias: &'a str, binary: &'a str) -> Self {
        Self {
            alias,
            binary,
            cgi_dir: "cgi-bin",
            path_info: false,
        }
    }

    pub fn cgi_dir(mut self, cgi_dir: &'a str) -> Self {
        self.cgi_dir = cgi_dir;
        self
    }

    pub fn path_info(mut self, on: bool) -> Self {
        self.path_info = on;
        self
    }
}

/// Owned mount (e.g. parsed from `Cargo.toml`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedMount {
    pub alias: String,
    pub binary: String,
    pub cgi_dir: String,
    pub path_info: bool,
}

impl OwnedMount {
    pub fn as_mount(&self) -> Mount<'_> {
        Mount {
            alias: self.alias.as_str(),
            binary: self.binary.as_str(),
            cgi_dir: self.cgi_dir.as_str(),
            path_info: self.path_info,
        }
    }
}

/// Only the `RewriteRule` lines (for merging into an existing `.htaccess`).
pub fn render_rules(mounts: &[Mount<'_>]) -> String {
    let mut out = String::new();
    for m in mounts {
        let target = format!("{}/{}", m.cgi_dir.trim_matches('/'), m.binary);
        if m.path_info {
            let _ = writeln!(
                out,
                "RewriteRule ^{alias}/(.+)$ {target}/$1 [L,PT]",
                alias = m.alias,
                target = target
            );
        }
        let _ = writeln!(
            out,
            "RewriteRule ^{alias}/?$ {target} [L,PT]",
            alias = m.alias,
            target = target
        );
    }
    out
}

const BEGIN_PREFIX: &str = "# BEGIN marty ";
const END_PREFIX: &str = "# END marty ";

/// Marker name for a set of mounts: the binary name, or binaries joined with `+`.
pub fn block_name(mounts: &[Mount<'_>]) -> String {
    if mounts.is_empty() {
        return "mounts".into();
    }
    mounts
        .iter()
        .map(|m| m.binary)
        .collect::<Vec<_>>()
        .join("+")
}

/// Full `.htaccess` body: comments + `<IfModule mod_rewrite.c>` wrapping [`render_rules`],
/// between `# BEGIN marty …` / `# END marty …` markers (see [`merge_block`]).
pub fn render(mounts: &[Mount<'_>]) -> String {
    let name = block_name(mounts);
    let mut out = String::new();
    let _ = writeln!(out, "{BEGIN_PREFIX}{name}");
    match mounts {
        [] => {
            out.push_str("# Apache: (no Marty CGI mounts)\n");
        }
        [m] => {
            let _ = writeln!(
                out,
                "# Apache: short URL /{alias} → CGI {cgi}/{bin} (same vhost).",
                alias = m.alias,
                cgi = m.cgi_dir,
                bin = m.binary
            );
            out.push_str("# Copy next to your site root (or merge into existing .htaccess).\n");
            if m.path_info {
                let _ = writeln!(
                    out,
                    "# Subpaths: /{alias}/… → same CGI with the tail as PATH_INFO.",
                    alias = m.alias
                );
            }
            out.push_str(
                "# Requires: mod_rewrite, AllowOverride FileInfo (or All), Options +FollowSymLinks as needed.\n",
            );
            out.push_str("# Generated by marty::htaccess (e.g. from build.rs).\n");
        }
        _ => {
            out.push_str(
                "# Apache: Marty CGI mounts on one vhost (same DocumentRoot, binaries under cgi-bin/).\n",
            );
            out.push_str("# Merge into your .htaccess or use as a starting point.\n");
            out.push_str(
                "# Requires: mod_rewrite, AllowOverride FileInfo (or All), Options +FollowSymLinks as needed.\n",
            );
            out.push_str("# Generated by marty::htaccess.\n");
            out.push_str("#\n");
            out.push_str("# Short URLs (internal rewrite, [PT] passes to CGI).\n");
            out.push_str("# Full CGI URLs still work, e.g. /cgi-bin/<binary>/…\n");
        }
    }
    out.push('\n');
    out.push_str("<IfModule mod_rewrite.c>\n");
    out.push_str("RewriteEngine On\n");
    out.push_str(
        "# If the app lives in a subdirectory, set the prefix here, e.g. RewriteBase /myapp/\n",
    );
    out.push_str("#RewriteBase /\n");
    out.push_str(&render_rules(mounts));
    out.push_str("</IfModule>\n");

    if let [m] = mounts {
        let _ = writeln!(out);
        out.push_str("# Alternative: HTTP redirect (browser URL becomes /cgi-bin/…):\n");
        let _ = writeln!(
            out,
            "#RedirectMatch permanent ^/{alias}/?$ /{cgi}/{bin}",
            alias = m.alias,
            cgi = m.cgi_dir,
            bin = m.binary
        );
    }
    let _ = writeln!(out, "{END_PREFIX}{name}");
    out
}

/// Merge a marked block (output of [`render`]) into an existing `.htaccess`.
///
/// If `existing` already has a `# BEGIN marty <name>` … `# END marty <name>` block with the
/// same name, that block is replaced in place. Otherwise the block is **prepended**:
/// mod_rewrite applies rules in order, and a CMS catch-all (`RewriteRule . /index.php [L]`)
/// further up would otherwise swallow the mount. A block without markers is prepended as is.
/// Lines outside the block are kept byte for byte.
pub fn merge_block(existing: &str, block: &str) -> String {
    let mut block = block.to_string();
    if !block.ends_with('\n') {
        block.push('\n');
    }
    let name = block
        .lines()
        .find_map(|l| l.trim_end().strip_prefix(BEGIN_PREFIX))
        .map(str::to_string);

    let lines: Vec<&str> = existing.split_inclusive('\n').collect();
    if let Some(name) = name {
        let begin = format!("{BEGIN_PREFIX}{name}");
        let end = format!("{END_PREFIX}{name}");
        // First END, then the last BEGIN before it: a dangling BEGIN from a broken edit
        // further up must not pull user lines into the replaced range.
        let end_idx = lines.iter().position(|l| l.trim_end() == end);
        let begin_idx =
            end_idx.and_then(|e| lines[..e].iter().rposition(|l| l.trim_end() == begin));
        if let (Some(b), Some(e)) = (begin_idx, end_idx) {
            let mut out = String::with_capacity(existing.len() + block.len());
            out.extend(lines[..b].iter().copied());
            out.push_str(&block);
            out.extend(lines[e + 1..].iter().copied());
            return out;
        }
    }

    if existing.trim().is_empty() {
        return block;
    }
    let mut out = block;
    out.push('\n');
    out.push_str(existing);
    out
}

/// Write [`render`] output to `path`.
pub fn write(path: impl AsRef<Path>, mounts: &[Mount<'_>]) -> io::Result<()> {
    fs::write(path, render(mounts))
}

/// Write `{CARGO_PKG_NAME}.htaccess` next to the package binary under `target/…`.
///
/// Resolves the profile/target directory from Cargo’s `OUT_DIR`
/// (`…/build/<pkg>-<hash>/out` → three levels up). Intended for `build.rs` only.
///
/// Returns the path written.
pub fn write_next_to_binary(mounts: &[Mount<'_>]) -> io::Result<PathBuf> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "OUT_DIR unset (call write_next_to_binary from build.rs)",
        )
    })?);
    let pkg = env::var("CARGO_PKG_NAME").map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "CARGO_PKG_NAME unset (call write_next_to_binary from build.rs)",
        )
    })?;
    // OUT_DIR = <target>/<triple?>/<profile>/build/<pkg>-<hash>/out
    let dest_dir = out_dir
        .join("../../..")
        .canonicalize()
        .unwrap_or_else(|_| out_dir.join("../../.."));
    let path = dest_dir.join(format!("{pkg}.htaccess"));
    write(&path, mounts)?;
    Ok(path)
}

/// Like [`write_next_to_binary`], but binary = `CARGO_PKG_NAME` and alias / flags from
/// `[package.metadata.marty]` in this package’s `Cargo.toml`.
pub fn write_next_to_binary_from_manifest() -> io::Result<PathBuf> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "CARGO_MANIFEST_DIR unset (call from build.rs)",
        )
    })?;
    let cargo_toml = Path::new(&manifest_dir).join("Cargo.toml");
    println!("cargo:rerun-if-changed={}", cargo_toml.display());
    let owned = owned_mount_from_cargo_toml(&cargo_toml)?;
    // Prefer live package name from Cargo (matches the linked binary).
    let binary = env::var("CARGO_PKG_NAME").unwrap_or_else(|_| owned.binary.clone());
    let mount = Mount {
        alias: owned.alias.as_str(),
        binary: binary.as_str(),
        cgi_dir: owned.cgi_dir.as_str(),
        path_info: owned.path_info,
    };
    write_next_to_binary(&[mount])
}

/// Parse `[package] name` and `[package.metadata.marty]` from a `Cargo.toml` file.
pub fn owned_mount_from_cargo_toml(path: impl AsRef<Path>) -> io::Result<OwnedMount> {
    let text = fs::read_to_string(path.as_ref())?;
    let binary = package_name(&text).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: missing [package] name", path.as_ref().display()),
        )
    })?;
    let (alias, path_info, cgi_dir) = metadata_marty(&text).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{}: missing [package.metadata.marty] alias = \"…\"",
                path.as_ref().display()
            ),
        )
    })?;
    Ok(OwnedMount {
        alias,
        binary,
        cgi_dir,
        path_info,
    })
}

fn package_name(toml: &str) -> Option<String> {
    let mut in_package = false;
    for line in toml.lines() {
        let line = strip_comment(line);
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some(v) = toml_string_value(line, "name") {
            return Some(v);
        }
    }
    None
}

fn metadata_marty(toml: &str) -> Option<(String, bool, String)> {
    let mut in_section = false;
    let mut alias: Option<String> = None;
    let mut path_info = false;
    let mut cgi_dir = "cgi-bin".to_string();
    for line in toml.lines() {
        let line = strip_comment(line);
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            in_section = line == "[package.metadata.marty]";
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some(v) = toml_string_value(line, "alias") {
            alias = Some(v);
        } else if let Some(v) = toml_bool_value(line, "path_info") {
            path_info = v;
        } else if let Some(v) = toml_string_value(line, "cgi_dir") {
            cgi_dir = v;
        }
    }
    Some((alias?, path_info, cgi_dir))
}

fn strip_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or("").trim()
}

fn toml_string_value(line: &str, key: &str) -> Option<String> {
    let (k, v) = line.split_once('=')?;
    if k.trim() != key {
        return None;
    }
    let v = v.trim();
    let v = v
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| v.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))?;
    Some(v.to_string())
}

fn toml_bool_value(line: &str, key: &str) -> Option<bool> {
    let (k, v) = line.split_once('=')?;
    if k.trim() != key {
        return None;
    }
    match v.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_mount_single_rule() {
        let rules = render_rules(&[Mount::new("basic", "marty-01-basic")]);
        assert_eq!(
            rules,
            "RewriteRule ^basic/?$ cgi-bin/marty-01-basic [L,PT]\n"
        );
    }

    #[test]
    fn routing_mount_path_info_first() {
        let rules = render_rules(&[Mount::new("routing", "marty-02-routing").path_info(true)]);
        assert_eq!(
            rules,
            "\
RewriteRule ^routing/(.+)$ cgi-bin/marty-02-routing/$1 [L,PT]
RewriteRule ^routing/?$ cgi-bin/marty-02-routing [L,PT]
"
        );
    }

    #[test]
    fn combined_mounts() {
        let text = render(&[
            Mount::new("basic", "marty-01-basic"),
            Mount::new("routing", "marty-02-routing").path_info(true),
        ]);
        assert!(text.contains("<IfModule mod_rewrite.c>"));
        assert!(text.contains("RewriteRule ^basic/?$ cgi-bin/marty-01-basic [L,PT]"));
        assert!(text.contains("RewriteRule ^routing/(.+)$ cgi-bin/marty-02-routing/$1 [L,PT]"));
        assert!(text.contains("RewriteRule ^routing/?$ cgi-bin/marty-02-routing [L,PT]"));
    }

    #[test]
    fn render_has_markers() {
        let text = render(&[Mount::new("basic", "marty-01-basic")]);
        assert!(text.starts_with("# BEGIN marty marty-01-basic\n"), "{text}");
        assert!(text.ends_with("# END marty marty-01-basic\n"), "{text}");
        let multi = render(&[Mount::new("a", "bin-a"), Mount::new("b", "bin-b")]);
        assert!(multi.starts_with("# BEGIN marty bin-a+bin-b\n"));
    }

    #[test]
    fn merge_into_empty_file() {
        let block = render(&[Mount::new("basic", "marty-01-basic")]);
        assert_eq!(merge_block("", &block), block);
    }

    #[test]
    fn merge_prepends_before_cms_rules() {
        let cms =
            "# BEGIN WordPress\nRewriteEngine On\nRewriteRule . /index.php [L]\n# END WordPress\n";
        let block = render(&[Mount::new("basic", "marty-01-basic")]);
        let merged = merge_block(cms, &block);
        assert!(merged.starts_with("# BEGIN marty marty-01-basic\n"));
        assert!(merged.ends_with(cms));
        let marty_at = merged.find("cgi-bin/marty-01-basic").unwrap();
        assert!(marty_at < merged.find("index.php").unwrap());
    }

    #[test]
    fn merge_replaces_existing_block_in_place() {
        let old = render(&[Mount::new("old", "marty-01-basic")]);
        let existing = format!("Options -Indexes\n\n{old}\n# tail\nRewriteRule . /index.php [L]\n");
        let new = render(&[Mount::new("basic", "marty-01-basic")]);
        let merged = merge_block(&existing, &new);
        assert_eq!(
            merged,
            format!("Options -Indexes\n\n{new}\n# tail\nRewriteRule . /index.php [L]\n")
        );
        // Idempotent.
        assert_eq!(merge_block(&merged, &new), merged);
    }

    #[test]
    fn merge_keeps_other_marty_blocks() {
        let a = render(&[Mount::new("a", "bin-a")]);
        let b = render(&[Mount::new("b", "bin-b")]);
        let existing = format!("{a}\n{b}");
        let b2 = render(&[Mount::new("b2", "bin-b")]);
        let merged = merge_block(&existing, &b2);
        assert_eq!(merged, format!("{a}\n{b2}"));
    }

    #[test]
    fn merge_handles_crlf_and_dangling_begin() {
        let existing = "# BEGIN marty bin\r\nuser line\r\n# BEGIN marty bin\r\nold\r\n# END marty bin\r\nrest\r\n";
        let block = "# BEGIN marty bin\nnew\n# END marty bin\n";
        let merged = merge_block(existing, block);
        assert_eq!(
            merged,
            "# BEGIN marty bin\r\nuser line\r\n# BEGIN marty bin\nnew\n# END marty bin\nrest\r\n"
        );
    }

    #[test]
    fn parse_manifest_metadata() {
        let toml = r#"
[package]
name = "marty-02-routing"
version = "0.1.0"

[package.metadata.marty]
alias = "routing"
path_info = true
"#;
        assert_eq!(package_name(toml).as_deref(), Some("marty-02-routing"));
        let (alias, path_info, cgi) = metadata_marty(toml).unwrap();
        assert_eq!(alias, "routing");
        assert!(path_info);
        assert_eq!(cgi, "cgi-bin");
    }
}
