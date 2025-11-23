#!/bin/bash
# Modular Blockchain Testnet Deployment Script

set -e

echo "🚀 Deploying Modular Blockchain Testnet"
echo "========================================"

# Configuration
NUM_NODES=${1:-3}
BASE_PORT=30333
BASE_RPC_PORT=9933
CONFIG_FILE="config/testnet.toml"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Number of nodes: $NUM_NODES${NC}"

# Build the project
echo -e "\n${GREEN}Building project...${NC}"
cargo build --release

# Create directories for each node
for i in $(seq 1 $NUM_NODES); do
    NODE_DIR="testnet/node$i"
    mkdir -p "$NODE_DIR"/{data,keys}
    
    echo -e "${GREEN}Setting up node $i in $NODE_DIR${NC}"
    
    # Generate validator keys
    ./target/release/node key-gen > "$NODE_DIR/keys/validator_key.json" 2>&1 || true
    
    # Calculate ports
    P2P_PORT=$((BASE_PORT + i - 1))
    RPC_PORT=$((BASE_RPC_PORT + i - 1))
    
    # Create node-specific config
    cat > "$NODE_DIR/config.toml" <<EOF
[network]
listen_addr = "/ip4/0.0.0.0/tcp/$P2P_PORT"

[rpc]
port = $RPC_PORT

[storage]
data_dir = "$NODE_DIR/data"
EOF
    
    echo "  - P2P Port: $P2P_PORT"
    echo "  - RPC Port: $RPC_PORT"
done

echo -e "\n${GREEN}✅ Testnet setup complete!${NC}"
echo ""
echo "To start the testnet nodes:"
echo "  Node 1: ./target/release/node start --config testnet/node1/config.toml"
echo "  Node 2: ./target/release/node start --config testnet/node2/config.toml"
echo "  Node 3: ./target/release/node start --config testnet/node3/config.toml"
echo ""
echo "Monitor metrics at:"
for i in $(seq 1 $NUM_NODES); do
    RPC_PORT=$((BASE_RPC_PORT + i - 1))
    echo "  Node $i: http://localhost:$RPC_PORT/metrics"
done
