//! `build <target>`, `deploy`, `check`, `init`.

use crate::build::{self, PackageInfo};
use crate::target::{self, DEPLOY_FILE, LOCAL_CONFIG_DIR, Protocol, Target, join};
use crate::transport::{self, Op, Remote};
use crate::{Args, Result};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

fn target_name(args: &Args, cmd: &str) -> Result<String> {
    match args.positional.as_slice() {
        [name] => Ok(name.clone()),
        [name, _path] if cmd == "check" => Ok(name.clone()),
        [] => Err(format!("usage: cargo marty {cmd} <target>").into()),
        _ => Err(format!(
            "cargo marty {cmd} takes one target, got {:?}",
            args.positional
        )
        .into()),
    }
}

struct Context {
    target: Target,
    info: PackageInfo,
    package: String,
}

fn context(args: &Args, cmd: &str) -> Result<Context> {
    let name = target_name(args, cmd)?;
    let meta = build::metadata()?;
    let cwd = env::current_dir()?;
    let file = target::find_deploy_file(&cwd, meta.workspace_root.as_std_path()).ok_or_else(|| {
        format!(
            "no {DEPLOY_FILE} in {} or {}; create one with `cargo marty init {name} --preset <preset>`",
            cwd.display(),
            meta.workspace_root
        )
    })?;
    let target = target::load(&file, &name)?;
    let package = args.package.clone().or_else(|| target.package.clone());
    let pkg = build::select_package(&meta, package.as_deref())?;
    let info = build::package_info(pkg)?;
    Ok(Context {
        package: info.name.clone(),
        target,
        info,
    })
}

/// Remote dir of the `.htaccess` must see `cgi_dir` at the path the generated rewrite uses.
fn warn_on_mount_mismatch(t: &Target, info: &PackageInfo) {
    let rel = if t.docroot == "." {
        Some(t.cgi_dir.as_str())
    } else {
        t.cgi_dir.strip_prefix(&format!("{}/", t.docroot))
    };
    if rel != Some(info.mount_cgi_dir.trim_matches('/')) {
        eprintln!(
            "warning: cgi_dir {} is not {}/{} — the generated RewriteRule points at {}/{}; \
             set cgi_dir in [package.metadata.marty] or adjust the target",
            t.cgi_dir, t.docroot, info.mount_cgi_dir, info.mount_cgi_dir, info.binary_name
        );
    }
}

