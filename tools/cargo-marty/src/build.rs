//! `cargo marty build`: zigbuild for Linux musl, ELF check, deploy bundle.

use crate::Result;
use cargo_metadata::{Metadata, MetadataCommand, Package};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const X86_64: &str = "x86_64-unknown-linux-musl";
pub const AARCH64: &str = "aarch64-unknown-linux-musl";

/// Staged upload: `target/deploy/<pkg>/{cgi-bin/<bin>, .htaccess}`.
#[derive(Debug)]
pub struct Bundle {
    pub binary_name: String,
    pub binary: PathBuf,
    pub htaccess: Option<PathBuf>,
}

/// What `deploy` / `check` need to know about the package without building it.
#[derive(Debug)]
pub struct PackageInfo {
    pub name: String,
    pub binary_name: String,
    /// `[package.metadata.marty] alias`
    pub alias: Option<String>,
    /// `[package.metadata.marty] cgi_dir` (default `cgi-bin`), as used by the generated rewrite.
    pub mount_cgi_dir: String,
}

pub fn triple(explicit: Option<&str>, arm: bool) -> Result<String> {
    match (explicit, arm) {
        (Some(t), false) if t == X86_64 || t == AARCH64 => Ok(t.to_string()),
        (Some(t), false) => {
            Err(format!("unsupported target {t} (use {X86_64} or {AARCH64})").into())
        }
        (Some(_), true) => Err("pass either --arm or --target, not both".into()),
        (None, true) => Ok(AARCH64.into()),
        (None, false) => Ok(X86_64.into()),
    }
}

pub fn metadata() -> Result<Metadata> {
    let mut cmd = MetadataCommand::new();
    cmd.no_deps();
    Ok(cmd.exec()?)
}

pub fn select_package<'m>(meta: &'m Metadata, name: Option<&str>) -> Result<&'m Package> {
    if let Some(name) = name {
        return meta
            .workspace_packages()
            .into_iter()
            .find(|p| p.name.as_str() == name)
            .ok_or_else(|| format!("package `{name}` is not a member of this workspace").into());
    }
    // `root_package()` needs the resolve graph (absent with --no-deps): match the nearest
    // Cargo.toml above cwd against the workspace members instead.
    if let Ok(cwd) = env::current_dir() {
        for dir in cwd.ancestors() {
            let manifest = dir.join("Cargo.toml");
            if let Some(p) = meta
                .workspace_packages()
                .into_iter()
                .find(|p| p.manifest_path.as_std_path() == manifest)
            {
                return Ok(p);
            }
        }
    }
    let names: Vec<&str> = meta
        .workspace_packages()
        .iter()
        .filter(|p| p.targets.iter().any(|t| t.is_bin()))
        .map(|p| p.name.as_str())
        .collect();
    match names.as_slice() {
        [] => Err("no binary package in this workspace".into()),
        [_] => select_package(meta, Some(names[0])),
        _ => Err(format!(
            "virtual workspace with several binaries; pass -p <package> (one of: {})",
            names.join(", ")
        )
        .into()),
    }
}

pub fn package_info(pkg: &Package) -> Result<PackageInfo> {
    let binary_name = pkg
        .targets
        .iter()
        .find(|t| t.is_bin())
        .map(|t| t.name.clone())
        .ok_or_else(|| format!("package `{}` has no binary target", pkg.name))?;
    let marty = pkg.metadata.get("marty");
    let field = |k: &str| {
        marty
            .and_then(|m| m.get(k))
            .and_then(|v| v.as_str())
            .map(str::to_string)
    };
    Ok(PackageInfo {
        name: pkg.name.to_string(),
        binary_name,
        alias: field("alias"),
        mount_cgi_dir: field("cgi_dir").unwrap_or_else(|| "cgi-bin".into()),
    })
}

fn cargo() -> String {
    env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}

