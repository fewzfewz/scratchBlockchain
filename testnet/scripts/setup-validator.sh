#!/bin/bash
set -e

echo "==================================="
echo "Modular Blockchain Validator Setup"
echo "==================================="
echo ""

# Check if running as root
if [ "$EUID" -eq 0 ]; then 
   echo "Please do not run as root"
   exit 1
fi

# Configuration
CHAIN_ID="modular-testnet-1"
NODE_DIR="$HOME/.modular"
GENESIS_URL="https://raw.githubusercontent.com/modular-blockchain/testnet/main/genesis.json"

# Step 1: Install dependencies
echo "Step 1: Installing dependencies..."
sudo apt-get update
sudo apt-get install -y build-essential curl git jq

# Step 2: Install Rust
if ! command -v rustc &> /dev/null; then
    echo "Step 2: Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo "Step 2: Rust already installed"
fi

# Step 3: Clone and build node
echo "Step 3: Building node..."
if [ ! -d "$HOME/modular-node" ]; then
    git clone https://github.com/modular-blockchain/node.git $HOME/modular-node
fi

cd $HOME/modular-node
git pull
cargo build --release

# Step 4: Initialize node
echo "Step 4: Initializing node..."
mkdir -p $NODE_DIR

# Generate validator keys if they don't exist
if [ ! -f "$NODE_DIR/validator_key.json" ]; then
    echo "Generating validator keys..."
    $HOME/modular-node/target/release/modular-node keys generate \
        --output $NODE_DIR/validator_key.json
    
    echo ""
    echo "IMPORTANT: Save your validator key!"
    echo "Location: $NODE_DIR/validator_key.json"
    echo ""
fi

# Step 5: Download genesis
echo "Step 5: Downloading genesis file..."
curl -o $NODE_DIR/genesis.json $GENESIS_URL

# Step 6: Create configuration
echo "Step 6: Creating configuration..."
cat > $NODE_DIR/config.toml << EOF
[network]
chain_id = "$CHAIN_ID"
p2p_port = 26656
rpc_port = 26657
bootstrap_nodes = [
    "/dns4/testnet-seed-1.modular.io/tcp/26656/p2p/12D3KooWExample1",
    "/dns4/testnet-seed-2.modular.io/tcp/26656/p2p/12D3KooWExample2",
]

[consensus]
block_time_ms = 3000
max_validators = 100
min_stake = "1000000000000000000000"

[validator]
enabled = true
key_file = "$NODE_DIR/validator_key.json"
commission_rate = "0.10"

[storage]
data_dir = "$NODE_DIR/data"
state_sync_enabled = true
state_sync_rpc = "https://rpc.testnet.modular.io"

[api]
enabled = true
address = "0.0.0.0:8545"
cors_origins = ["*"]

[metrics]
enabled = true
address = "0.0.0.0:9090"
EOF

# Step 7: Create systemd service
echo "Step 7: Creating systemd service..."
sudo tee /etc/systemd/system/modular-validator.service > /dev/null << EOF
[Unit]
Description=Modular Blockchain Validator
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$HOME/modular-node
ExecStart=$HOME/modular-node/target/release/modular-node start --config $NODE_DIR/config.toml
Restart=on-failure
RestartSec=10
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
EOF

# Step 8: Enable and start service
echo "Step 8: Enabling service..."
sudo systemctl daemon-reload
sudo systemctl enable modular-validator

echo ""
echo "==================================="
echo "Setup Complete!"
echo "==================================="
echo ""
echo "Your validator address:"
jq -r '.address' $NODE_DIR/validator_key.json
echo ""
echo "To start your validator:"
echo "  sudo systemctl start modular-validator"
echo ""
echo "To view logs:"
echo "  journalctl -u modular-validator -f"
echo ""
echo "To check status:"
echo "  curl http://localhost:26657/status"
echo ""
echo "Next steps:"
echo "1. Fund your validator address with test tokens from the faucet"
echo "2. Start the validator service"
echo "3. Monitor the logs to ensure it's syncing"
echo ""
