#!/usr/bin/env bash
set -euo pipefail

#=====================================================================
#  Modular Blockchain — Validator Setup Script
#  This script automates the full validator node setup process.
#  Run on a fresh Ubuntu 22.04 VM as root or with sudo.
#=====================================================================

# ---- Configuration ----
RPC_URL="${RPC_URL:-https://rpc.testnet.modular-blockchain.io}"
CHAIN_ID="${CHAIN_ID:-modular-testnet-1}"
DATA_DIR="${DATA_DIR:-/data/blockchain}"
VALIDATOR_KEY_PATH="${VALIDATOR_KEY_PATH:-$HOME/.modular/validator_key.json}"
NODE_HOME="${NODE_HOME:-$HOME/.modular}"
BOOTSTRAP_NODES="${BOOTSTRAP_NODES:-/dns4/bootstrap.testnet.modular-blockchain.io/tcp/26656}"
RELEASE_URL="${RELEASE_URL:-https://github.com/your-org/modular-blockchain/releases/latest/download/modular-node-linux-amd64.tar.gz}"
GENESIS_URL="${GENESIS_URL:-https://raw.githubusercontent.com/your-org/modular-blockchain/main/deployment/cloud/configs/genesis.json}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log()  { echo -e "${GREEN}[✓]${NC} $1"; }
warn() { echo -e "${YELLOW}[!]${NC} $1"; }
err()  { echo -e "${RED}[✗]${NC} $1"; exit 1; }
info() { echo -e "${BLUE}[i]${NC} $1"; }

# ---- Step 1: System Prerequisites ----
setup_prerequisites() {
  echo ""
  info "Step 1: Installing system prerequisites..."

  apt-get update -qq
  apt-get install -y -qq \
    curl wget jq openssl \
    ufw unattended-upgrades \
    chrony tmux htop \
    ca-certificates gnupg lsb-release

  # Set up chrony for accurate timekeeping
  systemctl enable chrony --now

  log "System prerequisites installed."
}

# ---- Step 2: Create User ----
setup_user() {
  echo ""
  info "Step 2: Creating dedicated user..."

  if ! id -u modular &>/dev/null; then
    useradd -r -s /sbin/nologin -m -d /home/modular modular
    log "User 'modular' created."
  else
    warn "User 'modular' already exists."
  fi
}

# ---- Step 3: Download and Install Node Binary ----
install_binary() {
  echo ""
  info "Step 3: Installing node binary..."

  local tmpdir=$(mktemp -d)
  cd "$tmpdir"

  curl -fsSL "$RELEASE_URL" -o modular-node.tar.gz
  tar xzf modular-node.tar.gz
  mv modular-node /usr/local/bin/modular-node
  chmod +x /usr/local/bin/modular-node

  cd / && rm -rf "$tmpdir"

  log "Node binary installed: $(modular-node --version 2>/dev/null || echo 'version check pending')"
}

# ---- Step 4: Generate Validator Key ----
generate_key() {
  echo ""
  info "Step 4: Generating validator key..."

  mkdir -p "$(dirname "$VALIDATOR_KEY_PATH")"

  if [ -f "$VALIDATOR_KEY_PATH" ]; then
    warn "Validator key already exists at $VALIDATOR_KEY_PATH"
    cat "$VALIDATOR_KEY_PATH"
  else
    modular-node keygen --output "$VALIDATOR_KEY_PATH"
    log "Validator key generated at $VALIDATOR_KEY_PATH"
    echo ""
    cat "$VALIDATOR_KEY_PATH"
    echo ""
    warn "BACK UP THIS KEY! Store it offline and encrypted."
  fi
}

# ---- Step 5: Create Data Directory ----
setup_data_dir() {
  echo ""
  info "Step 5: Creating data directory..."

  mkdir -p "$DATA_DIR"
  chown -R modular:modular "$DATA_DIR"
  log "Data directory: $DATA_DIR"
}

# ---- Step 6: Download Genesis File ----
download_genesis() {
  echo ""
  info "Step 6: Downloading genesis file..."

  mkdir -p /etc/modular

  if [ -f /etc/modular/genesis.json ]; then
    warn "Genesis file already exists at /etc/modular/genesis.json"
  else
    curl -fsSL "$GENESIS_URL" -o /etc/modular/genesis.json
    log "Genesis file downloaded."
  fi
}

