#!/usr/bin/env bash
# Start the full Nebula stack: Docker testnet + Hardhat + Bridge deploy + Frontend
#
# Usage:
#   ./start-all.sh              # build node, start compose, deploy bridge, print URLs
#   ./start-all.sh --no-build   # skip cargo build
#   ./start-all.sh --no-bridge  # skip Bridge.sol deploy
#   ./start-all.sh --local-frontend  # run Vite on host instead of Docker frontend
#
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
COMPOSE_FILE="$ROOT/deployment/local/docker-compose.yml"
SCRIPTS="$ROOT/deployment/local/scripts"
LOG_DIR="$ROOT/.nebula/logs"
PID_FILE="$ROOT/.nebula/pids"

# Prefer `docker compose` (v2 plugin); fall back to standalone `docker-compose`.
if docker compose version >/dev/null 2>&1; then
  DC="docker compose"
else
  DC="docker-compose"
fi

# The validator images mount the host-built node binary (target/release/node),
# so only trigger the (slow) in-Docker Rust build when the image is missing.
if docker image inspect local-validator1 >/dev/null 2>&1; then
  BUILD_FLAG=""
else
  BUILD_FLAG="--build"
fi

DO_BUILD=true
DO_BRIDGE=true
LOCAL_FRONTEND=false

for arg in "$@"; do
  case "$arg" in
    --no-build) DO_BUILD=false ;;
    --no-bridge) DO_BRIDGE=false ;;
    --local-frontend) LOCAL_FRONTEND=true ;;
    -h|--help)
      grep '^#' "$0" | tail -n +2 | head -n -1
      exit 0
      ;;
    *) echo "Unknown option: $arg"; exit 1 ;;
  esac
done

mkdir -p "$LOG_DIR" "$PID_FILE"

# Export host UID/GID so the hardhat container can chown its host-mounted
# build outputs (artifacts/cache/node_modules) back to the host user.
export HOST_UID="$(id -u)"
export HOST_GID="$(id -g)"

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║           Nebula — Full Stack Startup                    ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# ── Prerequisites ──────────────────────────────────────────────────────────
if ! command -v docker >/dev/null 2>&1; then
  echo "❌ Docker is required. Install Docker and try again."
  exit 1
fi
if ! docker info >/dev/null 2>&1; then
  echo "❌ Docker daemon is not running."
  exit 1
fi

# ── Build Nebula node binary (mounted into containers) ───────────────────────
if [ "$DO_BUILD" = true ]; then
  echo "▶ [1/6] Building Nebula node (release)..."
  cargo build --release -p node 2>&1 | tail -5
  echo "   ✓ target/release/node"
else
  echo "▶ [1/6] Skipping cargo build (--no-build)"
  if [ ! -x "$ROOT/target/release/node" ]; then
    echo "❌ target/release/node not found. Run without --no-build first."
    exit 1
  fi
fi

# ── Docker Compose ─────────────────────────────────────────────────────────
echo ""
echo "▶ [2/6] Starting Docker stack (validators, RPC, Hardhat, monitoring)..."

COMPOSE_SERVICES=(hardhat validator1 validator2 validator3 rpc1 rpc2 prometheus alertmanager grafana faucet nginx)
if [ "$LOCAL_FRONTEND" = false ]; then
  COMPOSE_SERVICES+=(frontend)
fi

$DC -f "$COMPOSE_FILE" up -d $BUILD_FLAG "${COMPOSE_SERVICES[@]}"

# ── Wait for services ───────────────────────────────────────────────────────
echo ""
echo "▶ [3/6] Waiting for Nebula RPC..."
chmod +x "$SCRIPTS/wait-for-url.sh"
"$SCRIPTS/wait-for-url.sh" "http://localhost:8545/health" 180

echo "▶ [4/6] Waiting for Hardhat (Ethereum)..."
"$SCRIPTS/wait-for-url.sh" --json-rpc "http://localhost:9545" 120

# ── Deploy Bridge.sol ───────────────────────────────────────────────────────
if [ "$DO_BRIDGE" = true ]; then
  echo ""
  echo "▶ [5/6] Deploying Bridge.sol to Hardhat..."
  chmod +x "$SCRIPTS/deploy-bridge.sh"
  "$SCRIPTS/deploy-bridge.sh"
else
  echo ""
  echo "▶ [5/6] Skipping bridge deploy (--no-bridge)"
fi

# ── Local frontend (optional) ───────────────────────────────────────────────
if [ "$LOCAL_FRONTEND" = true ]; then
  echo ""
  echo "▶ [6/6] Starting frontend on host (port 5173)..."
  $DC -f "$COMPOSE_FILE" stop frontend 2>/dev/null || true
  if [ ! -d "$ROOT/frontend/node_modules" ]; then
    npm --prefix "$ROOT/frontend" install --legacy-peer-deps
  fi
  npm --prefix "$ROOT/frontend" run dev > "$LOG_DIR/frontend.log" 2>&1 &
  echo "$!" > "$PID_FILE/frontend.pid"
  "$SCRIPTS/wait-for-url.sh" "http://localhost:5173" 60
else
  echo ""
  echo "▶ [6/6] Waiting for Docker frontend..."
  "$SCRIPTS/wait-for-url.sh" "http://localhost:5173" 120 || echo "   ⚠ Frontend still starting — check: docker logs nebula-frontend"
fi

# ── Summary ─────────────────────────────────────────────────────────────────
BRIDGE_ADDR=""
if [ -f "$ROOT/frontend/public/bridge-config.json" ]; then
  BRIDGE_ADDR=$(grep -o '"bridge"[[:space:]]*:[[:space:]]*"[^"]*"' "$ROOT/frontend/public/bridge-config.json" | sed 's/.*"\(0x[^"]*\)".*/\1/' || true)
fi

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║                    🚀 Nebula is ready                     ║"
echo "╠══════════════════════════════════════════════════════════╣"
echo "║  Frontend (UI)     http://localhost:5173                 ║"
echo "║  Nebula RPC        http://localhost:8545                 ║"
echo "║  Ethereum (Hardhat) http://localhost:9545                ║"
echo "║  Bridge UI         http://localhost:5173/bridge          ║"
echo "║  Explorer          http://localhost:5173/explorer        ║"
echo "║  API Docs          http://localhost:5173/api-docs        ║"
echo "║  Grafana           http://localhost:3000  (admin/admin)  ║"
echo "║  Prometheus        http://localhost:9095                 ║"
echo "╠══════════════════════════════════════════════════════════╣"
if [ -n "$BRIDGE_ADDR" ] && [ "$BRIDGE_ADDR" != "" ]; then
  echo "║  Bridge.sol        $BRIDGE_ADDR"
else
  echo "║  Bridge.sol        (deploy with ./start-all.sh or deploy-bridge.sh)"
fi
echo "╠══════════════════════════════════════════════════════════╣"
echo "║  Stop everything:  ./stop-all.sh                         ║"
echo "║  Docker logs:      docker compose -f deployment/local/   ║"
echo "║                    docker-compose.yml logs -f            ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""
