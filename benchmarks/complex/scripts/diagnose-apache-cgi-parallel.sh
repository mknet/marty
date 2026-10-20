#!/usr/bin/env bash
# Diagnose Apache/mod_cgi behavior under parallel Marty-vs-Go CGI load.
#
# This intentionally runs short traces. Long strace sessions perturb Apache CGI
# heavily, so the output is best used for shape/comparison, not absolute latency.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$HERE/docker-compose.yml"
COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-marty-bench-complex}"
BASE="${BENCH_BASE_URL:-http://127.0.0.1:8081}"
TRACE_ROUTE="${TRACE_ROUTE:-primes/400000}"
TRACE_REQUESTS="${TRACE_REQUESTS:-80}"
TRACE_CONCURRENCY="${TRACE_CONCURRENCY:-10}"
TRACE_WARMUP_REQUESTS="${TRACE_WARMUP_REQUESTS:-40}"
TRACE_SECONDS="${TRACE_SECONDS:-8}"
TRACE_STRACE="${TRACE_STRACE:-1}"
STATUS_INTERVAL_SEC="${STATUS_INTERVAL_SEC:-0.10}"
OUT_DIR="${TRACE_OUT_DIR:-$HERE/.trace/apache-cgi-parallel-$(date +%Y%m%d-%H%M%S)}"

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

start_stack() {
  local port
  port="$(bench_host_port "$BASE")"
  export BENCH_PORT="$port"
  export BENCH_TIMING=0
  export GO_COMPLEX_RAW_CGI="${GO_COMPLEX_RAW_CGI:-0}"
  stop_docker_on_port "$port"
  dc down --remove-orphans 2>/dev/null || true
  dc up -d
}

ensure_apache() {
  cd "$HERE"
  export BENCH_PORT="$(bench_host_port "$BASE")"
  export BENCH_TIMING=0
  export GO_COMPLEX_RAW_CGI="${GO_COMPLEX_RAW_CGI:-0}"

  if [[ "${BENCH_REBUILD:-1}" == "1" ]]; then
    echo ">> docker compose build (BENCH_REBUILD=${BENCH_REBUILD:-1})"
    dc build
    echo ">> recreating container so mod_status/config/binaries are current"
    start_stack
    return
  fi

  if [[ -n "$(dc ps -q bench 2>/dev/null)" ]] \
    && curl -sf -o /dev/null -m 5 "$BASE/cgi-bin/marty-sync/primes/10000?salt=0" \
    && curl -sf -o /dev/null -m 5 "$BASE/server-status?auto"; then
    echo ">> reusing running complex bench container on ${BASE}"
    return
  fi

  start_stack
}

sample_status() {
  local label="$1"
  local stop_file="$2"
  local out_file="$3"
  printf "elapsed_ms\tlabel\tbusy\tidle\tscoreboard\n" >"$out_file"
  local start_ns
  start_ns="$(python3 - <<'PY'
import time
print(time.monotonic_ns())
PY
)"
  while [[ -e "$stop_file" ]]; do
    local now_ns elapsed_ms status busy idle scoreboard
    now_ns="$(python3 - <<'PY'
import time
print(time.monotonic_ns())
PY
)"
    elapsed_ms=$(( (now_ns - start_ns) / 1000000 ))
    status="$(curl -sf -m 1 "$BASE/server-status?auto" || true)"
    busy="$(awk -F': ' '$1 == "BusyWorkers" { print $2 }' <<<"$status")"
    idle="$(awk -F': ' '$1 == "IdleWorkers" { print $2 }' <<<"$status")"
    scoreboard="$(awk -F': ' '$1 == "Scoreboard" { print $2 }' <<<"$status")"
    printf "%s\t%s\t%s\t%s\t%s\n" "$elapsed_ms" "$label" "${busy:-n/a}" "${idle:-n/a}" "${scoreboard:-n/a}" >>"$out_file"
    sleep "$STATUS_INTERVAL_SEC"
  done
}

start_strace() {
  local cid="$1"
  local label="$2"
  stop_strace "$cid"
  docker exec -d "$cid" sh -lc "
    rm -f /tmp/marty-strace/diag-${label}*;
    pids=\"\$(pgrep apache2 | tr '\n' ' ')\";
    attach=\"\";
    for pid in \$pids; do attach=\"\$attach -p \$pid\"; done;
    timeout -s INT ${TRACE_SECONDS}s strace -ff -ttT -s 160 \
      -e trace=clone,execve,pipe,pipe2,read,write,writev,wait4,poll,ppoll,select,pselect6,accept4,close \
      -o /tmp/marty-strace/diag-${label} \$attach \
      >/tmp/marty-strace/diag-${label}.strace.log 2>&1 || true
  "
}

stop_strace() {
  local cid="$1"
  docker exec "$cid" sh -lc '
    pkill -INT strace 2>/dev/null || true
    sleep 0.2
    pkill -TERM strace 2>/dev/null || true
    sleep 0.2
    pkill -KILL strace 2>/dev/null || true
  ' >/dev/null 2>&1 || true
}