fn on_path(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn ensure_toolchain(triple: &str) -> Result<()> {
    if !on_path("zig") {
        return Err(
            "zig not on PATH (required by cargo-zigbuild). Examples: brew install zig | apt install zig"
                .into(),
        );
    }
    if on_path("rustup") {
        let ok = Command::new("rustup")
            .args(["target", "add", triple])
            .stdout(Stdio::null())
            .status()?
            .success();
        if !ok {
            return Err(format!("rustup target add {triple} failed").into());
        }
    }
    // `cargo zigbuild --version` is rejected by its CLI; probe the binary directly.
    let have_zigbuild = on_path("cargo-zigbuild");
    if !have_zigbuild {
        eprintln!("Installing cargo-zigbuild into ~/.cargo/bin …");
        let ok = Command::new(cargo())
            .args(["install", "cargo-zigbuild", "--locked"])
            .status()?
            .success();
        if !ok {
            return Err("cargo install cargo-zigbuild failed".into());
        }
    }
    Ok(())
}

/// Build-time env read by `marty::Config` (`option_env!`): where the binary finds its config.
pub const BUILD_CONFIG_ENV: &str = "MARTY_BUILD_CONFIG_PATH";

/// Build `package` (or the current/only binary package) for `triple` and stage the bundle.
///
/// `config_path` is compiled into the binary via [`BUILD_CONFIG_ENV`]; `None` clears it, so
/// a value from the calling shell never leaks into a build.
pub fn build(package: Option<&str>, triple: &str, config_path: Option<&str>) -> Result<Bundle> {
    let meta = metadata()?;
    let pkg = select_package(&meta, package)?;
    let info = package_info(pkg)?;
    let target_dir = meta.target_directory.as_std_path();
    let out = zigbuild(
        meta.workspace_root.as_std_path(),
        &info.name,
        &info.binary_name,
        triple,
        target_dir,
        |cmd| match config_path {
            Some(p) => {
                let rel = if p.starts_with('/') {
                    ""
                } else {
                    " (relative to the binary)"
                };
                println!("config path compiled in: {p}{rel}");
                cmd.env(BUILD_CONFIG_ENV, p);
            }
            None => {
                cmd.env_remove(BUILD_CONFIG_ENV);
            }
        },
    )?;

    // Sibling `{package}.htaccess` from `marty::htaccess::write_next_to_binary*` (build.rs).
    let hta_src = out.with_file_name(format!("{}.htaccess", info.name));
    let deploy = target_dir.join("deploy").join(&info.name);
    let cgi = deploy.join("cgi-bin");
    fs::create_dir_all(&cgi)?;
    let binary = cgi.join(&info.binary_name);
    fs::copy(&out, &binary)?;
    set_mode(&binary, 0o755)?;
    let hta_dst = deploy.join(".htaccess");
    let htaccess = if hta_src.is_file() {
        fs::copy(&hta_src, &hta_dst)?;
        Some(hta_dst)
    } else {
        let _ = fs::remove_file(&hta_dst);
        None
    };

    println!("linux musl ok: {} ({triple})", out.display());
    match &htaccess {
        Some(_) => println!(
            "deploy bundle: {}  (cgi-bin/{} + .htaccess)",
            deploy.display(),
            info.binary_name
        ),
        None => println!(
            "deploy bundle: {}  (cgi-bin/{}; no .htaccess — package has no htaccess build.rs)",
            deploy.display(),
            info.binary_name
        ),
    }
    Ok(Bundle {
        binary_name: info.binary_name,
        binary,
        htaccess,
    })
}

/// `cargo zigbuild --release --target <triple>` for `package` in `manifest_dir`, then the
/// ELF check. `env` adjusts the command (build-time env vars). Returns the built binary.
fn zigbuild(
    manifest_dir: &Path,
    package: &str,
    binary_name: &str,
    triple: &str,
    target_dir: &Path,
    env: impl FnOnce(&mut Command),
) -> Result<PathBuf> {
    ensure_toolchain(triple)?;
    let mut cmd = Command::new(cargo());
    cmd.current_dir(manifest_dir).args([
        "zigbuild",
        "-p",
        package,
        "--release",
        "--target",
        triple,
    ]);
    cmd.arg("--target-dir").arg(target_dir);
    env(&mut cmd);
    let ok = cmd.status()?.success();
    if !ok {
        return Err(format!("cargo zigbuild -p {package} failed").into());
    }

    let candidates = [
        target_dir.join(triple).join("release"),
        target_dir.join("release"),
    ];
    let out = candidates
        .iter()
        .map(|d| d.join(binary_name))
        .find(|p| p.is_file())
        .ok_or_else(|| {
            format!(
                "binary not found under {} or {}",
                candidates[0].display(),
                candidates[1].display()
            )
        })?;
    check_elf(&out, triple)?;
    Ok(out)
}

/// Source of the hosting probe (`tools/hosting-probe` builds the same file).
const PROBE_RS: &str = include_str!("../probe/main.rs");
/// Perl variant of the probe, uploaded next to the binary.
pub const PROBE_PL: &str = include_str!("../probe/probe.pl");
const PROBE_PACKAGE: &str = "marty-hosting-probe";

const PROBE_MANIFEST: &str = r#"# Generated by cargo-marty for `cargo marty probe`; safe to delete.
[package]
name = "marty-hosting-probe"
version = "0.1.0"
edition = "2024"
publish = false

[workspace]

[profile.release]
lto = true
codegen-units = 1
strip = true
panic = "abort"
"#;

/// Build the std-only probe as its own crate under `<base>/marty-probe/` (stable path, so
/// repeated runs build incrementally). Works without the marty repo.
pub fn build_probe(base: &Path, triple: &str) -> Result<PathBuf> {
    let dir = base.join("marty-probe");
    fs::create_dir_all(dir.join("src"))?;
    write_if_changed(&dir.join("Cargo.toml"), PROBE_MANIFEST)?;
    write_if_changed(&dir.join("src/main.rs"), PROBE_RS)?;
    let out = zigbuild(
        &dir,
        PROBE_PACKAGE,
        PROBE_PACKAGE,
        triple,
        &dir.join("target"),
        |_| {},
    )?;
    println!("probe built: {} ({triple})", out.display());
    Ok(out)
}

/// Keep mtimes stable when nothing changed, so cargo does not rebuild.
fn write_if_changed(path: &Path, text: &str) -> Result<()> {
    if fs::read_to_string(path).ok().as_deref() != Some(text) {
        fs::write(path, text)?;
    }
    Ok(())
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

const EM_X86_64: u16 = 0x3e;
const EM_AARCH64: u16 = 0xb7;

/// `e_machine` of a 64-bit little-endian ELF, or a description of what the bytes look like.
fn elf_machine(head: &[u8]) -> std::result::Result<u16, String> {
    if head.len() < 20 {
        return Err("file too short for an ELF header".into());
    }
    if &head[..4] != b"\x7fELF" {
        let kind = match &head[..4] {
            [0xcf, 0xfa, 0xed, 0xfe] | [0xfe, 0xed, 0xfa, 0xcf] => "a Mach-O (macOS) binary",
            [0xca, 0xfe, 0xba, 0xbe] => "a universal Mach-O (macOS) binary",
            [b'M', b'Z', ..] => "a PE (Windows) binary",
            _ => "not an ELF file",
        };
        return Err(kind.into());
    }
    if head[4] != 2 || head[5] != 1 {
        return Err("ELF, but not 64-bit little-endian".into());
    }
    Ok(u16::from_le_bytes([head[18], head[19]]))
}

fn check_elf(path: &Path, triple: &str) -> Result<()> {
    let mut head = [0u8; 20];
    let n = {
        use std::io::Read;
        fs::File::open(path)?.read(&mut head)?
    };
    let (want, label) = match triple {
        X86_64 => (EM_X86_64, "x86-64"),
        AARCH64 => (EM_AARCH64, "aarch64"),
        other => return Err(format!("unsupported triple {other}").into()),
    };
    let got = match elf_machine(&head[..n]) {
        Ok(m) if m == want => return Ok(()),
        Ok(m) => format!("ELF for machine 0x{m:x}"),
        Err(kind) => kind,
    };
    Err(format!(
        "expected Linux {label} ELF for {triple}, got {got}: {}\n\
         hint: zig + cargo-zigbuild must cooperate; see https://github.com/rust-cross/cargo-zigbuild.\n\
         hint: Zig 0.16+ can still yield a macOS binary with some toolchains — try Zig 0.13.x–0.15.x if this persists.",
        path.display()
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elf(machine: u16) -> Vec<u8> {
        let mut h = vec![0x7f, b'E', b'L', b'F', 2, 1, 1, 0];
        h.resize(18, 0);
        h.extend_from_slice(&machine.to_le_bytes());
        h
    }

    #[test]
    fn elf_detection() {
        assert_eq!(elf_machine(&elf(EM_X86_64)), Ok(EM_X86_64));
        assert_eq!(elf_machine(&elf(EM_AARCH64)), Ok(EM_AARCH64));
        let mut macho = vec![0xcf, 0xfa, 0xed, 0xfe];
        macho.resize(20, 0);
        assert!(elf_machine(&macho).unwrap_err().contains("Mach-O"));
        assert!(elf_machine(b"short").is_err());
    }

    #[test]
    fn triple_selection() {
        assert_eq!(triple(None, false).unwrap(), X86_64);
        assert_eq!(triple(None, true).unwrap(), AARCH64);
        assert!(triple(Some("x86_64-apple-darwin"), false).is_err());
    }
}
