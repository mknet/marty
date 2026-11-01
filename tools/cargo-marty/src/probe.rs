//! `cargo marty probe`: upload the hosting probe under a random name, fetch it (inside view),
//! measure it from outside, remove it again, and save the report under `deploy/<target>/`.

use crate::build;
use crate::deploy;
use crate::target::{LOCAL_CONFIG_DIR, Target, join};
use crate::transport::{self, Op, Remote};
use crate::{Args, Result};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Pause between outside requests: low volume on a shared host.
const PAUSE: Duration = Duration::from_secs(1);
const README: &str = "https://github.com/mknet/marty/tree/main/tools/hosting-probe";

pub fn probe(args: &Args) -> Result<()> {
    let name = deploy::target_name(args, "probe")?;
    // A cargo project is optional: the probe is embedded, so only marty.deploy.toml matters.
    let meta = build::metadata().ok();
    let (t, deploy_file) =
        deploy::load_target(&name, meta.as_ref().map(|m| m.workspace_root.as_std_path()))?;
    let triple = build::triple(args.triple.as_deref(), args.arm || t.arm)?;
    let build_base = match &meta {
        Some(m) => m.target_directory.clone().into_std_path_buf(),
        None => env::temp_dir().join("cargo-marty"),
    };
    let base_url = probe_base_url(&t)?;
    let file = random_name()?;
    let remote_bin = join(&t.cgi_dir, &file);
    let remote_pl = format!("{remote_bin}.pl");
    let url_bin = format!("{base_url}/{file}");
    let url_pl = format!("{url_bin}.pl");
    let perl = !args.no_perl;
    let dest = deploy::destination(&t)?;
    let report_dir = deploy_file
        .parent()
        .unwrap_or(Path::new("."))
        .join(LOCAL_CONFIG_DIR)
        .join(&t.name);

    if args.dry_run {
        println!("dry run: probe target `{}` → {dest}", t.name);
        println!(
            "  build   embedded marty-hosting-probe --target {triple} in {}",
            build_base.join("marty-probe").display()
        );
        println!("  mkdir   {}", t.cgi_dir);
        println!("  put     probe → {remote_bin} (chmod 755; random name per run)");
        if perl {
            println!("  put     probe.pl → {remote_pl} (chmod 755)");
        }
        println!("  get     {url_bin}");
        if perl {
            println!("  get     {url_pl}");
        }
        println!("  outside headers, keep-alive, HTTP/2, 3× timing on {url_bin} (1 s apart)");
        if args.keep {
            println!("  (--keep: files stay on the server)");
        } else {
            println!("  rm      {remote_bin}");
            if perl {
                println!("  rm      {remote_pl}");
            }
        }
        println!(
            "  save    {} (chmod 600)",
            report_dir.join("probe-<YYYYMMDD-HHMMSS>.txt").display()
        );
        return Ok(());
    }

    let binary = build::build_probe(&build_base, &triple)?;
    let remote = Remote::connect(&t)?;
    let work = transport::temp_dir()?;
    let pl_local = work.join("probe.pl");
    fs::write(&pl_local, build::PROBE_PL)?;

    let mut ops = vec![
        Op::Mkdir(t.cgi_dir.clone()),
        Op::Upload {
            local: binary.clone(),
            remote: remote_bin.clone(),
            mode: 0o755,
            swap: false,
        },
    ];
    let mut uploaded = vec![remote_bin.clone()];
    if perl {
        ops.push(Op::Upload {
            local: pl_local,
            remote: remote_pl.clone(),
            mode: 0o755,
            swap: false,
        });
        uploaded.push(remote_pl.clone());
    }

    let stamp = utc_stamp(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs());
    let mut report = format!(
        "cargo marty probe {}\ntarget: {} → {dest}\ncgi_dir: {}\nurl: {url_bin}\nbuilt for: {triple}\ntime: {stamp} UTC\n",
        env!("CARGO_PKG_VERSION"),
        t.name,
        t.cgi_dir,
    );
    let mut fetched = false;
    let result = (|| -> Result<()> {
        for op in &ops {
            println!("  {op}");
        }
        remote.apply(&ops)?;
        fetched = true;
        let ok = inside(&mut report, "rust", &url_bin, &t)?;
        if perl {
            inside(&mut report, "perl", &url_pl, &t)?;
        }
        if !ok {
            return Err(format!("the probe at {url_bin} did not answer with 2xx").into());
        }
        outside(&mut report, &url_bin)
    })();

    if args.keep {
        println!("kept on the server (--keep), remove by hand:");
        for p in &uploaded {
            println!("  {p}");
        }
    } else {
        let cleanup: Vec<Op> = uploaded.iter().cloned().map(Op::Remove).collect();
        for op in &cleanup {
            println!("  {op}");
        }
        if let Err(e) = remote.apply(&cleanup) {
            eprintln!("warning: could not remove the probe ({e}); check and delete by hand:");
            for p in &uploaded {
                eprintln!("  {p}");
            }
        }
    }
    let _ = fs::remove_dir_all(&work);

    if fetched {
        let path = report_dir.join(format!("probe-{stamp}.txt"));
        save_private(&path, &report)?;
        println!("report: {}", path.display());
    }
    result
}

