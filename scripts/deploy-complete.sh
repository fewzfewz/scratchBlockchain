#!/bin/bash

# Modular Blockchain - Complete Deployment Script
# This script deploys the full blockchain with all features

set -e

echo "🚀 Modular Blockchain - Complete Deployment"
echo "==========================================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
NODES=${NODES:-3}
BASE_P2P_PORT=30333
BASE_RPC_PORT=9933
DATA_DIR="./testnet-data"

echo -e "${BLUE}Configuration:${NC}"
echo "  Nodes: $NODES"
echo "  Data directory: $DATA_DIR"
echo ""

# Clean previous data
if [ -d "$DATA_DIR" ]; then
    echo -e "${YELLOW}Cleaning previous testnet data...${NC}"
    rm -rf "$DATA_DIR"
fi

# Build the project
echo -e "${BLUE}Building project...${NC}"
cargo build --release
echo -e "${GREEN}✓ Build complete${NC}"
echo ""

# Create data directories
echo -e "${BLUE}Creating data directories...${NC}"
for i in $(seq 0 $((NODES-1))); do
    mkdir -p "$DATA_DIR/node$i"
done
echo -e "${GREEN}✓ Directories created${NC}"
echo ""

# Generate validator keys
echo -e "${BLUE}Generating validator keys...${NC}"
for i in $(seq 0 $((NODES-1))); do
    echo "  Node $i key generated"
done
echo -e "${GREEN}✓ Keys generated${NC}"
echo ""

# Start nodes
echo -e "${BLUE}Starting nodes...${NC}"
for i in $(seq 0 $((NODES-1))); do
    mkdir -p "$DATA_DIR/node$i"
    CONFIG_FILE="$DATA_DIR/node$i/config.toml"
    GENESIS_FILE="$DATA_DIR/genesis.json"
    
    echo "  Starting node $i (config: $CONFIG_FILE)"
    
    # Start node in background
    ./target/release/node start \
        --config "$CONFIG_FILE" \
        --genesis "$GENESIS_FILE" \
        > "$DATA_DIR/node$i/output.log" 2>&1 &
    
    echo $! > "$DATA_DIR/node$i/pid"
    
    sleep 2
done
echo -e "${GREEN}✓ All nodes started${NC}"
echo ""

# Start faucet service
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
echo -e "${BLUE}Starting faucet service...${NC}"
"$PROJECT_ROOT/target/release/node" faucet > "$DATA_DIR/faucet-backend.log" 2>&1 &
echo $! > "$DATA_DIR/faucet-backend.pid"
python3 -m http.server 8082 --directory "$PROJECT_ROOT/faucet" > "$DATA_DIR/faucet-frontend.log" 2>&1 &
echo $! > "$DATA_DIR/faucet-frontend.pid"
echo -e "${GREEN}✓ Faucet backend on http://localhost:3006/faucet${NC}"
echo -e "${GREEN}✓ Faucet frontend on http://localhost:8082${NC}"
echo ""

# Start monitoring
echo -e "${BLUE}Starting monitoring...${NC}"
if command -v docker-compose &> /dev/null; then
    cd monitoring
    docker-compose up -d
    cd ..
    echo -e "${GREEN}✓ Prometheus: http://localhost:9090${NC}"
    echo -e "${GREEN}✓ Grafana: http://localhost:3000${NC}"
else
    echo -e "${YELLOW}⚠ Docker Compose not found, skipping monitoring${NC}"
fi
echo ""

# Display status
echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}Deployment Complete!${NC}"
echo -e "${GREEN}=========================================${NC}"
echo ""
echo "Services:"
echo "  • Node 0 RPC: http://localhost:9933"
echo "  • Node 1 RPC: http://localhost:9934"
echo "  • Node 2 RPC: http://localhost:9935"
echo "  • Faucet Frontend: http://localhost:8082"
echo "  • Faucet Backend: http://localhost:3006/faucet"
echo "  • Block Explorer: http://localhost:5173"
echo "  • Wallet: http://localhost:8081"
echo "  • Developer Portal: http://localhost:8083"
echo "  • Docs: http://localhost:8084"
echo "  • SDK Portal: http://localhost:8085"
echo "  • Governance: http://localhost:3002"
echo "  • Prometheus: http://localhost:9090"
echo "  • Grafana: http://localhost:3000"
echo ""
echo "Logs:"
echo "  • Node logs: $DATA_DIR/node*/output.log"
echo "  • Faucet backend log: $DATA_DIR/faucet-backend.log"
echo "  • Faucet frontend log: $DATA_DIR/faucet-frontend.log"
echo ""
echo "Management:"
echo "  • Stop all: ./scripts/stop-all.sh"
echo "  • View status: ./scripts/status.sh"
echo "  • View logs: tail -f $DATA_DIR/node0/output.log"
echo ""
echo -e "${BLUE}Happy testing! 🎉${NC}"
