#!/usr/bin/env bash
# Capture strace for marty-complex CGI children and summarize slowest children.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$HERE/docker-compose.yml"
COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-marty-bench-complex}"
BASE="${BENCH_BASE_URL:-http://127.0.0.1:8081}"
ROUTE="${TRACE_ROUTE:-fibonacci/42}"
N="${TRACE_REQUESTS:-200}"
C="${TRACE_CONCURRENCY:-10}"
TRACE_DIR="${TRACE_DIR:-$HERE/.trace}"
TRACE_TIMEOUT_SEC="${TRACE_TIMEOUT_SEC:-30}"
HEY_TIMEOUT_SEC="${HEY_TIMEOUT_SEC:-45}"

# shellcheck source=../../simple/scripts/ensure-hey.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/../../simple/scripts/ensure-hey.sh"
# shellcheck source=docker-lib.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/docker-lib.sh"
ensure_hey

dc() {
  docker compose -p "$COMPOSE_PROJECT_NAME" -f "$COMPOSE_FILE" "$@"
}

wait_ready() {
  local url="$1"
  local i
  for i in $(seq 1 60); do
    if curl -sf -o /dev/null -m 5 "$url"; then
      return 0
    fi
    sleep 1
  done
  echo "error: timeout waiting for $url" >&2
  return 1
}

ensure_stack() {
  local port
  port="$(bench_host_port "$BASE")"
  export BENCH_PORT="$port"
  export BENCH_TIMING="${BENCH_TIMING:-0}"
  if [[ "${BENCH_REBUILD:-0}" == "1" ]]; then
    stop_docker_on_port "$port"
    dc build
    dc up -d --force-recreate
  elif [[ -z "$(dc ps -q bench 2>/dev/null)" ]]; then
    stop_docker_on_port "$port"
    dc up -d
  fi
  wait_ready "${BASE}/cgi-bin/marty-complex/primes/10000?salt=0"
}

start_strace() {
  local cid="$1"
  docker exec "$cid" sh -c 'rm -rf /tmp/marty-strace && mkdir -p /tmp/marty-strace'
  local apache_pids attach_args pid
  apache_pids="$(docker exec "$cid" sh -c 'pgrep apache2 | tr "\n" " "')"
  attach_args=""
  for pid in $apache_pids; do
    attach_args="${attach_args} -p ${pid}"
  done
  echo ">> strace apache pids ${apache_pids}"
  docker exec -d "$cid" sh -c "timeout -k 2s ${TRACE_TIMEOUT_SEC}s strace -ff -tt -T -s 128 -o /tmp/marty-strace/trace ${attach_args}"
  sleep 1
}

stop_strace() {
  local cid="$1"
  docker exec "$cid" sh -c 'pkill -TERM strace 2>/dev/null || true'
  for _ in $(seq 1 20); do
    if ! docker exec "$cid" sh -c 'pgrep strace >/dev/null 2>&1'; then
      return 0
    fi
    sleep 0.2
  done
  docker exec "$cid" sh -c 'pkill -KILL strace 2>/dev/null || true'
}

copy_traces() {
  local cid="$1"
  rm -rf "$TRACE_DIR"
  mkdir -p "$TRACE_DIR"
  docker cp "$cid:/tmp/marty-strace/." "$TRACE_DIR/"
}

run_hey_with_timeout() {
  local url="$1"
  python3 - "$HEY_TIMEOUT_SEC" "$N" "$C" "$url" <<'PY'
import subprocess
import sys

timeout_s = float(sys.argv[1])
n = sys.argv[2]
c = sys.argv[3]
url = sys.argv[4]

try:
    completed = subprocess.run(["hey", "-n", n, "-c", c, "-m", "GET", url], timeout=timeout_s)
    raise SystemExit(completed.returncode)
except subprocess.TimeoutExpired:
    print(f">> hey timed out after {timeout_s:g}s", file=sys.stderr)
    raise SystemExit(124)
PY
}

summarize_traces() {
  python3 - "$TRACE_DIR" <<'PY'
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1])
time_re = re.compile(r"^(\d\d):(\d\d):(\d\d)\.(\d+)")
dur_re = re.compile(r"<([0-9.]+)>")
rows = []

def ts_to_us(match):
    h, m, s, frac = match.groups()
    frac = (frac + "000000")[:6]
    return ((int(h) * 3600 + int(m) * 60 + int(s)) * 1_000_000) + int(frac)

for path in root.glob("trace.*"):
    text = path.read_text(errors="replace")
    if "marty-complex" not in text:
        continue
    stamps = [ts_to_us(m) for m in time_re.finditer(text)]
    if not stamps:
        continue
    syscall_us = sum(float(m.group(1)) * 1_000_000 for m in dur_re.finditer(text))
    rows.append((stamps[-1] - stamps[0], syscall_us, path.name, len(text.splitlines())))

if not rows:
    print("No marty-complex child traces found. Rebuild the image so strace is installed and SYS_PTRACE is enabled.")
    raise SystemExit(0)

print("Slowest marty-complex traced children:")
print(f"{'wall_ms':>10} {'syscall_ms':>12} {'lines':>8} file")
for wall_us, syscall_us, name, lines in sorted(rows, reverse=True)[:10]:
    print(f"{wall_us / 1000:10.2f} {syscall_us / 1000:12.2f} {lines:8d} {name}")
PY
}

main() {
  if ! command -v docker >/dev/null 2>&1; then
    echo "error: docker not on PATH" >&2
    exit 1
  fi

  ensure_stack
  local cid
  cid="$(dc ps -q bench)"
  if [[ -z "$cid" ]]; then
    echo "error: bench container not running" >&2
    exit 1
  fi

  start_strace "$cid"
  echo ">> hey ${N} requests, concurrency ${C}: ${BASE}/cgi-bin/marty-complex/${ROUTE}"
  run_hey_with_timeout "${BASE}/cgi-bin/marty-complex/${ROUTE}" || true
  stop_strace "$cid"
  copy_traces "$cid"
  summarize_traces
  echo ">> traces copied to ${TRACE_DIR}"
}

main "$@"