/// `https://<domain>/<cgi_dir below docroot>`, or the origin of `url` plus the same path.
fn probe_base_url(t: &Target) -> Result<String> {
    let origin = match (&t.url, &t.domain) {
        (Some(u), _) => origin_of(u),
        (None, Some(d)) => {
            let d = d.trim_end_matches('/');
            if d.contains("://") {
                d.to_string()
            } else {
                format!("https://{d}")
            }
        }
        (None, None) => {
            return Err(format!(
                "target `{}`: set domain = \"example.org\" in {}",
                t.name,
                crate::target::DEPLOY_FILE
            )
            .into());
        }
    };
    let mount = match deploy::cgi_mount(t) {
        Some(m) => m.trim_matches('/'),
        None => {
            let last = t.cgi_dir.rsplit('/').next().unwrap_or(&t.cgi_dir);
            eprintln!(
                "warning: cgi_dir {} is outside docroot {}; assuming it is served as /{last}/",
                t.cgi_dir, t.docroot
            );
            last
        }
    };
    Ok(if mount.is_empty() {
        origin
    } else {
        format!("{origin}/{mount}")
    })
}

/// `scheme://host[:port]` of a URL (`https://` when no scheme is given).
fn origin_of(url: &str) -> String {
    let (scheme, rest) = url.split_once("://").unwrap_or(("https", url));
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    format!("{scheme}://{host}")
}

/// `marty-probe-<16 hex>`: the probe prints the full CGI env, so its URL must not be guessable.
fn random_name() -> Result<String> {
    let mut bytes = [0u8; 8];
    fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|e| format!("cannot read /dev/urandom: {e}"))?;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!("marty-probe-{hex}"))
}

/// Add `text` to the report file; the terminal only gets the summary lines.
fn emit(report: &mut String, text: &str) {
    report.push_str(text);
    if !text.ends_with('\n') {
        report.push('\n');
    }
}

/// Add `text` to the report and print it (failures and hints).
fn shout(report: &mut String, text: &str) {
    eprint!("{text}");
    emit(report, text);
}

fn section(report: &mut String, title: &str) {
    emit(report, &format!("\n######## {title} ########\n"));
}

struct CurlOut {
    ok: bool,
    stdout: String,
    stderr: String,
}

