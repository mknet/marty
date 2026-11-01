//! Hosting probe (Rust CGI): same report as `probe.pl`, but as a static binary.
//!
//! Running it on the target host answers two questions at once: "do foreign
//! static binaries execute in this `cgi-bin` at all?" and "what does the
//! Apache/CGI setup look like from inside?" (mod_cgi pipe vs mod_cgid socket,
//! suexec/wrapper in the parent chain, MPM hints, kernel, CPU). Read-only,
//! std-only, one request is enough.

use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::mem::ManuallyDrop;
use std::os::fd::FromRawFd;
use std::os::unix::fs::FileTypeExt;

// libc, which std links anyway; works when `/proc` is hidden (hidepid).
unsafe extern "C" {
    safe fn getuid() -> u32;
    safe fn geteuid() -> u32;
    safe fn getgid() -> u32;
    safe fn getegid() -> u32;
}

fn main() {
    let mut out = String::with_capacity(8 * 1024);
    out.push_str("Content-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\n\r\n");

    section(&mut out, "probe");
    let _ = writeln!(out, "kind: rust {}", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(out, "target: {}", target_triple());

    section(&mut out, "process");
    let pid = std::process::id();
    let _ = writeln!(out, "pid: {pid}");
    let ppid = std::os::unix::process::parent_id();
    let _ = writeln!(out, "ppid: {ppid}");
    let _ = writeln!(
        out,
        "uid: {} euid: {} gid: {} egid: {}",
        getuid(),
        geteuid(),
        getgid(),
        getegid()
    );
    let _ = writeln!(
        out,
        "cwd: {}",
        std::env::current_dir().map_or("n/a".into(), |p| p.display().to_string())
    );
    let _ = writeln!(
        out,
        "exe: {}",
        std::env::current_exe().map_or("n/a".into(), |p| p.display().to_string())
    );
    let _ = writeln!(out, "stdin: {}", fd_kind(0));
    let _ = writeln!(
        out,
        "stdout: {}   (pipe => mod_cgi/prefork-style; socket => mod_cgid/threaded MPM)",
        fd_kind(1)
    );
    let _ = writeln!(out, "stderr: {}", fd_kind(2));
    let _ = writeln!(
        out,
        "threads: {}",
        proc_field(pid, "Threads").unwrap_or_else(na)
    );

    section(&mut out, "parent chain (up to 6 levels)");
    let mut cur = Some(ppid);
    for level in 1..=6 {
        let Some(p) = cur else { break };
        if p == 0 {
            break;
        }
        let _ = writeln!(
            out,
            "{level}: pid={p} comm={} uid={} threads={} cmdline={}",
            proc_comm(p).unwrap_or_else(na),
            proc_field(p, "Uid").unwrap_or_else(na),
            proc_field(p, "Threads").unwrap_or_else(na),
            proc_cmdline(p).unwrap_or_else(na),
        );
        if p == 1 {
            break;
        }
        cur = proc_ppid(p);
    }

    section(&mut out, "httpd-like processes visible in /proc");
    httpd_summary(&mut out);

    section(&mut out, "system");
    let _ = write!(
        out,
        "kernel: {}",
        read("/proc/version").unwrap_or_else(|| "n/a\n".into())
    );
    let _ = writeln!(out, "os: {}", os_pretty_name().unwrap_or_else(na));
    let (ncpu, model) = cpu_info();
    let _ = writeln!(out, "cpus: {ncpu} model: {}", model.unwrap_or_else(na));
    let _ = writeln!(out, "memtotal: {}", meminfo_total().unwrap_or_else(na));
    let _ = write!(
        out,
        "loadavg: {}",
        read("/proc/loadavg").unwrap_or_else(|| "n/a\n".into())
    );
    let _ = writeln!(
        out,
        "proc/1 readable: {}",
        if read("/proc/1/status").is_some() {
            "yes"
        } else {
            "no"
        }
    );
    let _ = writeln!(
        out,
        "max processes (limits): {}",
        proc_limit("Max processes").unwrap_or_else(na)
    );

    section(&mut out, "environment");
    let mut env: Vec<(String, String)> = std::env::vars().collect();
    env.sort();
    for (k, v) in env {
        let v = if v.starts_with("-----BEGIN") {
            format!("(PEM, {} bytes, not shown)", v.len())
        } else {
            v.replace(['\r', '\n'], " ")
        };
        let _ = writeln!(out, "{k}={v}");
    }

    // One write to stdout, independent of LineWriter buffering.
    let stdout = unsafe { fs::File::from_raw_fd(1) };
    let mut stdout = ManuallyDrop::new(stdout);
    if stdout.write_all(out.as_bytes()).is_err() {
        // Fallback for platforms where fd 1 is not a regular handle.
        let _ = io::stdout().write_all(out.as_bytes());
    }
}

fn na() -> String {
    "n/a".to_string()
}

fn section(out: &mut String, title: &str) {
    let _ = writeln!(out, "\n== {title} ==");
}

fn target_triple() -> String {
    format!(
        "{}-{}-{}",
        std::env::consts::ARCH,
        std::env::consts::OS,
        std::env::consts::FAMILY
    )
}

fn read(path: &str) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn fd_kind(fd: i32) -> &'static str {
    // SAFETY: fds 0..=2 are inherited from the CGI server; ManuallyDrop keeps
    // them open after metadata() is taken.
    let file = ManuallyDrop::new(unsafe { fs::File::from_raw_fd(fd) });
    match file.metadata() {
        Ok(md) => {
            let ft = md.file_type();
            if ft.is_fifo() {
                "pipe"
            } else if ft.is_socket() {
                "socket"
            } else if ft.is_char_device() {
                "chardev/tty"
            } else if ft.is_file() {
                "file"
            } else {
                "unknown"
            }
        }
        Err(_) => "closed/invalid",
    }
}

