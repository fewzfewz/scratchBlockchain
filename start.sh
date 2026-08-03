#!/usr/bin/env bash
# Legacy quick start — use ./start-all.sh for the full Docker + bridge + frontend stack.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
echo "Tip: For the full stack (Docker + Hardhat + Bridge + UI), run:"
echo "  ./start-all.sh"
echo ""
exec "$ROOT/start-all.sh" "$@"
