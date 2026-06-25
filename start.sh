#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
BINARY="$ROOT/target/debug/node"

echo "=== Scratch Blockchain Quick Start ==="

# ── 1. Build Rust node ──
echo ""
echo "[1/3] Building Rust node..."
cargo build -p node 2>&1 | tail -3

# ── 2. Install frontend deps ──
echo ""
echo "[2/3] Installing frontend dependencies..."
npm --prefix "$ROOT/frontend" install --silent

# ── 3. Start node + frontend ──
echo ""
echo "[3/3] Starting services..."
echo ""

# Start the node in background
"$BINARY" start --genesis "$ROOT/genesis.json" > /tmp/scratch-node.log 2>&1 &
NODE_PID=$!
echo "  Node     → http://localhost:9933  (PID $NODE_PID)"

# Start frontend dev server in background
npm --prefix "$ROOT/frontend" run dev > /tmp/scratch-frontend.log 2>&1 &
FRONTEND_PID=$!
echo "  Frontend → http://localhost:5173  (PID $FRONTEND_PID)"

echo ""
echo "=== Ready! ==="
echo "  Frontend:  http://localhost:5173"
echo "  Node RPC:  http://localhost:9933"
echo "  API Docs:  http://localhost:5173/api-docs"
echo ""
echo "  Stop with:  kill $NODE_PID $FRONTEND_PID"
echo "  Logs:       tail -f /tmp/scratch-node.log"
echo "              tail -f /tmp/scratch-frontend.log"

# Handle Ctrl-C gracefully
trap "kill $NODE_PID $FRONTEND_PID 2>/dev/null; exit" INT TERM

# Wait for either process to exit
wait