fn proc_field(pid: u32, field: &str) -> Option<String> {
    let status = read(&format!("/proc/{pid}/status"))?;
    let prefix = format!("{field}:");
    status
        .lines()
        .find_map(|l| l.strip_prefix(&prefix))
        .map(|v| v.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn proc_comm(pid: u32) -> Option<String> {
    read(&format!("/proc/{pid}/comm")).map(|s| s.trim().to_string())
}

fn proc_cmdline(pid: u32) -> Option<String> {
    let raw = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let s: String = raw
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p).into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    if s.is_empty() { None } else { Some(s) }
}

fn proc_ppid(pid: u32) -> Option<u32> {
    let stat = read(&format!("/proc/{pid}/stat"))?;
    // "pid (comm) state ppid ..." — comm may contain spaces, so split after the last ')'.
    let rest = stat.rsplit_once(')')?.1;
    rest.split_whitespace().nth(1)?.parse().ok()
}

fn proc_limit(name: &str) -> Option<String> {
    let limits = read("/proc/self/limits")?;
    limits.lines().find(|l| l.starts_with(name)).map(|l| {
        let cols: Vec<&str> = l.split_whitespace().collect();
        // "Max processes  <soft>  <hard>  processes"
        cols.iter()
            .rev()
            .nth(2)
            .map_or_else(|| l.to_string(), |v| (*v).to_string())
    })
}

fn httpd_summary(out: &mut String) {
    let Ok(dir) = fs::read_dir("/proc") else {
        let _ = writeln!(out, "/proc not readable");
        return;
    };
    let mut visible = 0usize;
    let mut matching = 0usize;
    let mut threads = 0u64;
    let mut by_comm: std::collections::BTreeMap<String, usize> = Default::default();
    for entry in dir.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let Some(comm) = proc_comm(pid) else { continue };
        visible += 1;
        let lc = comm.to_ascii_lowercase();
        let interesting = [
            "apache2",
            "httpd",
            "suexec",
            "cgi",
            "fcgi",
            "php",
            "lsphp",
            "litespeed",
            "nginx",
        ]
        .iter()
        .any(|p| lc.starts_with(p));
        if !interesting {
            continue;
        }
        matching += 1;
        *by_comm.entry(comm).or_default() += 1;
        if let Some(t) = proc_field(pid, "Threads").and_then(|t| t.parse::<u64>().ok()) {
            threads += t;
        }
    }
    let _ = writeln!(
        out,
        "visible processes in /proc: {visible} (only own processes visible => hidepid is active)"
    );
    let _ = writeln!(
        out,
        "matching processes: {matching}, summed threads: {threads}"
    );
    for (comm, n) in by_comm {
        let _ = writeln!(out, "  {comm:<20} {n}");
    }
    let _ = writeln!(
        out,
        "hint: many 1-thread apache2/httpd => prefork; few procs with many threads => worker/event"
    );
}

fn os_pretty_name() -> Option<String> {
    let os = read("/etc/os-release")?;
    os.lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim_matches('"').to_string())
}

fn cpu_info() -> (usize, Option<String>) {
    let Some(info) = read("/proc/cpuinfo") else {
        return (
            std::thread::available_parallelism().map_or(0, |n| n.get()),
            None,
        );
    };
    let ncpu = info.lines().filter(|l| l.starts_with("processor")).count();
    let model = info
        .lines()
        .find_map(|l| l.strip_prefix("model name"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.trim().to_string());
    (ncpu, model)
}

fn meminfo_total() -> Option<String> {
    let mem = read("/proc/meminfo")?;
    mem.lines()
        .find_map(|l| l.strip_prefix("MemTotal:"))
        .map(|v| v.trim().to_string())
}
