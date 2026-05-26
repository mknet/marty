#!/usr/bin/env bash
# Build a Docker benchmark image with Rust PGO for marty-complex.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$HERE/docker-compose.yml"
COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-marty-bench-complex-pgo}"
BASE="${BENCH_BASE_URL:-http://127.0.0.1:8081}"
PGO_DIR="${PGO_DIR:-$HERE/.pgo}"
RAW_DIR="$PGO_DIR/raw"
MERGED_PROFILE="$PGO_DIR/merged.profdata"
LLVM_PROFILE_FILE="${LLVM_PROFILE_FILE:-/tmp/marty-pgo/marty-%p-%m.profraw}"
PGO_REQUESTS="${PGO_REQUESTS:-1000}"
PGO_CONCURRENCY="${PGO_CONCURRENCY:-10}"
PGO_TARGET_CPU="${PGO_TARGET_CPU:-${MARTY_TARGET_CPU:-native}}"

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

find_llvm_profdata() {
  if command -v rustup >/dev/null 2>&1; then
    rustup component add llvm-tools-preview >/dev/null
    local host sysroot candidate
    host="$(rustc -vV | awk '/host:/ { print $2 }')"
    sysroot="$(rustc --print sysroot)"
    candidate="$sysroot/lib/rustlib/$host/bin/llvm-profdata"
    if [[ -x "$candidate" ]]; then
      echo "$candidate"
      return
    fi
  fi
  if command -v llvm-profdata >/dev/null 2>&1; then
    command -v llvm-profdata
    return
  fi
  if command -v xcrun >/dev/null 2>&1 && xcrun -f llvm-profdata >/dev/null 2>&1; then
    xcrun -f llvm-profdata
    return
  fi
  echo "error: llvm-profdata not found" >&2
  exit 1
}

build_image() {
  local phase="$1"
  echo ">> docker compose build (${phase})"
  dc build
}

run_training() {
  local url
  echo ">> docker compose up -d (instrumented image)"
  dc up -d --force-recreate
  wait_ready "${BASE}/cgi-bin/marty-complex/primes/10000?salt=0"

  for url in \
    "${BASE}/cgi-bin/marty-complex/primes/${BENCH_PRIME_LIMIT:-400000}" \
    "${BASE}/cgi-bin/marty-complex/fibonacci/${BENCH_FIB_N:-42}" \
    "${BASE}/cgi-bin/marty-complex/matrix/${BENCH_MATRIX_SIZE:-128}"
  do
    echo ">> PGO train ${url}"
    hey -n "$PGO_REQUESTS" -c "$PGO_CONCURRENCY" -m GET "$url" >/dev/null
  done
}

collect_profiles() {
  local cid
  cid="$(dc ps -q bench)"
  if [[ -z "$cid" ]]; then
    echo "error: bench container not running" >&2
    exit 1
  fi
  rm -rf "$RAW_DIR"
  mkdir -p "$RAW_DIR"
  docker cp "$cid:/tmp/marty-pgo/." "$RAW_DIR/"
  if ! compgen -G "$RAW_DIR/*.profraw" >/dev/null; then
    echo "error: no .profraw files collected from /tmp/marty-pgo" >&2
    exit 1
  fi
}

merge_profiles() {
  local profdata
  profdata="$(find_llvm_profdata)"
  echo ">> merging PGO profiles with ${profdata}"
  "$profdata" merge -o "$MERGED_PROFILE" "$RAW_DIR"/*.profraw
}

main() {
  if ! command -v docker >/dev/null 2>&1; then
    echo "error: docker not on PATH" >&2
    exit 1
  fi

  mkdir -p "$PGO_DIR"
  export BENCH_PORT="$(bench_host_port "$BASE")"
  export BENCH_TIMING=0
  export LLVM_PROFILE_FILE

  dc down --remove-orphans 2>/dev/null || true
  stop_docker_on_port "$BENCH_PORT"

  export MARTY_TARGET_CPU="$PGO_TARGET_CPU"
  export MARTY_PGO_PROFILE=""
  export MARTY_RUSTFLAGS="${PGO_GENERATE_RUSTFLAGS:--Cprofile-generate=/tmp/marty-pgo}"
  build_image "profile-generate, target-cpu=${MARTY_TARGET_CPU}"
  run_training
  collect_profiles
  merge_profiles

  dc down --remove-orphans

  export MARTY_RUSTFLAGS="${PGO_FINAL_RUSTFLAGS:-}"
  export MARTY_PGO_PROFILE="benchmarks/complex/.pgo/merged.profdata"
  build_image "profile-use, target-cpu=${MARTY_TARGET_CPU}"

  echo ">> PGO image built for project ${COMPOSE_PROJECT_NAME}"
  echo "   profile: ${MERGED_PROFILE}"
  echo "   run: COMPOSE_PROJECT_NAME=${COMPOSE_PROJECT_NAME} BENCH_TIMING=0 just bench-fair-throughput"
}

main "$@"
