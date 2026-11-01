//! Secrets and settings for CGI binaries: environment first, then one config file.
//!
//! Enable Cargo feature **`config`** (also pulled in by **`session`**). No extra dependencies.
//!
//! On suexec hosts (all-inkl, Hetzner, netcup) `.htaccess` `SetEnv` does not reach the CGI
//! process, so production settings live in a file **outside the document root**. Keys in
//! the file use the same names as the env vars, so one name covers both:
//!
//! ```text
//! # marty.conf — mode 600, never under the docroot
//! MARTY_SESSION_SECRET="9c1f…"      # at least 32 bytes
//! MARTY_MYSQL_USER=app
//! MARTY_MYSQL_PASSWORD="s3cr#t"     # quote values that contain ` #`, quotes, or spaces
//! ```
//!
//! ```rust,no_run
//! let cfg = marty::Config::load()?;
//! let user = cfg.require("MARTY_MYSQL_USER")?;   // env var wins, then the file
//! let host = cfg.get("MARTY_MYSQL_HOST").unwrap_or_else(|| "localhost".into());
//! # Ok::<(), marty::ConfigError>(())
//! ```
//!
//! ## Where the file is
//!
//! [`Config::load`] never looks in the cwd or next to the binary: on a shared host both are
//! `cgi-bin/` under the docroot. It reads, in order:
//!
//! 1. `MARTY_CONFIG` (runtime env): explicit path. For local runs; suexec drops it on the host.
//! 2. The path compiled in from `MARTY_BUILD_CONFIG_PATH` (build-time env, set by
//!    `cargo marty build/deploy` from the target's `config`). A relative path is resolved
//!    against the binary's directory, e.g. `../../private/marty.conf`.
//!
//! Neither set: env vars only.
//!
//! ## File format
//!
//! One `KEY=value` per line (spaces around `=` allowed, optional `export ` prefix).
//! `#` starts a comment at the beginning of a line or after whitespace in an unquoted value,
//! so `s3cr#t` stays intact. `"…"` supports `\"` and `\\`; `'…'` is literal. Anything else
//! (no `=`, unterminated quote, text after a closing quote) is an error with the line number.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Runtime env var with an explicit config file path (local runs).
pub const CONFIG_ENV: &str = "MARTY_CONFIG";

/// Path compiled in by `cargo marty` (`MARTY_BUILD_CONFIG_PATH` at build time).
const BUILD_PATH: Option<&str> = option_env!("MARTY_BUILD_CONFIG_PATH");

/// Values from the config file (if any), with env vars taking precedence on lookup.
#[derive(Debug, Clone, Default)]
pub struct Config {
    values: HashMap<String, String>,
    path: Option<PathBuf>,
    loaded: bool,
}

impl Config {
    /// Load from `MARTY_CONFIG`, else from the path compiled in by `cargo marty`.
    ///
    /// A missing file at the compiled-in path is not an error here; [`Self::require`] then
    /// reports the path. A missing `MARTY_CONFIG` file is an error (explicit request).
    /// On Unix, warns on stderr when the file is group/other-readable or lies under
    /// `DOCUMENT_ROOT`.
    pub fn load() -> Result<Self, ConfigError> {
        if let Some(p) = std::env::var_os(CONFIG_ENV).filter(|p| !p.is_empty()) {
            return Self::from_file(PathBuf::from(p));
        }
        match BUILD_PATH.filter(|p| !p.is_empty()) {
            Some(p) => {
                let path = resolve_build_path(p);
                match Self::from_file(&path) {
                    Err(ConfigError::Io { source, .. })
                        if source.kind() == io::ErrorKind::NotFound =>
                    {
                        Ok(Config {
                            path: Some(path),
                            ..Config::default()
                        })
                    }
                    other => other,
                }
            }
            None => Ok(Config::default()),
        }
    }

