# Validator Guide

Complete guide for running a validator on the Modular Blockchain Testnet.

## Requirements

### Hardware
- **CPU**: 4 cores (8 recommended)
- **RAM**: 8 GB (16 GB recommended)
- **Storage**: 100 GB SSD (500 GB recommended)
- **Network**: 100 Mbps (1 Gbps recommended)
- **Public IP**: Required

### Software
- **OS**: Ubuntu 20.04+ or similar Linux distribution
- **Rust**: Latest stable version
- **Git**: For cloning repositories

## Quick Setup

### Automated Setup

```bash
curl -sSL https://raw.githubusercontent.com/modular-blockchain/testnet/main/scripts/setup-validator.sh | bash
```

This script will:
1. Install dependencies
2. Build the node
3. Generate validator keys
4. Download genesis file
5. Create configuration
6. Set up systemd service

### Manual Setup

#### 1. Install Dependencies

```bash
sudo apt-get update
sudo apt-get install -y build-essential curl git jq
```

#### 2. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### 3. Clone and Build

```bash
git clone https://github.com/modular-blockchain/node.git
cd node
cargo build --release
```

#### 4. Generate Keys

```bash
./target/release/modular-node keys generate --output validator_key.json
```

**IMPORTANT**: Save your `validator_key.json` file securely! This contains your validator private key.

#### 5. Initialize Node

```bash
mkdir -p ~/.modular
./target/release/modular-node init --chain-id modular-testnet-1
```

#### 6. Download Genesis

```bash
curl -o ~/.modular/genesis.json \
    https://raw.githubusercontent.com/modular-blockchain/testnet/main/genesis.json
```

#### 7. Configure Node

Create `~/.modular/config.toml`:

```toml
[network]
chain_id = "modular-testnet-1"
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
key_file = "~/.modular/validator_key.json"
commission_rate = "0.10"

[storage]
data_dir = "~/.modular/data"
state_sync_enabled = true
state_sync_rpc = "https://rpc.testnet.modular.io"

[api]
enabled = true
address = "0.0.0.0:8545"
cors_origins = ["*"]

[metrics]
enabled = true
address = "0.0.0.0:9090"
```

#### 8. Create Systemd Service

```bash
sudo tee /etc/systemd/system/modular-validator.service > /dev/null << EOF
[Unit]
Description=Modular Blockchain Validator
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$HOME/node
ExecStart=$HOME/node/target/release/modular-node start --config ~/.modular/config.toml
Restart=on-failure
RestartSec=10
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
EOF
```

#### 9. Start Validator

```bash
sudo systemctl daemon-reload
sudo systemctl enable modular-validator
sudo systemctl start modular-validator
```

## Staking

### Get Test Tokens

1. Get your validator address:
   ```bash
   jq -r '.address' ~/.modular/validator_key.json
   ```

2. Request tokens from faucet:
   Visit https://faucet.testnet.modular.io

3. Verify balance:
   ```bash
   curl https://rpc.testnet.modular.io/balance/YOUR_ADDRESS
   ```

### Stake Tokens

```bash
./target/release/modular-node stake \
    --amount 1000000000000000000000 \
    --commission 0.10
```

## Monitoring

### Check Node Status

```bash
curl http://localhost:26657/status | jq
```

### View Logs

```bash
journalctl -u modular-validator -f
```

### Check Sync Status

```bash
curl http://localhost:26657/status | jq '.sync_info'
```

### Metrics

Access Prometheus metrics at:
```
http://localhost:9090/metrics
```

## Maintenance

### Update Node

```bash
cd ~/node
git pull
cargo build --release
sudo systemctl restart modular-validator
```

### Backup Keys

```bash
cp ~/.modular/validator_key.json ~/validator_key_backup.json
```

Store backup securely offline!

### Check Validator Info

```bash
curl https://rpc.testnet.modular.io/validator/YOUR_ADDRESS | jq
```

## Troubleshooting

### Node Not Syncing

1. Check network connectivity
2. Verify bootstrap nodes are reachable
3. Enable state sync in config
4. Check logs for errors

### High Memory Usage

1. Increase swap space
2. Reduce cache size in config
3. Enable state pruning

### Missed Blocks

1. Check system time synchronization
2. Verify network latency
3. Check CPU/disk performance
4. Review logs for errors

## Security

### Best Practices

1. **Firewall**: Only expose necessary ports
   ```bash
   sudo ufw allow 26656/tcp  # P2P
   sudo ufw allow 26657/tcp  # RPC (optional)
   sudo ufw enable
   ```

2. **SSH**: Disable password authentication
3. **Keys**: Store validator keys securely
4. **Updates**: Keep system updated
5. **Monitoring**: Set up alerts

### Sentry Nodes

For production, use sentry node architecture:
- Validator behind private network
- Public sentry nodes for P2P
- Firewall rules to restrict access

## Support

- **Discord**: #testnet-validators channel
- **Telegram**: @modular_validators
- **Email**: validators@modular.io

## Rewards

Testnet validators may receive:
- Uptime rewards
- Performance bonuses
- Mainnet allocations (TBD)

Track your performance:
https://monitor.testnet.modular.io/validator/YOUR_ADDRESS