# ---- Step 7: Create Configuration ----
create_config() {
  echo ""
  info "Step 7: Creating node configuration..."

  cat > /etc/modular/config.toml <<CONFIG
[network]
chain_id = "$CHAIN_ID"
p2p_port = 26656
rpc_port = 26657
bootstrap_nodes = ["$BOOTSTRAP_NODES"]

[consensus]
block_time_ms = 3000
max_validators = 100

[validator]
enabled = true

[storage]
data_dir = "$DATA_DIR"

[api]
enabled = false
address = "127.0.0.1:8545"

[metrics]
enabled = true
address = "0.0.0.0:9090"
CONFIG

  chmod 644 /etc/modular/config.toml
  log "Configuration created at /etc/modular/config.toml"
}

# ---- Step 8: Create Systemd Service ----
create_systemd() {
  echo ""
  info "Step 8: Creating systemd service..."

  cat > /etc/systemd/system/modular-node.service <<SERVICE
[Unit]
Description=Modular Blockchain Validator
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=modular
Group=modular
ExecStart=/usr/local/bin/modular-node start \
  --config /etc/modular/config.toml \
  --genesis /etc/modular/genesis.json \
  --validator-key $VALIDATOR_KEY_PATH
Restart=always
RestartSec=10
LimitNOFILE=65535
LimitNPROC=4096
MemoryMax=16G
CPUQuota=80%
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
SERVICE

  systemctl daemon-reload
  log "Systemd service created."
}

# ---- Step 9: Configure Firewall ----
setup_firewall() {
  echo ""
  info "Step 9: Configuring firewall..."

  ufw --force reset
  ufw default deny incoming
  ufw default allow outgoing
  ufw allow 26656/tcp comment 'P2P'
  ufw allow 22/tcp comment 'SSH'
  ufw --force enable

  log "Firewall configured."
  ufw status verbose
}

# ---- Step 10: Enable Automatic Security Updates ----
setup_security() {
  echo ""
  info "Step 10: Enabling automatic security updates..."

  cat > /etc/apt/apt.conf.d/20auto-upgrades <<APT
APT::Periodic::Update-Package-Lists "1";
APT::Periodic::Download-Upgradeable-Packages "1";
APT::Periodic::AutocleanInterval "7";
APT::Periodic::Unattended-Upgrade "1";
APT

  log "Automatic security updates enabled."
}

# ---- Step 11: Start the Node ----
start_node() {
  echo ""
  info "Step 11: Starting the node..."

  systemctl enable modular-node
  systemctl start modular-node

  log "Node started! Check status: systemctl status modular-node"

  echo ""
  info "Waiting for node to initialize..."
  sleep 5

  if curl -sf http://localhost:26657/health > /dev/null 2>&1; then
    log "Node is healthy!"
  else
    warn "Node may still be starting. Check: journalctl -u modular-node -f"
  fi
}

# ---- Summary ----
print_summary() {
  echo ""
  echo "============================================"
  echo "  Validator Setup Complete!"
  echo "============================================"
  echo ""
  echo "  Public Key:"
  [ -f "$VALIDATOR_KEY_PATH" ] && jq -r '.public_key' "$VALIDATOR_KEY_PATH" 2>/dev/null || echo "  (not found)"
  echo ""
  echo "  Next Steps:"
  echo "  1. Fund your validator account via faucet:"
  echo "     curl -X POST https://faucet.testnet.modular-blockchain.io \\"
  echo "       -H 'Content-Type: application/json' \\"
  echo "       -d '{\"address\": \"0x<YOUR_ADDRESS>\"}'"
  echo ""
  echo "  2. Register your validator:"
  echo "     modular-node tx register-validator \\"
  echo "       --public-key <PUBLIC_KEY> \\"
  echo "       --stake 100000 \\"
  echo "       --commission-rate 0.10 \\"
  echo "       --rpc $RPC_URL"
  echo ""
  echo "  3. Monitor your node:"
  echo "     journalctl -u modular-node -f"
  echo ""
  echo "  4. View validator docs:"
  echo "     docs/validator-onboarding.md"
  echo ""
  echo "============================================"
}

# ---- Main ----
main() {
  echo ""
  echo "============================================"
  echo "  Modular Blockchain Validator Setup"
  echo "============================================"
  echo ""

  if [ "$EUID" -ne 0 ]; then
    err "This script must be run as root. Use: sudo $0"
  fi

  setup_prerequisites
  setup_user
  install_binary
  generate_key
  setup_data_dir
  download_genesis
  create_config
  create_systemd
  setup_firewall
  setup_security
  start_node
  print_summary
}

main "$@"