fn curl(args: &[&str]) -> Result<CurlOut> {
    let out = Command::new("curl")
        .args(["-sS", "--max-time", "30"])
        .args(args)
        .output()
        .map_err(|e| format!("cannot run curl: {e}"))?;
    Ok(CurlOut {
        ok: out.status.success(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
    })
}

const MARKER: &str = "\n__marty__ ";

/// GET the probe once and record its body. `Ok(false)` on a non-2xx answer.
fn inside(report: &mut String, kind: &str, url: &str, t: &Target) -> Result<bool> {
    section(report, &format!("inside: {kind} probe, GET {url}"));
    let out = curl(&[
        "-w",
        "\n__marty__ %{http_code} %{time_starttransfer} %{time_total}\n",
        url,
    ])?;
    if !out.ok {
        shout(
            report,
            &format!("{kind} probe: GET {url} failed: {}\n", out.stderr),
        );
        return Ok(false);
    }
    let Some((status, ttfb, total)) = deploy::parse_curl_check(&out.stdout) else {
        shout(report, &format!("{kind} probe: unexpected curl output\n"));
        return Ok(false);
    };
    let body = match out.stdout.rfind(MARKER) {
        Some(i) => &out.stdout[..i],
        None => out.stdout.as_str(),
    };
    emit(
        report,
        &format!(
            "status {status}, TTFB {:.0} ms, total {:.0} ms\n",
            ttfb * 1000.0,
            total * 1000.0
        ),
    );
    emit(report, body);
    let ok = (200..300).contains(&status);
    println!(
        "  {:<11} {status} in {:.0} ms{}",
        format!("{kind} probe"),
        total * 1000.0,
        if ok {
            format!("  {}", inside_summary(body))
        } else {
            String::new()
        }
    );
    if ok {
        return Ok(true);
    }
    let hint = match status {
        403 | 404 | 500 => format!(
            "hint: {status} usually means {} does not execute CGI: enable CGI for the domain \
             (netcup: WCP → CGI-Unterstützung) or allow it with a .htaccess there \
             (`Options +ExecCGI`, `AddHandler cgi-script .pl .cgi`); see {README}\n",
            t.cgi_dir
        ),
        _ => format!("hint: unexpected status {status}; see {README}\n"),
    };
    shout(report, &hint);
    if let Some(n) = &t.notes {
        shout(report, &format!("note ({}): {n}\n", t.name));
    }
    Ok(false)
}

/// Port of `tools/hosting-probe/probe-outside.sh`: ~6 sequential requests.
fn outside(report: &mut String, url: &str) -> Result<()> {
    section(
        report,
        "outside: response headers (HTTP/1.1, single request)",
    );
    let out = curl(&["--http1.1", "-o", "/dev/null", "-D", "-", url])?;
    emit(report, &out.stdout.replace('\r', ""));
    if !out.ok {
        emit(report, &format!("curl failed: {}\n", out.stderr));
    }
    thread::sleep(PAUSE);

    section(
        report,
        "outside: keep-alive, two requests on one connection",
    );
    let out = curl(&[
        "--http1.1",
        "-o",
        "/dev/null",
        "-o",
        "/dev/null",
        "-v",
        url,
        url,
    ])?;
    let lines = keepalive_lines(&out.stderr);
    emit(report, &lines.join("\n"));
    let reused = lines.iter().any(|l| {
        let l = l.to_ascii_lowercase();
        l.contains("re-using") || l.contains("reusing")
    });
    emit(
        report,
        &format!("connection reused: {}\n", if reused { "yes" } else { "no" }),
    );
    thread::sleep(PAUSE);

    section(report, "outside: HTTP/2 negotiation (TLS only)");
    let out = curl(&[
        "--http2",
        "-o",
        "/dev/null",
        "-w",
        "http_version=%{http_version} code=%{http_code}\n",
        url,
    ])?;
    emit(report, if out.ok { &out.stdout } else { &out.stderr });
    let http = out
        .stdout
        .split_whitespace()
        .find_map(|w| w.strip_prefix("http_version="))
        .filter(|_| out.ok)
        .map(|v| format!("HTTP/{v}"))
        .unwrap_or_else(|| "HTTP version n/a".into());
    thread::sleep(PAUSE);

    section(
        report,
        "outside: timing, 3 sequential requests (new connection each)",
    );
    emit(
        report,
        &format!(
            "{:<6} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}\n",
            "run", "connect", "tls", "ttfb", "total", "size", "server≈"
        ),
    );
    let mut server = Vec::new();
    for run in 1..=3 {
        let out = curl(&[
            "--http1.1",
            "-o",
            "/dev/null",
            "-w",
            "\n__marty__ %{time_connect} %{time_appconnect} %{time_starttransfer} %{time_total} %{size_download}\n",
            url,
        ])?;
        let line = match parse_timing(&out.stdout) {
            Some(t) if out.ok => {
                server.push(format!("{:.0}", t.server() * 1000.0));
                format!(
                    "{run:<6} {:>7.0} ms {:>7.0} ms {:>7.0} ms {:>7.0} ms {:>8} B {:>7.0} ms\n",
                    t.connect * 1000.0,
                    t.appconnect * 1000.0,
                    t.ttfb * 1000.0,
                    t.total * 1000.0,
                    t.size,
                    t.server() * 1000.0
                )
            }
            _ => format!("{run:<6} curl failed: {}\n", out.stderr),
        };
        emit(report, &line);
        if run < 3 {
            thread::sleep(PAUSE);
        }
    }
    emit(
        report,
        "server≈ = ttfb - tls (or - connect for plain http): server-side time incl. CGI fork/exec\n",
    );
    println!(
        "  {:<11} {http}, keep-alive {}, server≈ {} ms",
        "outside",
        if reused { "reused" } else { "not reused" },
        if server.is_empty() {
            "n/a".to_string()
        } else {
            server.join("/")
        }
    );
    Ok(())
}

/// One line from the probe body: CGI model, user, OS and CPUs.
fn inside_summary(body: &str) -> String {
    let field = |key: &str| {
        body.lines()
            .find_map(|l| l.strip_prefix(key))
            .map(str::trim)
            .filter(|v| !v.is_empty() && *v != "n/a")
    };
    let first = |key: &str| field(key).and_then(|v| v.split_whitespace().next());
    let mut parts = Vec::new();
    if let Some(v) = first("stdout:") {
        parts.push(format!("stdout={v}"));
    }
    if let Some(v) = first("uid:") {
        match first("user:") {
            Some(u) if u != "n/a" => parts.push(format!("uid={v} ({u})")),
            _ => parts.push(format!("uid={v}")),
        }
    }
    let mut sys = Vec::new();
    if let Some(v) = field("os:") {
        sys.push(v.to_string());
    }
    if let Some(v) = first("cpus:").filter(|v| *v != "0") {
        sys.push(format!("{v} cpus"));
    }
    if !sys.is_empty() {
        parts.push(sys.join(", "));
    }
    parts.join("  ")
}

/// The connection-related lines of `curl -v` output (same filter as `probe-outside.sh`).
fn keepalive_lines(verbose: &str) -> Vec<String> {
    const KEYS: &[&str] = &[
        "connected to",
        "re-using",
        "reusing",
        "connection #",
        "closing connection",
        "< connection",
        "< keep-alive",
        "< transfer-encoding",
        "< content-length",
        "< server",
    ];
    verbose
        .lines()
        .map(|l| l.trim_end_matches('\r'))
        .filter(|l| {
            let l = l.to_ascii_lowercase();
            KEYS.iter().any(|k| l.contains(k))
        })
        .map(str::to_string)
        .collect()
}

#[derive(Debug, PartialEq)]
struct Timing {
    connect: f64,
    appconnect: f64,
    ttfb: f64,
    total: f64,
    size: u64,
}

impl Timing {
    fn server(&self) -> f64 {
        self.ttfb - self.appconnect.max(self.connect)
    }
}

fn parse_timing(text: &str) -> Option<Timing> {
    let line = text
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("__marty__ "))?;
    let mut p = line.split_whitespace();
    let mut f = || p.next()?.parse::<f64>().ok();
    Some(Timing {
        connect: f()?,
        appconnect: f()?,
        ttfb: f()?,
        total: f()?,
        size: f()? as u64,
    })
}

