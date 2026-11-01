//! Remote operations over the system `sftp` (batch mode) or `curl` (FTP/FTPS).
//!
//! No SSH/FTP implementation of our own: keys, agent, `~/.ssh/config`, and curl's TLS
//! stack are used as installed.

use crate::Result;
use crate::target::{Protocol, Target};
use std::env;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Set when ssh runs `cargo-marty` as `SSH_ASKPASS` (prints `MARTY_DEPLOY_PASSWORD`).
pub const ASKPASS_ENV: &str = "CARGO_MARTY_ASKPASS";
const PASSWORD_ENV: &str = "MARTY_DEPLOY_PASSWORD";

#[derive(Debug)]
pub enum Op {
    /// Create `dir` and its parents (ignored when present).
    Mkdir(String),
    /// chmod a directory; a host that refuses it only gets a warning, not an abort.
    Chmod { remote: String, mode: u32 },
    /// Upload `local` to `remote` and chmod it. `swap` uploads to a temp name first and
    /// renames over the old file, so a running CGI never sees a half-written binary.
    Upload {
        local: PathBuf,
        remote: String,
        mode: u32,
        swap: bool,
    },
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Mkdir(d) => write!(f, "mkdir -p {d}"),
            Op::Chmod { remote, mode } => write!(f, "chmod {mode:o} {remote}"),
            Op::Upload {
                local,
                remote,
                mode,
                swap,
            } => {
                write!(f, "put {} → {remote} (chmod {mode:o}", local.display())?;
                if *swap {
                    write!(f, ", via temp file + rename")?;
                }
                write!(f, ")")
            }
        }
    }
}

pub struct Remote {
    protocol: Protocol,
    host: String,
    user: Option<String>,
    port: Option<u16>,
    password: Option<String>,
}

impl Remote {
    /// Resolve credentials. FTP(S) asks for a password when `MARTY_DEPLOY_PASSWORD` is unset;
    /// SFTP leaves auth to ssh (keys / agent / its own prompt).
    pub fn connect(target: &Target) -> Result<Self> {
        let host = target.host()?.to_string();
        let password = env::var(PASSWORD_ENV).ok().filter(|p| !p.is_empty());
        let password = match (target.protocol, password) {
            (Protocol::Sftp, p) => p,
            (_, Some(p)) => Some(p),
            (_, None) => Some(prompt_password(&format!(
                "{} password for {}@{host}: ",
                target.protocol,
                target.user()?
            ))?),
        };
        if target.protocol != Protocol::Sftp {
            target.user()?;
        }
        if target.protocol == Protocol::Ftp {
            eprintln!(
                "warning: plain FTP sends the password unencrypted; prefer protocol = \"ftps\" or \"sftp\""
            );
        }
        Ok(Remote {
            protocol: target.protocol,
            host,
            user: target.user.clone(),
            port: target.port,
            password,
        })
    }

    /// Fetch `remote` into `local`. `Ok(false)` when the remote file does not exist.
    pub fn download(&self, remote: &str, local: &Path) -> Result<bool> {
        let _ = fs::remove_file(local);
        match self.protocol {
            Protocol::Sftp => {
                let batch = format!("-get {} {}\n", quote(remote), quote_path(local));
                self.sftp(&batch)?;
                Ok(local.is_file())
            }
            Protocol::Ftp | Protocol::Ftps => {
                let mut cmd = self.curl_base();
                cmd.arg(self.ftp_url(remote)).arg("-o").arg(local);
                let (status, stderr) = self.run_curl(cmd)?;
                match status.code() {
                    Some(0) => Ok(true),
                    // 78: file not found; 9: directory missing (CWD refused). A real permission
                    // problem then surfaces on the upload, which creates the directory.
                    Some(78 | 9) => Ok(false),
                    _ => {
                        Err(format!("curl download of {remote} failed ({status}): {stderr}").into())
                    }
                }
            }
        }
    }

    pub fn apply(&self, ops: &[Op]) -> Result<()> {
        match self.protocol {
            Protocol::Sftp => self.sftp(&sftp_batch(ops)),
            Protocol::Ftp | Protocol::Ftps => {
                // Directories come from --ftp-create-dirs on upload, so chmod them afterwards.
                for op in ops {
                    if let Op::Upload {
                        local,
                        remote,
                        mode,
                        swap,
                    } = op
                    {
                        self.ftp_upload(local, remote, *mode, *swap)?;
                    }
                }
                for op in ops {
                    if let Op::Chmod { remote, mode } = op {
                        self.ftp_chmod(remote, *mode)?;
                    }
                }
                Ok(())
            }
        }
    }

    fn destination(&self) -> String {
        match &self.user {
            Some(u) => format!("{u}@{}", self.host),
            None => self.host.clone(),
        }
    }

