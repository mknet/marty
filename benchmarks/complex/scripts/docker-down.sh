#!/usr/bin/env bash
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$HERE/docker-compose.yml"
COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-marty-bench-complex}"
BASE="${BENCH_BASE_URL:-http://127.0.0.1:8081}"

# shellcheck source=docker-lib.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/docker-lib.sh"

PORT="$(bench_host_port "$BASE")"
export BENCH_PORT="$PORT"

docker compose -p "$COMPOSE_PROJECT_NAME" -f "$COMPOSE_FILE" down --remove-orphans
stop_docker_on_port "$PORT"

echo "complex bench down (port ${PORT} should be free)"
