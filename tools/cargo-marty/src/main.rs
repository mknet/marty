//! `cargo marty`: musl CGI bundle, upload, and a reachability check for shared hosting.
//!
//! Hosters differ in data (`marty.deploy.toml` + embedded presets), not in code.
//! Transport uses the system `sftp` and `curl` binaries.

mod build;
mod deploy;
mod target;
mod transport;

use std::env;
use std::process::ExitCode;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const USAGE: &str = "\
cargo marty — build and deploy Marty CGI binaries to shared hosting

USAGE:
    cargo marty build  [<target>] [-p <package>] [--arm | --target <triple>]
    cargo marty deploy <target> [-p <package>] [--arm] [--dry-run]
    cargo marty check  <target> [<path>]
    cargo marty init   <target> [--preset <name>] [--host <h>] [--user <u>] [--domain <d>]
                                [--protocol sftp|ftps|ftp] [--docroot <dir>] [--config <file>]
    cargo marty presets

build   cargo zigbuild for Linux musl, ELF check, bundle target/deploy/<pkg>/{cgi-bin/<bin>, .htaccess};
        with <target>: its package/arch, and its `config` path compiled into the binary
deploy  build <target>, upload binary (chmod 755) + deploy/<target>/<file> → config (chmod 600,
        outside the docroot), merge the marty block into .htaccess
check   one GET on https://<domain>/<alias>/<path>: status, latency, Server header
init    append [target.<name>] to marty.deploy.toml, create deploy/<name>/marty.conf (600),
        add both to .gitignore

Targets live in marty.deploy.toml (cwd, else workspace root). Passwords never go there:
SFTP uses your ssh config/agent (or MARTY_DEPLOY_PASSWORD), FTP(S) uses
MARTY_DEPLOY_PASSWORD or a prompt.
";

#[derive(Debug, Default)]
pub struct Args {
    pub positional: Vec<String>,
    pub package: Option<String>,
    pub triple: Option<String>,
    pub arm: bool,
    pub dry_run: bool,
    pub preset: Option<String>,
    pub host: Option<String>,
    pub user: Option<String>,
    pub domain: Option<String>,
    pub protocol: Option<String>,
    pub docroot: Option<String>,
    pub config: Option<String>,
    pub help: bool,
}

fn parse_args(raw: Vec<String>) -> Result<Args> {
    let mut args = Args::default();
    let mut it = raw.into_iter();
    while let Some(arg) = it.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };
        let mut value = |name: &str| -> Result<String> {
            inline
                .clone()
                .or_else(|| it.next())
                .ok_or_else(|| format!("{name} needs a value").into())
        };
        match flag.as_str() {
            "-p" | "--package" => args.package = Some(value(&flag)?),
            "--target" => args.triple = Some(value(&flag)?),
            "--preset" => args.preset = Some(value(&flag)?),
            "--host" => args.host = Some(value(&flag)?),
            "--user" => args.user = Some(value(&flag)?),
            "--domain" => args.domain = Some(value(&flag)?),
            "--protocol" => args.protocol = Some(value(&flag)?),
            "--docroot" => args.docroot = Some(value(&flag)?),
            "--config" => args.config = Some(value(&flag)?),
            "--arm" => args.arm = true,
            "--dry-run" => args.dry_run = true,
            "-h" | "--help" => args.help = true,
            f if f.starts_with('-') => return Err(format!("unknown flag {f}\n\n{USAGE}").into()),
            _ => args.positional.push(arg),
        }
    }
    Ok(args)
}

fn main() -> ExitCode {
    // ssh calls us back as SSH_ASKPASS when MARTY_DEPLOY_PASSWORD is set (see transport).
    if env::var_os(transport::ASKPASS_ENV).is_some() {
        println!("{}", env::var("MARTY_DEPLOY_PASSWORD").unwrap_or_default());
        return ExitCode::SUCCESS;
    }
    let mut raw: Vec<String> = env::args().skip(1).collect();
    // `cargo marty …` runs us as `cargo-marty marty …`.
    if raw.first().map(String::as_str) == Some("marty") {
        raw.remove(0);
    }
    match run(raw) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(raw: Vec<String>) -> Result<()> {
    if raw.is_empty() {
        print!("{USAGE}");
        return Ok(());
    }
    let cmd = raw[0].clone();
    let args = parse_args(raw[1..].to_vec())?;
    if args.help || matches!(cmd.as_str(), "-h" | "--help" | "help") {
        print!("{USAGE}");
        return Ok(());
    }
    match cmd.as_str() {
        "build" if !args.positional.is_empty() => deploy::build_for_target(&args),
        "build" => {
            let triple = build::triple(args.triple.as_deref(), args.arm)?;
            build::build(args.package.as_deref(), &triple, None).map(|_| ())
        }
        "deploy" => deploy::deploy(&args),
        "check" => deploy::check(&args),
        "init" => deploy::init(&args),
        "presets" => {
            target::print_presets();
            Ok(())
        }
        "-V" | "--version" => {
            println!("cargo-marty {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        other => Err(format!("unknown command `{other}`\n\n{USAGE}").into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn parses_flags_and_positionals() {
        let a = parse_args(s(&["prod", "-p", "app", "--dry-run", "--preset=netcup"])).unwrap();
        assert_eq!(a.positional, ["prod"]);
        assert_eq!(a.package.as_deref(), Some("app"));
        assert_eq!(a.preset.as_deref(), Some("netcup"));
        assert!(a.dry_run);
        assert!(parse_args(s(&["--bogus"])).is_err());
        assert!(parse_args(s(&["-p"])).is_err());
    }
}