    fn sftp(&self, batch: &str) -> Result<()> {
        let dir = temp_dir()?;
        let file = dir.join("batch");
        fs::write(&file, batch)?;
        let mut cmd = Command::new("sftp");
        // Before -b, so ssh may still prompt for a password on the tty.
        cmd.args(["-o", "BatchMode=no"]);
        if let Some(p) = self.port {
            cmd.arg("-P").arg(p.to_string());
        }
        cmd.arg("-b").arg(&file).arg(self.destination());
        if self.password.is_some() {
            cmd.env("SSH_ASKPASS", env::current_exe()?)
                .env("SSH_ASKPASS_REQUIRE", "force")
                .env(ASKPASS_ENV, "1");
        }
        let status = cmd.status().map_err(|e| format!("cannot run sftp: {e}"))?;
        let _ = fs::remove_dir_all(&dir);
        if !status.success() {
            return Err(format!("sftp to {} failed ({status})", self.destination()).into());
        }
        Ok(())
    }

    fn curl_base(&self) -> Command {
        let mut cmd = Command::new("curl");
        cmd.args(["-sS", "--fail", "-K", "-"]);
        if self.protocol == Protocol::Ftps {
            cmd.arg("--ssl-reqd");
        }
        cmd
    }

    /// Credentials go through `-K -` (stdin), not argv, so they stay out of `ps`.
    fn run_curl(&self, mut cmd: Command) -> Result<(std::process::ExitStatus, String)> {
        cmd.stdin(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| format!("cannot run curl: {e}"))?;
        let user = self.user.as_deref().unwrap_or("");
        let pass = self.password.as_deref().unwrap_or("");
        let config = format!("user = \"{}\"\n", curl_escape(&format!("{user}:{pass}")));
        child
            .stdin
            .take()
            .expect("piped stdin")
            .write_all(config.as_bytes())?;
        let out = child.wait_with_output()?;
        Ok((
            out.status,
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ))
    }

    fn ftp_url(&self, path: &str) -> String {
        let port = self.port.map(|p| format!(":{p}")).unwrap_or_default();
        // `ftp://host/a/b` is relative to the login dir; `%2F` makes it absolute.
        let (abs, rel) = match path.strip_prefix('/') {
            Some(rest) => ("%2F", rest),
            None => ("", path),
        };
        let encoded: Vec<String> = rel.split('/').map(url_encode).collect();
        format!("ftp://{}{port}/{abs}{}", self.host, encoded.join("/"))
    }

    fn ftp_chmod(&self, remote: &str, mode: u32) -> Result<()> {
        let mut cmd = self.curl_base();
        // Quote commands run relative to the login dir before the (harmless) listing.
        cmd.args(["--list-only", "-o", "/dev/null", "-Q"])
            .arg(format!("SITE CHMOD {mode:o} {remote}"))
            .arg(self.ftp_url(""));
        let (status, stderr) = self.run_curl(cmd)?;
        if !status.success() {
            eprintln!(
                "warning: chmod {mode:o} {remote} failed ({stderr}); check the directory mode by hand"
            );
        }
        Ok(())
    }

    fn ftp_upload(&self, local: &Path, remote: &str, mode: u32, swap: bool) -> Result<()> {
        let name = remote.rsplit('/').next().unwrap_or(remote);
        let mut cmd = self.curl_base();
        cmd.arg("--ftp-create-dirs").arg("-T").arg(local);
        // Post-transfer commands (`-` prefix) run in the upload's directory; `*` = ignore failure.
        let mut post = |c: String| {
            cmd.arg("-Q").arg(format!("-{c}"));
        };
        if swap {
            let tmp = format!("{name}.marty-upload");
            let old = format!("{name}.marty-old");
            post(format!("SITE CHMOD {mode:o} {tmp}"));
            post(format!("*DELE {old}"));
            post(format!("*RNFR {name}"));
            post(format!("*RNTO {old}"));
            post(format!("RNFR {tmp}"));
            post(format!("RNTO {name}"));
            post(format!("*DELE {old}"));
            cmd.arg(self.ftp_url(&swap_path(remote, &tmp)));
        } else {
            post(format!("SITE CHMOD {mode:o} {name}"));
            cmd.arg(self.ftp_url(remote));
        }
        let (status, stderr) = self.run_curl(cmd)?;
        if !status.success() {
            return Err(format!("curl upload to {remote} failed ({status}): {stderr}").into());
        }
        Ok(())
    }
}

fn swap_path(remote: &str, name: &str) -> String {
    match remote.rsplit_once('/') {
        Some((dir, _)) => format!("{dir}/{name}"),
        None => name.to_string(),
    }
}

