//! `marty.deploy.toml` targets and embedded hoster presets (data, not code).

use crate::Result;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEPLOY_FILE: &str = "marty.deploy.toml";
/// Local secrets live in `<LOCAL_CONFIG_DIR>/<target>/<file name of config>`, next to [`DEPLOY_FILE`].
pub const LOCAL_CONFIG_DIR: &str = "deploy";

/// `(name, toml)` — add a hoster by dropping a file into `presets/` and listing it here.
const PRESETS: &[(&str, &str)] = &[
    ("allinkl", include_str!("../presets/allinkl.toml")),
    ("generic", include_str!("../presets/generic.toml")),
    ("netcup", include_str!("../presets/netcup.toml")),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Sftp,
    Ftps,
    Ftp,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Protocol::Sftp => "sftp",
            Protocol::Ftps => "ftps",
            Protocol::Ftp => "ftp",
        })
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    pub protocol: Option<Protocol>,
    pub docroot: Option<String>,
    pub cgi_dir: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct TargetConfig {
    preset: Option<String>,
    host: Option<String>,
    user: Option<String>,
    port: Option<u16>,
    /// Public host name; `check` requests `https://<domain>/<alias>/`.
    domain: Option<String>,
    protocol: Option<Protocol>,
    docroot: Option<String>,
    cgi_dir: Option<String>,
    package: Option<String>,
    /// `x86_64` (default) or `aarch64`
    arch: Option<String>,
    /// Full URL for `check` instead of `https://<domain>/<alias>/` (rarely needed).
    url: Option<String>,
    /// Remote config file (login-relative, or absolute with `/`); compiled into the binary.
    config: Option<String>,
    /// Local file uploaded to `config` (relative to this deploy file).
    /// Default: `deploy/<target>/<file name of config>`.
    config_source: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeployFile {
    #[serde(default)]
    target: BTreeMap<String, TargetConfig>,
}

/// A resolved target: preset defaults applied, paths relative to the remote login dir.
#[derive(Debug)]
pub struct Target {
    pub name: String,
    pub host: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub domain: Option<String>,
    pub protocol: Protocol,
    pub docroot: String,
    pub cgi_dir: String,
    pub package: Option<String>,
    pub arm: bool,
    pub url: Option<String>,
    /// Remote config file, normalized; never inside `docroot` / `cgi_dir`.
    pub config: Option<String>,
    pub config_source: Option<PathBuf>,
    pub notes: Option<String>,
}

impl Target {
    pub fn host(&self) -> Result<&str> {
        self.host
            .as_deref()
            .ok_or_else(|| format!("target `{}`: host missing in {DEPLOY_FILE}", self.name).into())
    }

    pub fn user(&self) -> Result<&str> {
        self.user
            .as_deref()
            .ok_or_else(|| format!("target `{}`: user missing in {DEPLOY_FILE}", self.name).into())
    }

    /// Path compiled into the binary (`MARTY_BUILD_CONFIG_PATH`): `config` relative to
    /// `cgi_dir` (e.g. `../../private/marty.conf`), or `config` itself when absolute.
    pub fn build_config_path(&self) -> Option<String> {
        let config = self.config.as_deref()?;
        if config.starts_with('/') {
            return Some(config.to_string());
        }
        Some(relative_to(&self.cgi_dir, config))
    }
}

pub fn preset(name: &str) -> Result<Preset> {
    let (_, text) = PRESETS.iter().find(|(n, _)| *n == name).ok_or_else(|| {
        let names: Vec<&str> = PRESETS.iter().map(|(n, _)| *n).collect();
        format!("unknown preset `{name}` (available: {})", names.join(", "))
    })?;
    Ok(toml::from_str(text).map_err(|e| format!("preset {name}: {e}"))?)
}

pub fn print_presets() {
    for (name, _) in PRESETS {
        match preset(name) {
            Ok(p) => {
                println!("{name}");
                println!(
                    "  protocol = {}  docroot = {}  cgi_dir = {}",
                    p.protocol.unwrap_or(Protocol::Sftp),
                    p.docroot.as_deref().unwrap_or("(set per target)"),
                    p.cgi_dir.as_deref().unwrap_or("{docroot}/cgi-bin"),
                );
                if let Some(n) = p.notes {
                    println!("  {n}");
                }
            }
            Err(e) => println!("{name}: {e}"),
        }
    }
}

/// `marty.deploy.toml` in `cwd`, else in `workspace_root`.
pub fn find_deploy_file(cwd: &Path, workspace_root: &Path) -> Option<PathBuf> {
    [cwd, workspace_root]
        .iter()
        .map(|d| d.join(DEPLOY_FILE))
        .find(|p| p.is_file())
}

pub fn load(path: &Path, name: &str) -> Result<Target> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let base = path.parent().unwrap_or(Path::new("."));
    parse_target(&text, name, base).map_err(|e| format!("{}: {e}", path.display()).into())
}