    /// Read and parse one file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref().to_path_buf();
        let text = fs::read_to_string(&path).map_err(|source| ConfigError::Io {
            path: path.clone(),
            source,
        })?;
        warn_if_exposed(&path);
        let values = parse_lines(&text).map_err(|(line, message)| ConfigError::Parse {
            path: Some(path.clone()),
            line,
            message,
        })?;
        Ok(Config {
            values,
            path: Some(path),
            loaded: true,
        })
    }

    /// Parse config text (no file).
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let values = parse_lines(text).map_err(|(line, message)| ConfigError::Parse {
            path: None,
            line,
            message,
        })?;
        Ok(Config {
            values,
            path: None,
            loaded: true,
        })
    }

    /// Env var `key` if set and non-empty, else `key` from the file.
    pub fn get(&self, key: &str) -> Option<String> {
        std::env::var(key)
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| self.file_value(key).map(str::to_string))
    }

    /// Like [`Self::get`], but a missing value is an error naming the key and the file.
    pub fn require(&self, key: &str) -> Result<String, ConfigError> {
        self.get(key).ok_or_else(|| ConfigError::Missing {
            key: key.to_string(),
            path: self.path.clone(),
            loaded: self.loaded,
        })
    }

    /// Raw value from the file only (no env lookup).
    pub fn file_value(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// The config file path (read, or expected but missing).
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

/// Error from [`Config`].
#[derive(Debug)]
pub enum ConfigError {
    /// The file could not be read.
    Io { path: PathBuf, source: io::Error },
    /// Malformed line.
    Parse {
        path: Option<PathBuf>,
        line: usize,
        message: String,
    },
    /// Neither env nor file provided the key.
    Missing {
        key: String,
        path: Option<PathBuf>,
        loaded: bool,
    },
    /// The value is present but unusable (e.g. a session secret that is too short).
    Invalid { key: String, reason: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io { path, source } => {
                write!(f, "cannot read config file {}: {source}", path.display())
            }
            ConfigError::Parse {
                path,
                line,
                message,
            } => match path {
                Some(p) => write!(f, "{}:{line}: {message}", p.display()),
                None => write!(f, "config line {line}: {message}"),
            },
            ConfigError::Missing { key, path, loaded } => {
                write!(f, "missing config {key}: set env {key} or add {key}=… to ")?;
                match (path, loaded) {
                    (Some(p), true) => write!(f, "{}", p.display()),
                    (Some(p), false) => write!(f, "{} (file not found)", p.display()),
                    (None, _) => write!(
                        f,
                        "a config file (none configured: set {CONFIG_ENV}, or build with `cargo marty` and a target `config`)"
                    ),
                }
            }
            ConfigError::Invalid { key, reason } => write!(f, "invalid config {key}: {reason}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn resolve_build_path(p: &str) -> PathBuf {
    let path = Path::new(p);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join(path)))
        .unwrap_or_else(|| path.to_path_buf())
}

fn parse_lines(text: &str) -> Result<HashMap<String, String>, (usize, String)> {
    let mut map = HashMap::new();
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line
            .strip_prefix("export ")
            .map(str::trim_start)
            .unwrap_or(line);
        let (key, rest) = line
            .split_once('=')
            .ok_or_else(|| (n, "expected KEY=value".to_string()))?;
        let key = key.trim();
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err((n, format!("invalid key {key:?} (use A-Z, 0-9, _)")));
        }
        let value = parse_value(rest.trim_start()).map_err(|m| (n, m))?;
        map.insert(key.to_string(), value);
    }
    Ok(map)
}

fn parse_value(s: &str) -> Result<String, String> {
    let (value, after) = match s.chars().next() {
        Some('"') => {
            let mut out = String::new();
            let mut chars = s[1..].char_indices();
            loop {
                match chars.next() {
                    None => return Err("unterminated \"".into()),
                    Some((i, '"')) => break (out, &s[1 + i + 1..]),
                    Some((_, '\\')) => match chars.next() {
                        Some((_, c @ ('"' | '\\'))) => out.push(c),
                        Some((_, c)) => {
                            out.push('\\');
                            out.push(c);
                        }
                        None => return Err("unterminated \"".into()),
                    },
                    Some((_, c)) => out.push(c),
                }
            }
        }
        Some('\'') => {
            let end = s[1..].find('\'').ok_or("unterminated '")?;
            (s[1..1 + end].to_string(), &s[1 + end + 1..])
        }
        _ => {
            // `#` after whitespace starts a comment; `s3cr#t` stays one value.
            let mut end = s.len();
            let bytes = s.as_bytes();
            for i in 0..bytes.len() {
                if bytes[i] == b'#' && i > 0 && bytes[i - 1].is_ascii_whitespace() {
                    end = i;
                    break;
                }
            }
            return Ok(s[..end].trim_end().to_string());
        }
    };
    let after = after.trim_start();
    if !after.is_empty() && !after.starts_with('#') {
        return Err(format!("unexpected text after closing quote: {after:?}"));
    }
    Ok(value)
}

#[cfg(unix)]
fn warn_if_exposed(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            eprintln!(
                "marty::config: warning: {} is readable by group/other (mode {mode:03o}); use chmod 600 if the host allows it",
                path.display()
            );
        }
    }
    let docroot = std::env::var_os("DOCUMENT_ROOT").map(PathBuf::from);
    if let (Some(root), Ok(file)) = (docroot, path.canonicalize()) {
        if let Ok(root) = root.canonicalize() {
            if file.starts_with(&root) {
                eprintln!(
                    "marty::config: warning: {} is under DOCUMENT_ROOT {} and may be downloadable; move it outside the docroot",
                    path.display(),
                    root.display()
                );
            }
        }
    }
}