summarize_one() {
  local label="$1"
  local dir="$2"
  python3 - "$label" "$dir" <<'PY'
import pathlib
import re
import sys

label = sys.argv[1]
root = pathlib.Path(sys.argv[2])

hey = (root / "hey.txt").read_text(errors="replace") if (root / "hey.txt").exists() else ""
def metric(name):
    m = re.search(rf"{re.escape(name)}:\s+([0-9.eE+-]+)", hey)
    return float(m.group(1)) * 1000 if m else None
def pct(p):
    m = re.search(rf"^\s+{p}%%\s+in\s+([0-9.eE+-]+)\s+secs", hey, re.M)
    return float(m.group(1)) * 1000 if m else None
def rps():
    m = re.search(r"Requests/sec:\s+([0-9.eE+-]+)", hey)
    return float(m.group(1)) if m else None

syscalls = {}
durations = {}
execs = 0
for path in (root / "container-strace").glob(f"diag-{label}.*"):
    if path.name.endswith(".strace.log"):
        continue
    for line in path.read_text(errors="replace").splitlines():
        m = re.search(r"\b([a-zA-Z0-9_]+)\(", line)
        if not m:
            continue
        name = m.group(1)
        syscalls[name] = syscalls.get(name, 0) + 1
        if "execve(" in line and f"{label}-complex" in line:
            execs += 1
        d = re.search(r"<([0-9.]+)>$", line)
        if d:
            durations.setdefault(name, []).append(float(d.group(1)) * 1000)

status_path = root / "status.tsv"
max_busy = None
max_w = 0
max_r = 0
if status_path.exists():
    for line in status_path.read_text(errors="replace").splitlines()[1:]:
        parts = line.split("\t")
        if len(parts) < 5:
            continue
        try:
            busy = int(parts[2])
            max_busy = busy if max_busy is None else max(max_busy, busy)
        except ValueError:
            pass
        scoreboard = parts[4]
        max_w = max(max_w, scoreboard.count("W"))
        max_r = max(max_r, scoreboard.count("R"))

def slow(name):
    vals = durations.get(name, [])
    return max(vals) if vals else None

fmt = lambda v: "n/a" if v is None else f"{v:.2f}"
print(f"{label}: avg={fmt(metric('Average'))}ms p50={fmt(pct(50))}ms p95={fmt(pct(95))}ms p99={fmt(pct(99))}ms slow={fmt(metric('Slowest'))}ms rps={rps() or 0:.1f}")
print(f"{label}: max_busy={max_busy if max_busy is not None else 'n/a'} max_scoreboard_W={max_w} max_scoreboard_R={max_r}")
print(
    f"{label}: trace_execs={execs} read={syscalls.get('read', 0)} write={syscalls.get('write', 0)} "
    f"wait4={syscalls.get('wait4', 0)} poll={syscalls.get('poll', 0)} "
    f"max_read_ms={fmt(slow('read'))} max_write_ms={fmt(slow('write'))} max_wait4_ms={fmt(slow('wait4'))} max_poll_ms={fmt(slow('poll'))}"
)
PY
}

run_case() {
  local label="$1"
  local path="$2"
  local cid="$3"
  local case_dir="$OUT_DIR/$label"
  local stop_file="$case_dir/status.keep"
  mkdir -p "$case_dir/container-strace"
  : >"$stop_file"

  local url="${BASE}/cgi-bin/${path}/${TRACE_ROUTE}"

  echo ">> ${label}: warm-up ${TRACE_WARMUP_REQUESTS} requests"
  hey -n "$TRACE_WARMUP_REQUESTS" -c "$TRACE_CONCURRENCY" -m GET "$url" >/dev/null || true

  if [[ "$TRACE_STRACE" == "1" ]]; then
    echo ">> ${label}: status sampler + Apache strace + hey -n ${TRACE_REQUESTS} -c ${TRACE_CONCURRENCY}"
  else
    echo ">> ${label}: status sampler only + hey -n ${TRACE_REQUESTS} -c ${TRACE_CONCURRENCY}"
  fi
  sample_status "$label" "$stop_file" "$case_dir/status.tsv" &
  local sampler_pid=$!
  if [[ "$TRACE_STRACE" == "1" ]]; then
    start_strace "$cid" "$label"
    sleep 0.5
  fi
  hey -n "$TRACE_REQUESTS" -c "$TRACE_CONCURRENCY" -m GET "$url" >"$case_dir/hey.txt" 2>&1 || {
    rm -f "$stop_file"
    wait "$sampler_pid" 2>/dev/null || true
    if [[ "$TRACE_STRACE" == "1" ]]; then
      stop_strace "$cid"
    fi
    echo "error: hey failed for $label" >&2
    sed -n '1,80p' "$case_dir/hey.txt" >&2
    return 1
  }
  rm -f "$stop_file"
  wait "$sampler_pid" 2>/dev/null || true
  if [[ "$TRACE_STRACE" == "1" ]]; then
    stop_strace "$cid"
    sleep 0.5
    docker cp "$cid:/tmp/marty-strace/." "$case_dir/container-strace" >/dev/null
  fi
  summarize_one "$label" "$case_dir" | tee "$case_dir/summary.txt"
}

mkdir -p "$OUT_DIR"
ensure_apache
wait_ready "$BASE/cgi-bin/marty-sync/primes/10000?salt=0"
wait_ready "$BASE/server-status?auto"

cid="$(dc ps -q bench)"
if [[ -z "$cid" ]]; then
  echo "error: bench container not running" >&2
  exit 1
fi

echo
echo "Apache CGI parallel diagnosis"
echo "  Base:        $BASE"
echo "  Route:       $TRACE_ROUTE"
echo "  Requests:    $TRACE_REQUESTS"
echo "  Concurrency: $TRACE_CONCURRENCY"
echo "  Strace:      $TRACE_STRACE"
echo "  Trace sec:   $TRACE_SECONDS"
echo "  Output:      $OUT_DIR"
echo

run_case "marty" "marty-sync" "$cid"
echo
run_case "go" "go-complex" "$cid"

echo
echo "Diagnosis files written to: $OUT_DIR"