pub fn deploy(args: &Args) -> Result<()> {
    let Context {
        target: t,
        info,
        package,
    } = context(args, "deploy")?;
    let triple = build::triple(args.triple.as_deref(), args.arm || t.arm)?;
    if let Some(f) = &t.config_source {
        if !f.is_file() {
            return Err(format!(
                "local config {} not found; create it (chmod 600), e.g.\n  \
                 mkdir -p {dir} && cp marty.conf.example {file} && chmod 600 {file}\n\
                 or set config_source in {DEPLOY_FILE}",
                f.display(),
                dir = f
                    .parent()
                    .map(|d| d.display().to_string())
                    .unwrap_or_default(),
                file = f.display(),
            )
            .into());
        }
        warn_if_loose(f);
    }
    let config_path = t.build_config_path();
    let has_mount = info.alias.is_some();
    if has_mount {
        warn_on_mount_mismatch(&t, &info);
    }
    let remote_bin = join(&t.cgi_dir, &info.binary_name);
    let remote_hta = join(&t.docroot, ".htaccess");
    let dest = format!(
        "{}{} ({})",
        t.user
            .as_deref()
            .map(|u| format!("{u}@"))
            .unwrap_or_default(),
        t.host()?,
        t.protocol
    );

    if args.dry_run {
        println!("dry run: target `{}` → {dest}", t.name);
        println!("  build   cargo marty build -p {package} --target {triple}");
        match &config_path {
            Some(p) => println!("          {}={p}", build::BUILD_CONFIG_ENV),
            None => println!("          (no `config` → binary reads env vars only)"),
        }
        println!("  mkdir   {}", t.cgi_dir);
        println!(
            "  put     cgi-bin/{} → {remote_bin} (chmod 755, temp file + rename)",
            info.binary_name
        );
        if let (Some(src), Some(dst)) = (&t.config_source, &t.config) {
            if let Some(dir) = parent_dir(dst) {
                println!("  mkdir   {dir} (chmod 700)");
            }
            println!("  put     {} → {dst} (chmod 600)", src.display());
        }
        if has_mount {
            println!("  get     {remote_hta} (missing is fine)");
            println!(
                "  merge   replace `# BEGIN marty {0}` … `# END marty {0}` (or prepend it), keep other rules",
                info.binary_name
            );
            println!("  put     {remote_hta} (chmod 644)");
        } else {
            println!("  (no [package.metadata.marty] alias → no .htaccess change)");
        }
        if let Some(n) = &t.notes {
            println!("note: {n}");
        }
        return Ok(());
    }

    let bundle = build::build(Some(&package), &triple, config_path.as_deref())?;
    let remote = Remote::connect(&t)?;

    let mut ops = vec![
        Op::Mkdir(t.cgi_dir.clone()),
        Op::Upload {
            local: bundle.binary.clone(),
            remote: remote_bin.clone(),
            mode: 0o755,
            swap: true,
        },
    ];
    if let (Some(src), Some(dst)) = (&t.config_source, &t.config) {
        if let Some(dir) = parent_dir(dst) {
            ops.push(Op::Mkdir(dir.to_string()));
            ops.push(Op::Chmod {
                remote: dir.to_string(),
                mode: 0o700,
            });
        }
        ops.push(Op::Upload {
            local: src.clone(),
            remote: dst.clone(),
            mode: 0o600,
            swap: false,
        });
    }

    let work = transport::temp_dir()?;
    let result = (|| -> Result<()> {
        if let Some(block_path) = &bundle.htaccess {
            let current = work.join("htaccess.remote");
            let existing = if remote.download(&remote_hta, &current)? {
                fs::read_to_string(&current)?
            } else {
                String::new()
            };
            let block = fs::read_to_string(block_path)?;
            let merged = marty::htaccess::merge_block(&existing, &block);
            if merged != existing {
                let local = work.join("htaccess.merged");
                fs::write(&local, &merged)?;
                ops.push(Op::Upload {
                    local,
                    remote: remote_hta.clone(),
                    mode: 0o644,
                    swap: false,
                });
            } else {
                println!("{remote_hta}: marty block already current");
            }
        }
        for op in &ops {
            println!("  {op}");
        }
        remote.apply(&ops)
    })();
    let _ = fs::remove_dir_all(&work);
    result?;

    println!("deployed {} to {} ({dest})", bundle.binary_name, t.name);
    if t.url.is_some() || t.domain.is_some() {
        println!("next: cargo marty check {}", t.name);
    }
    Ok(())
}

#[cfg(unix)]
fn warn_if_loose(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            eprintln!(
                "warning: {} is readable by group/other (mode {mode:03o}); chmod 600 it",
                path.display()
            );
        }
    }
}

#[cfg(not(unix))]
fn warn_if_loose(_path: &Path) {}

/// `cargo marty build <target>`: like `build`, with the target's package, arch, and config path.
pub fn build_for_target(args: &Args) -> Result<()> {
    let Context {
        target: t, package, ..
    } = context(args, "build")?;
    let triple = build::triple(args.triple.as_deref(), args.arm || t.arm)?;
    build::build(Some(&package), &triple, t.build_config_path().as_deref()).map(|_| ())
}

/// Parent directory of a remote file path; `None` for the login dir itself.
fn parent_dir(path: &str) -> Option<&str> {
    match path.rsplit_once('/') {
        Some(("", _)) | None => None,
        Some((dir, _)) => Some(dir),
    }
}

