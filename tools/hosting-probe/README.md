# Hosting probe

Read-only CGI probes that tell us what a shared-hosting Apache
looks like from the inside and from the outside, so the setup can be mirrored
in the local Docker benchmark **without** putting load on the provider.
Verified so far: **all-inkl** and **Hetzner Webhosting S** (both socket/cgid-class — see [`docs/HOSTERS.md`](../../docs/HOSTERS.md)).

A handful of sequential requests is all it takes — no `hey`, no concurrency.

## What it reports

| Fact | Why it matters for the CGI benchmarks | Source |
|---|---|---|
| stdout is a **pipe** or a **socket** | pipe → `mod_cgi` (prefork-style), socket → `mod_cgid` (threaded MPM) | `fstat(1)` |
| parent chain (`comm`, uid, threads) | suexec / cgi-wrapper = one extra `exec` per request; threads hint at MPM | `/proc/<ppid>/…` |
| httpd-like processes + summed threads | many 1-thread procs → prefork; few procs, many threads → worker/event | `/proc` scan |
| kernel, distro, CPU model, nproc, memory | pick a matching Docker base image / CPU count | `/proc`, `/etc/os-release` |
| full CGI environment | `ScriptAlias` vs `AddHandler`, rewrites, `REDIRECT_*`, `HTTPS` | env |
| response headers, keep-alive, chunked, HTTP/2, TTFB | reproduces the TCP/keep-alive side that moved our numbers before | `probe-outside.sh` |

Some `/proc` paths may be unreadable on hardened hosts (`hidepid`); the probes
print `n/a` instead of failing.

## Files

- `probe.pl` — Perl CGI, core modules only. Zero build risk; try this first.
- `probe-rs/` — `marty-hosting-probe`, std-only Rust binary (static musl).
  Running it also proves that foreign static binaries execute in that `cgi-bin`.
- `probe-outside.sh` — `curl`-based outside view (≈8 sequential requests).

## Usage

```bash
cd tools/hosting-probe
just linux            # x86_64 musl → dist/   (or: just linux arm)
```

Upload `dist/probe.pl` and `dist/marty-hosting-probe` into the host's `cgi-bin`
(`chmod 755`). If the directory is not a `ScriptAlias`, a `.htaccess` like

```apache
Options +ExecCGI
AddHandler cgi-script .pl .cgi
```

may be needed (rename the binary to `marty-hosting-probe.cgi` in that case).

Then, once each:

```bash
curl -sS https://example.org/cgi-bin/probe.pl
curl -sS https://example.org/cgi-bin/marty-hosting-probe
just outside https://example.org/cgi-bin/marty-hosting-probe
```

Save the three outputs; they are the input for adjusting
`benchmarks/complex/docker/` (MPM, `mod_cgi`/`mod_cgid`, wrapper, keep-alive,
TLS/h2) before running the complex benchmark again locally.

Local sanity check without Apache: `just run-local` / `just run-local-perl`.