/// sftp batch script for `ops`. Lines prefixed with `-` may fail without aborting.
pub fn sftp_batch(ops: &[Op]) -> String {
    let mut out = String::new();
    for op in ops {
        match op {
            Op::Mkdir(dir) => {
                let mut prefix = if dir.starts_with('/') {
                    "/".to_string()
                } else {
                    String::new()
                };
                for part in dir.split('/').filter(|p| !p.is_empty() && *p != ".") {
                    if !prefix.is_empty() && !prefix.ends_with('/') {
                        prefix.push('/');
                    }
                    prefix.push_str(part);
                    out.push_str(&format!("-mkdir {}\n", quote(&prefix)));
                }
            }
            Op::Chmod { remote, mode } => {
                out.push_str(&format!("-chmod {mode:o} {}\n", quote(remote)));
            }
            Op::Upload {
                local,
                remote,
                mode,
                swap: false,
            } => {
                out.push_str(&format!("put {} {}\n", quote_path(local), quote(remote)));
                out.push_str(&format!("chmod {mode:o} {}\n", quote(remote)));
            }
            Op::Upload {
                local,
                remote,
                mode,
                swap: true,
            } => {
                let tmp = format!("{remote}.marty-upload");
                let old = format!("{remote}.marty-old");
                out.push_str(&format!("put {} {}\n", quote_path(local), quote(&tmp)));
                out.push_str(&format!("chmod {mode:o} {}\n", quote(&tmp)));
                // Plain SFTP rename refuses to overwrite: move the old binary aside first.
                out.push_str(&format!("-rm {}\n", quote(&old)));
                out.push_str(&format!("-rename {} {}\n", quote(remote), quote(&old)));
                out.push_str(&format!("rename {} {}\n", quote(&tmp), quote(remote)));
                out.push_str(&format!("-rm {}\n", quote(&old)));
            }
        }
    }
    out
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn quote_path(p: &Path) -> String {
    quote(&p.to_string_lossy())
}

fn curl_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn temp_dir() -> Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let dir = env::temp_dir().join(format!("cargo-marty-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
    }
    Ok(dir)
}

/// Read a password from the terminal without echo (`stty -echo`).
fn prompt_password(prompt: &str) -> Result<String> {
    let tty = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|_| format!("no terminal for a password prompt; set {PASSWORD_ENV}"))?;
    let stty = |arg: &str| {
        Command::new("stty")
            .arg(arg)
            .stdin(tty.try_clone().map(Stdio::from).unwrap_or(Stdio::null()))
            .status()
    };
    let mut out = tty.try_clone()?;
    write!(out, "{prompt}")?;
    out.flush()?;
    let _ = stty("-echo");
    let mut line = String::new();
    let read =
        std::io::BufRead::read_line(&mut std::io::BufReader::new(tty.try_clone()?), &mut line);
    let _ = stty("echo");
    writeln!(out)?;
    read?;
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_for_swap_upload() {
        let ops = [
            Op::Mkdir("httpdocs/cgi-bin".into()),
            Op::Upload {
                local: "/tmp/app".into(),
                remote: "httpdocs/cgi-bin/app".into(),
                mode: 0o755,
                swap: true,
            },
            Op::Upload {
                local: "/tmp/marty.conf".into(),
                remote: "httpdocs/cgi-bin/marty.conf".into(),
                mode: 0o600,
                swap: false,
            },
        ];
        assert_eq!(
            sftp_batch(&ops),
            "\
-mkdir \"httpdocs\"
-mkdir \"httpdocs/cgi-bin\"
put \"/tmp/app\" \"httpdocs/cgi-bin/app.marty-upload\"
chmod 755 \"httpdocs/cgi-bin/app.marty-upload\"
-rm \"httpdocs/cgi-bin/app.marty-old\"
-rename \"httpdocs/cgi-bin/app\" \"httpdocs/cgi-bin/app.marty-old\"
rename \"httpdocs/cgi-bin/app.marty-upload\" \"httpdocs/cgi-bin/app\"
-rm \"httpdocs/cgi-bin/app.marty-old\"
put \"/tmp/marty.conf\" \"httpdocs/cgi-bin/marty.conf\"
chmod 600 \"httpdocs/cgi-bin/marty.conf\"
"
        );
    }

    #[test]
    fn absolute_mkdir_and_urls() {
        assert_eq!(
            sftp_batch(&[Op::Mkdir("/www/cgi-bin".into())]),
            "-mkdir \"/www\"\n-mkdir \"/www/cgi-bin\"\n"
        );
        let r = Remote {
            protocol: Protocol::Ftps,
            host: "ftp.example.org".into(),
            user: Some("u".into()),
            port: Some(2121),
            password: None,
        };
        assert_eq!(
            r.ftp_url("a b/.htaccess"),
            "ftp://ftp.example.org:2121/a%20b/.htaccess"
        );
        assert_eq!(r.ftp_url("/abs/x"), "ftp://ftp.example.org:2121/%2Fabs/x");
        assert_eq!(swap_path("d/app", "app.tmp"), "d/app.tmp");
    }
}