pub fn check(args: &Args) -> Result<()> {
    let Context {
        target: t, info, ..
    } = context(args, "check")?;
    let url = check_url(&t, &info, args.positional.get(1).map(String::as_str))?;
    let start = Instant::now();
    let out = Command::new("curl")
        .args(["-sS", "-o", "/dev/null", "-D", "-", "--max-time", "30"])
        .args([
            "-w",
            "\n__marty__ %{http_code} %{time_starttransfer} %{time_total}\n",
        ])
        .arg(&url)
        .output()
        .map_err(|e| format!("cannot run curl: {e}"))?;
    let elapsed = start.elapsed();
    if !out.status.success() {
        return Err(format!(
            "GET {url} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )
        .into());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let (status, ttfb, total) =
        parse_curl_check(&text).ok_or_else(|| format!("GET {url}: unexpected curl output"))?;
    let server = text
        .lines()
        .filter_map(|l| l.split_once(':'))
        .filter(|(k, _)| k.trim().eq_ignore_ascii_case("server"))
        .map(|(_, v)| v.trim().to_string())
        .next_back()
        .unwrap_or_else(|| "(none)".into());
    println!(
        "GET {url} → {status} in {:.0} ms (TTFB {:.0} ms, wall {:.0} ms), Server: {server}",
        total * 1000.0,
        ttfb * 1000.0,
        elapsed.as_secs_f64() * 1000.0
    );
    if (200..400).contains(&status) {
        Ok(())
    } else {
        Err(format!("unexpected status {status}").into())
    }
}

/// `url` if set, else `https://<domain>/<alias>/` (or `/<cgi_dir>/<bin>/` without alias),
/// with an optional extra path such as `/visits`.
fn check_url(t: &Target, info: &PackageInfo, path: Option<&str>) -> Result<String> {
    let base = match (&t.url, &t.domain) {
        (Some(u), _) => u.trim_end_matches('/').to_string(),
        (None, Some(d)) => {
            let d = d.trim_end_matches('/');
            let d = if d.contains("://") {
                d.to_string()
            } else {
                format!("https://{d}")
            };
            match &info.alias {
                Some(alias) => format!("{d}/{alias}"),
                None => format!("{d}/{}/{}", info.mount_cgi_dir, info.binary_name),
            }
        }
        (None, None) => {
            return Err(format!(
                "target `{}`: set domain = \"example.org\" in {DEPLOY_FILE}",
                t.name
            )
            .into());
        }
    };
    Ok(match path.map(|p| p.trim_start_matches('/')) {
        Some(p) if !p.is_empty() => format!("{base}/{p}"),
        _ => format!("{base}/"),
    })
}

fn parse_curl_check(text: &str) -> Option<(u16, f64, f64)> {
    let line = text
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("__marty__ "))?;
    let mut parts = line.split_whitespace();
    Some((
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ))
}

pub fn init(args: &Args) -> Result<()> {
    let name = target_name(args, "init")?;
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("target name `{name}`: use letters, digits, - or _").into());
    }
    let preset_name = args.preset.clone().unwrap_or_else(|| "generic".into());
    let preset = target::preset(&preset_name)?;
    let path = PathBuf::from(DEPLOY_FILE);
    let existing = fs::read_to_string(&path).unwrap_or_default();
    if existing
        .lines()
        .any(|l| l.trim() == format!("[target.{name}]"))
    {
        return Err(format!("{DEPLOY_FILE} already has [target.{name}]").into());
    }
    let docroot = preset.docroot.as_deref();
    let mut out = String::new();
    if existing.is_empty() {
        out.push_str(
            "# cargo marty deploy targets. Keep this file out of git; no passwords here\n\
             # (SFTP: ssh keys/agent or MARTY_DEPLOY_PASSWORD; FTP(S): MARTY_DEPLOY_PASSWORD or prompt).\n",
        );
    } else if !existing.ends_with("\n\n") {
        out.push('\n');
    }
    let _ = writeln!(out, "\n[target.{name}]");
    let _ = writeln!(out, "preset = \"{preset_name}\"");
    let _ = writeln!(out, "host = \"{}\"", args.host.as_deref().unwrap_or("HOST"));
    let _ = writeln!(out, "user = \"{}\"", args.user.as_deref().unwrap_or("USER"));
    let _ = writeln!(
        out,
        "domain = \"{}\"   # `cargo marty check` requests https://<domain>/<alias>/",
        args.domain.as_deref().unwrap_or("example.org")
    );
    match args.protocol.as_deref() {
        Some(p @ ("sftp" | "ftps" | "ftp")) => {
            let _ = writeln!(out, "protocol = \"{p}\"");
        }
        Some(other) => {
            return Err(format!("--protocol must be sftp, ftps or ftp, not {other}").into());
        }
        None => {
            let _ = writeln!(
                out,
                "# protocol = \"{}\"   # sftp | ftps | ftp",
                preset.protocol.unwrap_or(Protocol::Sftp)
            );
        }
    }
    match (args.docroot.as_deref(), docroot) {
        (Some(d), _) => {
            let _ = writeln!(out, "docroot = \"{d}\"");
        }
        (None, Some(d)) => {
            let _ = writeln!(out, "# docroot = \"{d}\"   # preset default");
        }
        (None, None) => {
            let _ = writeln!(
                out,
                "docroot = \"public_html\"   # required for this preset: the folder the domain points to"
            );
        }
    }
    let _ = writeln!(out, "# cgi_dir = \"{{docroot}}/cgi-bin\"");
    let _ = writeln!(
        out,
        "# package = \"my-cgi\"   # default: -p or the current package"
    );
    let config = args.config.as_deref().unwrap_or("private/marty.conf");
    let _ = writeln!(
        out,
        "config = \"{config}\"   # remote, outside the docroot; path compiled into the binary"
    );
    let _ = writeln!(
        out,
        "# config_source = \"…\"   # default: {LOCAL_CONFIG_DIR}/{name}/marty.conf (gitignored)"
    );
    // Same checks as deploy (e.g. config inside the docroot) before anything is written.
    target::parse_target(&format!("{existing}{out}"), &name, Path::new("."))
        .map_err(|e| format!("not written: {e}"))?;
    fs::write(&path, format!("{existing}{out}"))?;
    println!("wrote [target.{name}] to {}", path.display());

    let local = target::local_config_path(Path::new("."), &name, config);
    create_local_config(&local)?;
    ensure_gitignored(&name)?;
    if let Some(n) = preset.notes {
        println!("note: {n}");
    }
    Ok(())
}