pub fn parse_target(text: &str, name: &str, base: &Path) -> Result<Target> {
    let file: DeployFile = toml::from_str(text)?;
    let Some(cfg) = file.target.get(name) else {
        let names: Vec<&String> = file.target.keys().collect();
        return Err(format!(
            "no [target.{name}] (defined: {})",
            if names.is_empty() {
                "none".to_string()
            } else {
                names
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        )
        .into());
    };
    let preset = match &cfg.preset {
        Some(p) => preset(p)?,
        None => Preset::default(),
    };
    let docroot = cfg
        .docroot
        .clone()
        .or(preset.docroot)
        .ok_or_else(|| format!("target `{name}`: set docroot (no preset default)"))?;
    let docroot = normalize(&docroot);
    let cgi_dir = cfg
        .cgi_dir
        .clone()
        .or(preset.cgi_dir)
        .unwrap_or_else(|| "{docroot}/cgi-bin".into())
        .replace("{docroot}", &docroot);
    let cgi_dir = normalize(&cgi_dir);
    let arm = match cfg.arch.as_deref() {
        None | Some("x86_64") => false,
        Some("aarch64") => true,
        Some(other) => {
            return Err(
                format!("target `{name}`: arch must be x86_64 or aarch64, not {other}").into(),
            );
        }
    };
    let config = cfg.config.as_deref().map(normalize);
    for p in [Some(&docroot), Some(&cgi_dir), config.as_ref()]
        .into_iter()
        .flatten()
    {
        if p.contains('"') || p.contains('\n') {
            return Err(
                format!("target `{name}`: remote path {p:?} contains a quote or newline").into(),
            );
        }
        if p.split('/').any(|seg| seg == "..") {
            return Err(format!("target `{name}`: remote path {p:?} must not contain `..`").into());
        }
    }
    if let Some(c) = &config {
        check_config_location(name, c, &docroot, &cgi_dir)?;
    }
    if cfg.config_source.is_some() && config.is_none() {
        return Err(
            format!("target `{name}`: config_source needs config (the remote path)").into(),
        );
    }
    Ok(Target {
        name: name.to_string(),
        host: cfg.host.clone(),
        user: cfg.user.clone(),
        port: cfg.port,
        domain: cfg.domain.clone(),
        protocol: cfg.protocol.or(preset.protocol).unwrap_or(Protocol::Sftp),
        docroot,
        cgi_dir,
        package: cfg.package.clone(),
        arm,
        url: cfg.url.clone(),
        config_source: match (&cfg.config_source, &config) {
            (Some(f), _) => Some(base.join(f)),
            (None, Some(c)) => Some(local_config_path(base, name, c)),
            (None, None) => None,
        },
        config,
        notes: preset.notes,
    })
}

/// Convention: `deploy/<target>/<file name of config>`, so the local file is named like the
/// remote one.
pub fn local_config_path(base: &Path, target: &str, config: &str) -> PathBuf {
    let file = config.rsplit('/').next().unwrap_or(config);
    base.join(LOCAL_CONFIG_DIR).join(target).join(file)
}

/// The config must never be reachable over HTTP: not in the docroot, not in `cgi-bin`.
fn check_config_location(name: &str, config: &str, docroot: &str, cgi_dir: &str) -> Result<()> {
    if config.ends_with('/') || config == "." {
        return Err(format!(
            "target `{name}`: config must be a file path, e.g. private/marty.conf"
        )
        .into());
    }
    for (label, dir) in [("docroot", docroot), ("cgi_dir", cgi_dir)] {
        if config.starts_with('/') != dir.starts_with('/') {
            eprintln!(
                "warning: target `{name}`: cannot verify config {config} against {label} {dir} \
                 (one absolute, one relative); make sure it is outside the docroot"
            );
            continue;
        }
        if is_within(config, dir) {
            return Err(format!(
                "target `{name}`: config {config} is inside {label} {dir} and could be served over HTTP; \
                 put it next to the docroot, e.g. private/marty.conf"
            )
            .into());
        }
    }
    Ok(())
}

/// `path` equals `dir` or lies below it (both normalized, same kind). `.` contains everything.
fn is_within(path: &str, dir: &str) -> bool {
    if dir == "." {
        return true;
    }
    let p: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let d: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
    p.len() >= d.len() && p[..d.len()] == d[..]
}

/// Relative path from directory `from` to `to` (both login-relative, normalized).
fn relative_to(from: &str, to: &str) -> String {
    let f: Vec<&str> = from
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    let t: Vec<&str> = to
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    let common = f.iter().zip(&t).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<&str> = vec![".."; f.len() - common];
    parts.extend(&t[common..]);
    parts.join("/")
}

/// Drop `./` segments and duplicate slashes (`./cgi-bin` → `cgi-bin`).
fn normalize(path: &str) -> String {
    let parts: Vec<&str> = path
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    if parts.is_empty() {
        return if path.starts_with('/') {
            "/".into()
        } else {
            ".".into()
        };
    }
    let joined = parts.join("/");
    if path.starts_with('/') {
        format!("/{joined}")
    } else {
        joined
    }
}

/// Remote path `dir/file` (with `.` meaning the login dir).
pub fn join(dir: &str, file: &str) -> String {
    if dir == "." || dir.is_empty() {
        file.to_string()
    } else {
        format!("{}/{file}", dir.trim_end_matches('/'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_presets_parse() {
        for (name, _) in PRESETS {
            preset(name).unwrap();
        }
    }

    #[test]
    fn preset_defaults_and_overrides() {
        let text = r#"
[target.prod]
preset = "netcup"
host = "h"
user = "u"
domain = "example.org"
config = "private/marty.conf"

[target.other]
preset = "netcup"
docroot = "example.org/httpdocs"

[target.custom]
docroot = "./public_html/"
cgi_dir = "cgi-bin"
protocol = "ftps"
arch = "aarch64"
"#;
        let base = Path::new("/proj");
        let t = parse_target(text, "prod", base).unwrap();
        assert_eq!(t.protocol, Protocol::Sftp);
        assert_eq!(t.docroot, "httpdocs");
        assert_eq!(t.cgi_dir, "httpdocs/cgi-bin");
        assert_eq!(t.config.as_deref(), Some("private/marty.conf"));
        assert_eq!(
            t.config_source,
            Some(PathBuf::from("/proj/deploy/prod/marty.conf"))
        );
        assert_eq!(
            t.build_config_path().as_deref(),
            Some("../../private/marty.conf")
        );

        let t = parse_target(text, "other", base).unwrap();
        assert_eq!(t.cgi_dir, "example.org/httpdocs/cgi-bin");

        let t = parse_target(text, "custom", base).unwrap();
        assert_eq!(t.protocol, Protocol::Ftps);
        assert_eq!(t.docroot, "public_html");
        assert_eq!(t.cgi_dir, "cgi-bin");
        assert!(t.arm);

        let err = parse_target(text, "nope", base).unwrap_err().to_string();
        assert!(err.contains("custom, other, prod"), "{err}");
    }

    #[test]
    fn allinkl_layout() {
        let text = "[target.a]\npreset = \"allinkl\"\nconfig = \"_config/marty.conf\"\n";
        let t = parse_target(text, "a", Path::new(".")).unwrap();
        assert_eq!(t.protocol, Protocol::Ftps);
        assert_eq!(t.cgi_dir, "htdocs/cgi-bin");
        assert_eq!(
            t.build_config_path().as_deref(),
            Some("../../_config/marty.conf")
        );
    }

    #[test]
    fn generic_needs_docroot_and_rejects_unknown_keys() {
        let err = parse_target("[target.a]\npreset = \"generic\"\n", "a", Path::new("."))
            .unwrap_err()
            .to_string();
        assert!(err.contains("docroot"), "{err}");
        assert!(
            parse_target(
                "[target.a]\ndocroot = \"x\"\npassword = \"no\"\n",
                "a",
                Path::new(".")
            )
            .is_err()
        );
        let t = parse_target(
            "[target.a]\npreset = \"generic\"\ndocroot = \"public_html\"\n",
            "a",
            Path::new("."),
        )
        .unwrap();
        assert_eq!(t.cgi_dir, "public_html/cgi-bin");
    }

    #[test]
    fn config_never_inside_docroot_or_cgi_bin() {
        let t = |extra: &str| {
            parse_target(
                &format!("[target.a]\npreset = \"netcup\"\n{extra}"),
                "a",
                Path::new("."),
            )
        };
        for bad in [
            "httpdocs/marty.conf",
            "httpdocs/cgi-bin/marty.conf",
            "./httpdocs/private/marty.conf",
        ] {
            let err = t(&format!("config = \"{bad}\"\n")).unwrap_err().to_string();
            assert!(err.contains("could be served over HTTP"), "{bad}: {err}");
        }
        // Login dir == docroot: nowhere safe to put it.
        let err = parse_target(
            "[target.a]\ndocroot = \".\"\nconfig = \"private/marty.conf\"\n",
            "a",
            Path::new("."),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("inside docroot"), "{err}");
        assert!(t("config = \"../marty.conf\"\n").is_err());
        assert!(t("config_source = \"x\"\n").is_err());
        let explicit =
            t("config = \"private/app.conf\"\nconfig_source = \"secrets/a.conf\"\n").unwrap();
        assert_eq!(
            explicit.config_source,
            Some(PathBuf::from("./secrets/a.conf"))
        );
        // Sibling dir whose name starts like the docroot is fine.
        assert!(t("config = \"httpdocs-private/marty.conf\"\n").is_ok());

        let abs = t("config = \"/var/www/vhosts/x/private/marty.conf\"\n").unwrap();
        assert_eq!(
            abs.build_config_path().as_deref(),
            Some("/var/www/vhosts/x/private/marty.conf")
        );
    }

    #[test]
    fn relative_paths() {
        assert_eq!(
            relative_to("httpdocs/cgi-bin", "private/marty.conf"),
            "../../private/marty.conf"
        );
        assert_eq!(
            relative_to("example.de/cgi-bin", "_config/example.de/marty.conf"),
            "../../_config/example.de/marty.conf"
        );
        assert_eq!(
            relative_to("site/www/cgi-bin", "site/conf/marty.conf"),
            "../../conf/marty.conf"
        );
        assert_eq!(relative_to("cgi-bin", "marty.conf"), "../marty.conf");
    }

    #[test]
    fn local_config_named_like_remote() {
        assert_eq!(
            local_config_path(Path::new("/p"), "stage", "_config/example.de/app.conf"),
            PathBuf::from("/p/deploy/stage/app.conf")
        );
    }

    #[test]
    fn join_paths() {
        assert_eq!(join(".", ".htaccess"), ".htaccess");
        assert_eq!(join("httpdocs/", ".htaccess"), "httpdocs/.htaccess");
        assert_eq!(normalize("./a//b/"), "a/b");
    }
}