/// The report contains the server's CGI env: dir 700, file 600.
fn save_private(path: &Path, text: &str) -> Result<()> {
    let dir: PathBuf = path.parent().expect("deploy/<target>").to_path_buf();
    let fresh = !dir.exists();
    fs::create_dir_all(&dir)?;
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        if fresh {
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
        }
        opts.mode(0o600);
    }
    #[cfg(not(unix))]
    let _ = fresh;
    opts.open(path)?.write_all(text.as_bytes())?;
    Ok(())
}

/// `YYYYMMDD-HHMMSS` (UTC) for `secs` since the Unix epoch.
fn utc_stamp(secs: u64) -> String {
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    let s = secs % 86_400;
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        s / 3600,
        s % 3600 / 60,
        s % 60
    )
}

/// Days since 1970-01-01 → (year, month, day), proleptic Gregorian (H. Hinnant).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::parse_target;

    fn target(extra: &str) -> Target {
        parse_target(&format!("[target.a]\n{extra}"), "a", Path::new(".")).unwrap()
    }

    #[test]
    fn base_url_from_domain_and_cgi_dir() {
        let t = target("preset = \"netcup\"\ndomain = \"example.org\"\n");
        assert_eq!(probe_base_url(&t).unwrap(), "https://example.org/cgi-bin");
        let t = target("docroot = \".\"\ndomain = \"http://example.org/\"\n");
        assert_eq!(probe_base_url(&t).unwrap(), "http://example.org/cgi-bin");
        let t = target(
            "docroot = \"htdocs\"\ncgi_dir = \"htdocs/app/cgi\"\ndomain = \"example.org\"\n",
        );
        assert_eq!(probe_base_url(&t).unwrap(), "https://example.org/app/cgi");
        let t = target("docroot = \"htdocs\"\ncgi_dir = \"htdocs\"\ndomain = \"example.org\"\n");
        assert_eq!(probe_base_url(&t).unwrap(), "https://example.org");
        assert!(probe_base_url(&target("docroot = \"htdocs\"\n")).is_err());
    }

    #[test]
    fn base_url_from_url_origin() {
        let t = target(
            "preset = \"netcup\"\ndomain = \"ignored.org\"\nurl = \"https://example.org:8443/app/health?x=1\"\n",
        );
        assert_eq!(
            probe_base_url(&t).unwrap(),
            "https://example.org:8443/cgi-bin"
        );
        assert_eq!(origin_of("example.org/x"), "https://example.org");
    }

    #[test]
    fn parses_outside_timing() {
        let text = "\n__marty__ 0.012 0.045 0.120 0.121 4711\n";
        let t = parse_timing(text).unwrap();
        assert_eq!(
            t,
            Timing {
                connect: 0.012,
                appconnect: 0.045,
                ttfb: 0.120,
                total: 0.121,
                size: 4711
            }
        );
        assert!((t.server() - 0.075).abs() < 1e-9);
        let plain = parse_timing("__marty__ 0.010 0.000 0.050 0.051 10").unwrap();
        assert!((plain.server() - 0.040).abs() < 1e-9);
        assert_eq!(parse_timing("garbage"), None);
    }

    #[test]
    fn summarizes_inside_view() {
        let body = "\n== process ==\nppid: 1\nuid: 2712 euid: 2712 gid: 1164 egid: 1164\n\
                    user: koch euser: koch\nstdout: socket   (pipe => …)\n\
                    os: Ubuntu 24.04.5 LTS\ncpus: 8 model: n/a\n";
        assert_eq!(
            inside_summary(body),
            "stdout=socket  uid=2712 (koch)  Ubuntu 24.04.5 LTS, 8 cpus"
        );
        assert_eq!(
            inside_summary("stdout: pipe\nos: n/a\ncpus: 0 model: n/a\n"),
            "stdout=pipe"
        );
    }

    #[test]
    fn filters_keepalive_lines() {
        let v = "* Connected to example.org (1.2.3.4) port 443\r\n> GET / HTTP/1.1\r\n\
                 < Connection: keep-alive\r\n< Content-Type: text/plain\r\n\
                 * Re-using existing connection with host example.org\r\n";
        assert_eq!(
            keepalive_lines(v),
            [
                "* Connected to example.org (1.2.3.4) port 443",
                "< Connection: keep-alive",
                "* Re-using existing connection with host example.org",
            ]
        );
    }

    #[test]
    fn random_names_and_stamps() {
        let n = random_name().unwrap();
        assert_eq!(n.len(), "marty-probe-".len() + 16);
        assert_ne!(n, random_name().unwrap());
        assert_eq!(utc_stamp(0), "19700101-000000");
        assert_eq!(utc_stamp(951_782_400 + 3_723), "20000229-010203");
    }
}
