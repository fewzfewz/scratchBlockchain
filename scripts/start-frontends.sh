#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PIDS=()
VERBOSE=false

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Start all frontend development servers for the Modular Blockchain.

Options:
  -v, --verbose      Show server output in foreground (no background)
  -k, --kill         Stop all running frontend servers
  -l, --list         List running frontend servers
  -h, --help         Show this help

Port mapping:
  5173  frontend/            Vite dev server (npm run dev)
  3006  faucet/              Faucet backend service (scratch-blockchain faucet)
EOF
    exit 0
}

kill_all() {
    local ports=(5173 3006)
    local killed=0
    for port in "${ports[@]}"; do
        local pid
        pid=$(lsof -ti :"$port" 2>/dev/null || true)
        if [ -n "$pid" ]; then
            kill "$pid" 2>/dev/null || true
            echo "Killed server on port $port (PID $pid)"
            killed=1
        fi
    done
    [ $killed -eq 0 ] && echo "No frontend servers running."
    exit 0
}

list_servers() {
    local ports=(5173 3006)
    local running=0
    for port in "${ports[@]}"; do
        local pid
        pid=$(lsof -ti :"$port" 2>/dev/null || true)
        if [ -n "$pid" ]; then
            local pname
            pname=$(ps -p "$pid" -o comm= 2>/dev/null || echo "unknown")
            echo ":$port → PID $pid ($pname)"
            running=1
        fi
    done
    [ $running -eq 0 ] && echo "No frontend servers running."
    exit 0
}

start_vite() {
    local name="$1" dir="$2" port="$3"
    if ! [ -d "$ROOT/$dir/node_modules" ]; then
        echo "  SKIP  $name  (run 'cd $dir && npm install' first)"
        return
    fi
    if ! [ -f "$ROOT/$dir/node_modules/.bin/vite" ]; then
        echo "  SKIP  $name  (vite not installed in $dir/node_modules)"
        return
    fi
    echo "  START $name  → http://localhost:$port"
    if [ "$VERBOSE" = true ]; then
        npm --prefix "$ROOT/$dir" run dev
    else
        npm --prefix "$ROOT/$dir" run dev >/dev/null 2>&1 &
        PIDS+=($!)
        disown
    fi
}

start_faucet_backend() {
    local binary
    for candidate in "$ROOT/target/release/node" "$ROOT/target/debug/node"; do
        if [ -x "$candidate" ]; then
            binary="$candidate"
            break
        fi
    done
    if [ -z "${binary:-}" ]; then
        echo "  SKIP  faucet-backend  (binary not found; run 'cargo build' first)"
        return
    fi
    local name="faucet-backend"
    local port=3006
    echo "  START $name  → http://localhost:$port/faucet  (binary: $binary)"
    if [ "$VERBOSE" = true ]; then
        "$binary" faucet
    else
        "$binary" faucet >/dev/null 2>&1 &
        PIDS+=($!)
        disown
    fi
}

# Parse args
while [ $# -gt 0 ]; do
    case "$1" in
        -k|--kill) kill_all ;;
        -l|--list) list_servers ;;
        -v|--verbose) VERBOSE=true ;;
        -h|--help) usage ;;
        *) echo "Unknown option: $1"; usage ;;
    esac
    shift
done

echo "Starting frontend servers..."
echo ""

echo "Frontend:"
start_vite "frontend" "frontend" 5173

echo ""
echo "Backend services:"

start_faucet_backend

echo ""
echo ""

# Print helpful next steps
FAUCET_BINARY=""
for candidate in "$ROOT/target/release/node" "$ROOT/target/debug/node"; do
    if [ -x "$candidate" ]; then FAUCET_BINARY="$candidate"; break; fi
done
if [ -z "$FAUCET_BINARY" ]; then
    echo "  ℹ️  To start the faucet backend, build the node first:"
    echo "     cargo build"
    echo "     then re-run this script."
fi
echo "  🌐 Open http://localhost:5173 to access the unified frontend."
echo ""

wait
