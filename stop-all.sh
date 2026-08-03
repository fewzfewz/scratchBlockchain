#!/usr/bin/env bash
# Stop the full Nebula stack
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
COMPOSE_FILE="$ROOT/deployment/local/docker-compose.yml"
PID_FILE="$ROOT/.nebula/pids"

echo "Stopping Nebula stack..."

# Host frontend started by start-all.sh --local-frontend
if [ -f "$PID_FILE/frontend.pid" ]; then
  pid=$(cat "$PID_FILE/frontend.pid")
  kill "$pid" 2>/dev/null && echo "Stopped host frontend (PID $pid)" || true
  rm -f "$PID_FILE/frontend.pid"
fi

# Kill anything on 5173 if still running
if command -v lsof >/dev/null 2>&1; then
  lsof -ti :5173 2>/dev/null | xargs -r kill 2>/dev/null || true
fi

docker compose -f "$COMPOSE_FILE" down

echo "✓ All services stopped."