#[cfg(not(unix))]
fn warn_if_exposed(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("marty-config-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parses_values_comments_and_quotes() {
        let cfg = Config::parse(
            "# comment\n\
             HOST = localhost\n\
             USER=bob  # trailing\n\
             export PORT=3306\n\
             PLAIN=s3cr#t;x\n\
             DQ=\"s3cr #t \\\"q\\\" \\\\ \\n\"  # c\n\
             SQ='a \\ \"b\" #c'\n\
             EMPTY=\n",
        )
        .unwrap();
        assert_eq!(cfg.file_value("HOST"), Some("localhost"));
        assert_eq!(cfg.file_value("USER"), Some("bob"));
        assert_eq!(cfg.file_value("PORT"), Some("3306"));
        assert_eq!(cfg.file_value("PLAIN"), Some("s3cr#t;x"));
        assert_eq!(cfg.file_value("DQ"), Some("s3cr #t \"q\" \\ \\n"));
        assert_eq!(cfg.file_value("SQ"), Some("a \\ \"b\" #c"));
        assert_eq!(cfg.file_value("EMPTY"), Some(""));
    }

    #[test]
    fn rejects_malformed_lines_with_line_number() {
        for (text, line, needle) in [
            ("A=1\nnot a pair\n", 2, "expected KEY=value"),
            ("A=\"open\n", 1, "unterminated"),
            ("A='x' y\n", 1, "after closing quote"),
            ("bad key=1\n", 1, "invalid key"),
        ] {
            let err = Config::parse(text).unwrap_err().to_string();
            assert!(err.contains(&format!("line {line}")), "{err}");
            assert!(err.contains(needle), "{err}");
        }
    }

    #[test]
    fn env_wins_over_file() {
        let cfg = Config::parse("MARTY_CONFIG_TEST_ENV_WINS=file\nMARTY_CONFIG_TEST_FILE=only\n")
            .unwrap();
        // SAFETY: unique keys, only touched by this test.
        unsafe { std::env::set_var("MARTY_CONFIG_TEST_ENV_WINS", "env") };
        assert_eq!(
            cfg.get("MARTY_CONFIG_TEST_ENV_WINS").as_deref(),
            Some("env")
        );
        assert_eq!(cfg.get("MARTY_CONFIG_TEST_FILE").as_deref(), Some("only"));
        unsafe { std::env::set_var("MARTY_CONFIG_TEST_ENV_WINS", "") };
        assert_eq!(
            cfg.get("MARTY_CONFIG_TEST_ENV_WINS").as_deref(),
            Some("file")
        );
    }

    #[test]
    fn from_file_reports_path_in_errors() {
        let dir = temp_dir("file");
        let p = dir.join("marty.conf");
        fs::write(&p, "A=1\nbroken\n").unwrap();
        let err = Config::from_file(&p).unwrap_err().to_string();
        assert!(err.starts_with(&format!("{}:2:", p.display())), "{err}");

        fs::write(&p, "A=1\n").unwrap();
        let cfg = Config::from_file(&p).unwrap();
        let err = cfg
            .require("MARTY_CONFIG_TEST_MISSING")
            .unwrap_err()
            .to_string();
        assert!(err.contains("MARTY_CONFIG_TEST_MISSING"), "{err}");
        assert!(err.contains(&p.display().to_string()), "{err}");
        assert!(!err.contains("not found"), "{err}");

        assert!(Config::from_file(dir.join("nope.conf")).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_without_any_file_mentions_how_to_configure() {
        let cfg = Config::default();
        let err = cfg
            .require("MARTY_CONFIG_TEST_MISSING")
            .unwrap_err()
            .to_string();
        assert!(err.contains(CONFIG_ENV), "{err}");
        let cfg = Config {
            path: Some("/x/marty.conf".into()),
            ..Config::default()
        };
        let err = cfg
            .require("MARTY_CONFIG_TEST_MISSING")
            .unwrap_err()
            .to_string();
        assert!(err.contains("/x/marty.conf (file not found)"), "{err}");
    }

    #[test]
    fn relative_build_path_resolves_next_to_exe() {
        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        assert_eq!(
            resolve_build_path("../../private/marty.conf"),
            exe_dir.join("../../private/marty.conf")
        );
        assert_eq!(
            resolve_build_path("/abs/marty.conf"),
            PathBuf::from("/abs/marty.conf")
        );
    }
}
