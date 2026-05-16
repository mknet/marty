#!/usr/bin/env bash
# Shared Docker helpers for complex benchmarks (source, do not execute directly).

bench_host_port() {
  if [[ -n "${BENCH_PORT:-}" ]]; then
    echo "$BENCH_PORT"
    return
  fi
  local base="${1:-http://127.0.0.1:8081}"
  if [[ "$base" =~ :([0-9]+)(/|$) ]]; then
    echo "${BASH_REMATCH[1]}"
  else
    echo 8081
  fi
}

# Stop any running container publishing host port $1 (frees bind after compose down).
stop_docker_on_port() {
  local port="$1"
  local ids names
  ids="$(docker ps -q --filter "publish=${port}" 2>/dev/null || true)"
  if [[ -z "$ids" ]]; then
    return 0
  fi
  names="$(docker ps --filter "id=${ids}" --format '{{.Names}}' | tr '\n' ' ')"
  echo ">> freeing host port ${port}: stopping ${names}" >&2
  docker stop ${ids} >/dev/null
}
