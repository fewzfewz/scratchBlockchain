# Validator Onboarding Guide

This guide covers everything you need to know to run a validator on the Modular Blockchain testnet.

---

## Table of Contents

1. [Hardware Requirements](#hardware-requirements)
2. [Prerequisites](#prerequisites)
3. [Key Management](#key-management)
4. [Node Installation](#node-installation)
5. [Validator Registration](#validator-registration)
6. [Starting Your Node](#starting-your-node)
7. [Monitoring Your Validator](#monitoring-your-validator)
8. [Validator Operations](#validator-operations)
9. [Delegation & Rewards](#delegation--rewards)
10. [Security Best Practices](#security-best-practices)
11. [Troubleshooting](#troubleshooting)

---

## Hardware Requirements

### Minimum Requirements
| Component | Validator | RPC Node |
|-----------|-----------|----------|
| CPU | 4 cores @ 2.5+ GHz | 8 cores @ 3.0+ GHz |
| RAM | 8 GB | 16 GB |
| Storage | 100 GB SSD (NVMe recommended) | 200 GB SSD (NVMe recommended) |
| Network | 100 Mbps | 1 Gbps |
| Monthly traffic | ~500 GB | ~2 TB |

### Recommended Requirements
| Component | Validator | RPC Node |
|-----------|-----------|----------|
| CPU | 8 cores @ 3.0+ GHz (AMD EPYC / Intel Xeon) | 16 cores @ 3.5+ GHz |
| RAM | 16 GB ECC | 32 GB ECC |
| Storage | 500 GB NVMe SSD | 1 TB NVMe SSD |
| Network | 500 Mbps dedicated | 10 Gbps |
| Monthly traffic | ~2 TB | ~10 TB |

### Cloud Instance Reference

| Provider | Validator SKU | vCPU | RAM | Cost/Month |
|----------|---------------|------|-----|------------|
| AWS | `t3.large` / `c6i.large` | 2-4 | 8-16 GB | ~$70-130 |
| GCP | `n1-standard-4` / `n2-standard-4` | 4 | 16 GB | ~$120-150 |
| Azure | `Standard_D4s_v5` | 4 | 16 GB | ~$130 |
| DigitalOcean | 4GB / 8GB Droplet | 2-4 | 4-8 GB | ~$24-48 |

### Disk Performance
- **IOPS requirement**: >3,000 IOPS (sustained)
- **Throughput**: >200 MB/s
- **Recommended**: AWS gp3, GCP pd-ssd, Azure Premium SSD, or local NVMe

---

## Prerequisites

### Software
- **OS**: Ubuntu 22.04 LTS (recommended), Debian 12, or any modern Linux distribution
- **Dependencies**: `curl`, `jq`, `openssl` (pre-installed on most distributions)
- **Optional**: `ansible`, `docker`, `docker-compose` (for advanced deployments)

### Network
- Open **TCP 26656** (P2P) — must be publicly reachable
- Open **TCP 26657** (RPC) — optional, for querying
- Open **TCP 8545** (API) — optional, for JSON-RPC
- Open **TCP 9090** (Metrics) — optional, for Prometheus scraping

### Accounts
- An account with at least 100,000 NBL for the validator stake
- Testnet tokens available from the [faucet](https://faucet.testnet.modular-blockchain.io)

---

## Key Management

> **Warning**: Never share your private key or validator key file. Loss of your validator key will result in loss of validator control and potential slashing.

### Validator Key Types

| Key | Purpose | Storage |
|-----|---------|---------|
| **Validator Consensus Key** | Signs blocks and votes | `~/.modular/validator_key.json` |
| **Validator Account Key** | Controls staking, rewards, and governance | Wallet (software or hardware) |
| **Node P2P Key** | Authenticates peer-to-peer connections | `~/.modular/node_key.json` (auto-generated) |

### Generating a Validator Key

```bash
# Create the data directory
mkdir -p ~/.modular

# Generate a new Ed25519 validator key
modular-node keygen --output ~/.modular/validator_key.json

# View the generated key
cat ~/.modular/validator_key.json
```

Example output:
```json
{
  "public_key": "401c76b85552dfd28fd120e236b252b4eb7f45ef6b72d3103ea0082fbb476642",
  "private_key": "94823ff3ade5eb1e1c6c9ea72fad231e1db6d7d3eaa62d0efbf75d9af44987b3",
  "address": "0x4c398804adb1e483cc1b486bfc11093491322de6"
}
```

### Key Backup

```bash
# Backup the validator key to a secure location
cp ~/.modular/validator_key.json /backup/validator_key.json

# Encrypt the backup with GPG
gpg --symmetric --cipher-algo AES256 /backup/validator_key.json

# Store the encrypted file offline (USB drive, hardware wallet)
```

> **Recommendation**: Store validator keys on a hardware security module (HSM) or use multi-sig for production validators.

### Key Recovery

If your validator key is lost but you still have the mnemonic phrase:
```bash
modular-node keygen --from-mnemonic "your twelve word mnemonic phrase here" \
  --output ~/.modular/validator_key.json
```

---

## Node Installation

### Option A: One-Click Setup Script

```bash
# Download and run the setup script
curl -fsSL https://raw.githubusercontent.com/your-org/modular-blockchain/main/scripts/setup-validator.sh | bash
```

### Option B: Manual Installation

```bash
# 1. Download the latest release
curl -LO https://github.com/your-org/modular-blockchain/releases/latest/download/modular-node-linux-amd64.tar.gz

# 2. Verify checksum
curl -LO https://github.com/your-org/modular-blockchain/releases/latest/download/modular-node-linux-amd64.tar.gz.sha256
sha256sum --check modular-node-linux-amd64.tar.gz.sha256

# 3. Extract and install
tar xzf modular-node-linux-amd64.tar.gz
sudo mv modular-node /usr/local/bin/
rm modular-node-linux-amd64.tar.gz

# 4. Verify installation
modular-node --version
```

### Option C: Build from Source

```bash
# Prerequisites: Rust toolchain (rustc 1.80+)
git clone https://github.com/your-org/modular-blockchain.git
cd modular-blockchain

# Build the node binary (this may take 15-30 minutes)
cargo build --release -p node

# Copy the binary
sudo cp target/release/modular-node /usr/local/bin/
```

---

## Validator Registration

### Step 1: Create a Validator Account

```bash
# Generate a wallet
curl -X POST https://rpc.testnet.modular-blockchain.io/wallet \
  -H "Content-Type: application/json" \
  -d '{"action": "create"}'
```

Or using the SDK:
```ts
import { Wallet } from "@modular-blockchain/sdk";
const wallet = Wallet.generate();
console.log("Address:", wallet.address);
console.log("Private key:", wallet.getPrivateKey());
```

### Step 2: Fund Your Account

Get testnet tokens from the faucet:
```bash
curl -X POST https://faucet.testnet.modular-blockchain.io \
  -H "Content-Type: application/json" \
  -d '{"address": "0x<YOUR_ACCOUNT_ADDRESS>"}'
```

### Step 3: Submit Registration Transaction

```bash
# Submit a validator registration transaction
modular-node tx register-validator \
  --from 0x<YOUR_ACCOUNT_ADDRESS> \
  --private-key <YOUR_PRIVATE_KEY> \
  --public-key <VALIDATOR_PUBLIC_KEY> \
  --stake 100000 \
  --commission-rate 0.10 \
  --rpc https://rpc.testnet.modular-blockchain.io
```

This creates a governance proposal that the network will process. Your validator will be added to the active set after the next epoch boundary.

### Step 4: Verify Registration

```bash
# Check if your validator is in the active set
curl https://rpc.testnet.modular-blockchain.io/validators

# Check your account balance
curl https://rpc.testnet.modular-blockchain.io/balance/0x<YOUR_ADDRESS>

# Query your validator status
curl https://rpc.testnet.modular-blockchain.io/validator/0x<YOUR_ADDRESS>
```

### Registration Parameters

| Parameter | Value | Description |
|-----------|-------|-------------|
| Minimum stake | 100,000 NBL | Minimum tokens to self-bond |
| Maximum validators | 100 | Active validator set size |
| Commission rate | 0.0 - 1.0 | Fee on delegator rewards |
| Unbonding period | 21,600 blocks (~18 hrs) | Time to unstake |

---

## Starting Your Node

### Step 1: Create Configuration

```toml
# /etc/modular/config.toml
[network]
chain_id = "modular-testnet-1"
p2p_port = 26656
rpc_port = 26657
bootstrap_nodes = [
    "/dns4/bootstrap.testnet.modular-blockchain.io/tcp/26656"
]

[consensus]
block_time_ms = 3000
max_validators = 100

[validator]
enabled = true
commission_rate = "0.10"

[storage]
data_dir = "/data/blockchain"

[api]
enabled = true
address = "0.0.0.0:8545"
cors_origins = ["*"]

[metrics]
enabled = true
address = "0.0.0.0:9090"
```

### Step 2: Download Genesis File

```bash
curl -o /etc/modular/genesis.json \
  https://raw.githubusercontent.com/your-org/modular-blockchain/main/deployment/cloud/configs/genesis.json
```

### Step 3: Start the Node (Manual)

```bash
modular-node start \
  --config /etc/modular/config.toml \
  --genesis /etc/modular/genesis.json \
  --validator-key ~/.modular/validator_key.json
```

### Step 4: Systemd Service (Recommended)

```ini
# /etc/systemd/system/modular-node.service
[Unit]
Description=Modular Blockchain Validator
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=modular
Group=modular
ExecStart=/usr/local/bin/modular-node start \
  --config /etc/modular/config.toml \
  --genesis /etc/modular/genesis.json \
  --validator-key /home/modular/.modular/validator_key.json
Restart=always
RestartSec=10
LimitNOFILE=65535
LimitNPROC=4096
MemoryMax=16G
CPUQuota=80%

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable modular-node
sudo systemctl start modular-node
sudo systemctl status modular-node
```

### Step 5: Verify Node is Running

```bash
# Check node status
curl http://localhost:26657/health

# Check if your node is peering
curl http://localhost:26657/peers

# Check block sync status
curl http://localhost:26657/status | jq
```

---

## Monitoring Your Validator

### Essential Metrics

| Metric | Description | Target |
|--------|-------------|--------|
| Block height | Current chain height | `>= network height` |
| Peer count | Connected peers | `>= 5` |
| Missed blocks | Blocks missed in current window | `0` |
| Active status | Whether validator is in active set | `true` |
| Stake | Current bonded stake | `>= 100,000 NBL` |

### Prometheus Metrics

Your node exposes metrics at `http://localhost:9090/metrics`:

```
# HELP chain_validator_count Number of active validators
# HELP chain_validator_stake_total Total stake across all validators
# HELP chain_missed_blocks_total Total missed blocks by this validator
# HELP chain_blocks_produced_total Total blocks produced by this validator
# HELP chain_peer_count Number of connected peers
```

### Grafana Dashboard

Access the validator dashboard at `https://grafana.testnet.modular-blockchain.io`.

To import the dashboard:
1. Open Grafana → "+" → "Import"
2. Paste the dashboard JSON from `monitoring/grafana/dashboards/validator-performance.json`
3. Select the Prometheus data source
4. Click "Import"

### Alerts

Configured alerts (via Prometheus Alertmanager):

| Alert | Condition | Severity |
|-------|-----------|----------|
| ValidatorDown | Node unreachable for 2 minutes | Critical |
| HighMissedBlocks | Missed block rate > 0.1/s for 5 minutes | Warning |
| LowPeerCount | < 3 peers connected for 5 minutes | Warning |
| ConsensusStalled | No blocks produced for 5 minutes | Critical |
| HighDiskUsage | State size > 80 GB | Warning |

---

## Validator Operations

### Checking Your Validator Status

```bash
# Using the CLI
modular-node query validator --address 0x<YOUR_ADDRESS>

# Using the API
curl https://rpc.testnet.modular-blockchain.io/validator/0x<YOUR_ADDRESS>
```

### Updating Commission Rate

```bash
modular-node tx update-commission \
  --from 0x<YOUR_ADDRESS> \
  --private-key <KEY> \
  --commission-rate 0.15 \
  --rpc https://rpc.testnet.modular-blockchain.io
```

### Unbonding / Stopping

```bash
# Initiate unbonding (18 hour waiting period)
modular-node tx unbond \
  --from 0x<YOUR_ADDRESS> \
  --private-key <KEY> \
  --amount 50000 \
  --rpc https://rpc.testnet.modular-blockchain.io

# Check unbonding status
curl https://rpc.testnet.modular-blockchain.io/unbonding/0x<YOUR_ADDRESS>
```

### Governance Participation

```bash
# List active proposals
curl https://rpc.testnet.modular-blockchain.io/governance/proposals

# Vote on a proposal
modular-node tx vote \
  --from 0x<YOUR_ADDRESS> \
  --private-key <KEY> \
  --proposal-id 1 \
  --vote yes \
  --rpc https://rpc.testnet.modular-blockchain.io
```

---

## Delegation & Rewards

### How Delegation Works

Token holders can delegate NBL to validators. The validator's voting power increases with total delegated stake.

### Reward Distribution

- **Block rewards**: Distribution per block based on inflation schedule (initial: 10B NBL/year, halves every 2.1M blocks)
- **Transaction fees**: 50% of priority fees burned, 50% distributed to validators and delegators
- **Treasury cut**: 10% of each block reward goes to the on-chain treasury

### Reward Calculation

1. Total block reward = inflation reward + transaction fees
2. Treasury takes 10%
3. Validator takes commission_rate on delegator share
4. Remaining split proportionally by stake

Example:
- Block reward: 1,000 NBL
- Treasury: 100 NBL (10%)
- Validator stake: 10%, Delegator stake: 90%
- Commission: 10%
- Validator reward: 100 + (810 × 0.10) = 181 NBL
- Delegators share: 810 - 81 = 729 NBL (split by their stake proportion)

### Claiming Rewards

```bash
# Claim all pending rewards
modular-node tx claim-rewards \
  --from 0x<YOUR_ADDRESS> \
  --private-key <KEY> \
  --rpc https://rpc.testnet.modular-blockchain.io

# Check pending rewards
curl https://rpc.testnet.modular-blockchain.io/rewards/0x<YOUR_ADDRESS>
```

---

## Security Best Practices

### Do's

- ✅ Use a dedicated machine for your validator (no other services)
- ✅ Enable firewall rules (UFW or iptables)
- ✅ Use SSH key authentication only (disable passwords)
- ✅ Keep your OS and software up to date
- ✅ Set up monitoring and alerting
- ✅ Back up your validator key offline (encrypted)
- ✅ Use a hardware key for your validator account
- ✅ Run your node as a non-root user
- ✅ Set resource limits (systemd `MemoryMax`, `CPUQuota`)

### Don'ts

- ❌ Never share your validator private key
- ❌ Don't run your validator and RPC on the same machine
- ❌ Don't expose your validator's RPC port to the internet
- ❌ Don't ignore security updates
- ❌ Don't use the same key for validator consensus and account
- ❌ Don't run two validators with the same key (double-sign slashing)

### Firewall Configuration

```bash
# UFW example
ufw default deny incoming
ufw default allow outgoing
ufw allow 26656/tcp    # P2P (required)
ufw allow 22/tcp       # SSH
ufw allow 9100/tcp     # node-exporter (optional)
ufw allow 9090/tcp     # prometheus (optional, restrict to monitoring IPs)
ufw enable
```

### SSH Hardening

```bash
# /etc/ssh/sshd_config
PermitRootLogin no
PasswordAuthentication no
PubkeyAuthentication yes
AllowUsers modular
MaxAuthTries 3
ClientAliveInterval 300
ClientAliveCountMax 2
```

### System Hardening

```bash
# Create dedicated user
sudo useradd -r -s /sbin/nologin -m -d /home/modular modular

# Set resource limits
echo "modular soft nofile 65535" | sudo tee -a /etc/security/limits.conf
echo "modular hard nofile 65535" | sudo tee -a /etc/security/limits.conf

# Enable automatic security updates
sudo apt-get install unattended-upgrades
sudo dpkg-reconfigure -plow unattended-upgrades
```

---

## Troubleshooting

### Node Won't Start

| Symptom | Cause | Solution |
|---------|-------|----------|
| `connection refused` | Node not running | Check `systemctl status modular-node` |
| `address already in use` | Port conflict | Change port in config.toml |
| `genesis file not found` | Missing genesis | Download genesis file |
| `validator key not found` | Missing key | Generate key with `modular-node keygen` |

### Node Can't Peer

| Symptom | Cause | Solution |
|---------|-------|----------|
| `no peers found` | P2P port blocked | Check firewall (TCP 26656) |
| `dial error` | Bootstrap unreachable | Verify bootstrap address |
| `incompatible chain` | Wrong chain ID | Verify `chain_id` in config |

### Consensus Issues

| Symptom | Cause | Solution |
|---------|-------|----------|
| `block not producing` | Not in active set | Check validator registration |
| `missed blocks` | Resource constraints | Upgrade hardware, check CPU/memory |
| `clock skew` | Time not synced | Install `chrony` or `ntp` |

### Quick Diagnostics

```bash
# Full health check
modular-node health --rpc http://localhost:26657

# Check logs
journalctl -u modular-node -f --since "1 hour ago"

# Check disk usage
df -h /data/blockchain

# Check memory
free -h

# Check CPU
top -b -n1 | head -20

# Check network connections
ss -tlnp | grep -E "(26656|26657|8545|9090)"

# Test P2P connectivity
nc -zv <YOUR_IP> 26656
```

---

## Useful Links

| Resource | URL |
|----------|-----|
| Explorer | https://explorer.testnet.modular-blockchain.io |
| Faucet | https://faucet.testnet.modular-blockchain.io |
| Grafana | https://grafana.testnet.modular-blockchain.io |
| RPC | https://rpc.testnet.modular-blockchain.io |
| Status | https://status.testnet.modular-blockchain.io |
| GitHub | https://github.com/your-org/modular-blockchain |
| Discord | https://discord.gg/modular-blockchain |