const CONF_TEMPLATE: &str = "\
# Secrets for this target. Keys are env var names (marty::Config), e.g.:
# MARTY_SESSION_SECRET=\"…\"   # openssl rand -hex 32
# MARTY_MYSQL_PASSWORD=\"…\"
# Uploaded by `cargo marty deploy` outside the docroot with chmod 600. Never commit it.
";

/// `deploy/<target>/marty.conf` (dir 700, file 600) from `marty.conf.example` if present.
fn create_local_config(path: &Path) -> Result<()> {
    if path.exists() {
        println!("kept existing {}", path.display());
        return Ok(());
    }
    let dir = path.parent().expect("deploy/<target>");
    fs::create_dir_all(dir)?;
    let example = Path::new("marty.conf.example");
    let text = match fs::read_to_string(example) {
        Ok(t) => t,
        Err(_) => CONF_TEMPLATE.to_string(),
    };
    fs::write(path, text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    let from = if example.is_file() {
        " from marty.conf.example"
    } else {
        ""
    };
    println!(
        "created {} (chmod 600){from}: fill in the secrets",
        path.display()
    );
    Ok(())
}

/// Make sure `marty.deploy.toml` and `deploy/` are ignored by git (any `.gitignore` up the
/// tree counts); append to `./.gitignore` only when they are not.
fn ensure_gitignored(name: &str) -> Result<()> {
    let probe = format!("{LOCAL_CONFIG_DIR}/{name}/marty.conf");
    let ignored = |p: &str| {
        Command::new("git")
            .args(["check-ignore", "-q", p])
            .status()
            .ok()
            .and_then(|s| s.code())
    };
    let (toml, conf) = (ignored(DEPLOY_FILE), ignored(&probe));
    if toml == Some(0) && conf == Some(0) {
        return Ok(());
    }
    if toml != Some(1) && conf != Some(1) {
        // 128: not a git checkout (or git missing).
        println!("hint: keep {DEPLOY_FILE} and {LOCAL_CONFIG_DIR}/ out of version control");
        return Ok(());
    }
    let gi = Path::new(".gitignore");
    let mut text = fs::read_to_string(gi).unwrap_or_default();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    let mut add = Vec::new();
    if toml == Some(1) {
        add.push(DEPLOY_FILE.to_string());
    }
    if conf == Some(1) {
        add.push(format!("/{LOCAL_CONFIG_DIR}/"));
    }
    text.push_str("# cargo marty: deploy targets and local secrets\n");
    for a in &add {
        text.push_str(a);
        text.push('\n');
    }
    fs::write(gi, text)?;
    println!("added {} to {}", add.join(", "), gi.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_url_from_domain_and_alias() {
        let t = target::parse_target(
            "[target.a]\npreset = \"netcup\"\ndomain = \"marty.example.net\"\n",
            "a",
            Path::new("."),
        )
        .unwrap();
        let info = PackageInfo {
            name: "app".into(),
            binary_name: "app".into(),
            alias: Some("playground".into()),
            mount_cgi_dir: "cgi-bin".into(),
        };
        assert_eq!(
            check_url(&t, &info, None).unwrap(),
            "https://marty.example.net/playground/"
        );
        assert_eq!(
            check_url(&t, &info, Some("/visits")).unwrap(),
            "https://marty.example.net/playground/visits"
        );
        let no_alias = PackageInfo {
            alias: None,
            ..info
        };
        assert_eq!(
            check_url(&t, &no_alias, None).unwrap(),
            "https://marty.example.net/cgi-bin/app/"
        );
    }

    #[test]
    fn parses_curl_write_out() {
        let text = "HTTP/2 200\r\nserver: nginx\r\n\r\n\n__marty__ 200 0.050 0.051\n";
        assert_eq!(parse_curl_check(text), Some((200, 0.05, 0.051)));
    }
}
