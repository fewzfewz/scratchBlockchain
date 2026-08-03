#!/usr/bin/env bash
# Wait until an HTTP endpoint responds successfully.
# Usage:
#   wait-for-url.sh http://localhost:8545/health [timeout_seconds]
#   wait-for-url.sh --json-rpc http://localhost:9545 [timeout_seconds]

set -euo pipefail

MODE="get"
URL=""
TIMEOUT=120

if [ "${1:-}" = "--json-rpc" ]; then
  MODE="json-rpc"
  URL="${2:-}"
  TIMEOUT="${3:-120}"
else
  URL="${1:-}"
  TIMEOUT="${2:-120}"
fi

if [ -z "$URL" ]; then
  echo "Usage: $0 [--json-rpc] URL [timeout_seconds]" >&2
  exit 1
fi

echo "Waiting for $URL (timeout ${TIMEOUT}s)..."
start=$(date +%s)

while true; do
  now=$(date +%s)
  if [ $((now - start)) -ge "$TIMEOUT" ]; then
    echo "Timeout waiting for $URL" >&2
    exit 1
  fi

  if [ "$MODE" = "json-rpc" ]; then
    if curl -sf -X POST "$URL" \
      -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
      >/dev/null 2>&1; then
      echo "OK: $URL"
      exit 0
    fi
  else
    if curl -sf "$URL" >/dev/null 2>&1; then
      echo "OK: $URL"
      exit 0
    fi
  fi

  sleep 2
done
