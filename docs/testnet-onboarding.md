# Public Testnet Onboarding Guide

Welcome to the Modular Blockchain public testnet! This guide will help you connect, get tokens, and start building.

---

## Network Overview

| Parameter | Value |
|-----------|-------|
| Chain ID | `modular-testnet-1` |
| Block Time | ~3 seconds |
| Consensus | BFT (libp2p + gossipsub) |
| Cryptography | Ed25519 |
| Gas Model | EIP-1559 |
| RPC Endpoint | `https://rpc.testnet.modular-blockchain.io` |
| Faucet | `https://faucet.testnet.modular-blockchain.io` |
| Explorer | `https://explorer.testnet.modular-blockchain.io` |
| Bootstrap Peer | `/dns4/bootstrap.testnet.modular-blockchain.io/tcp/26656` |
| Chain Monitor | `https://grafana.testnet.modular-blockchain.io` |
| Network Status | `https://status.testnet.modular-blockchain.io` |

---

## Quick Start

### 1. Install the SDK
```bash
npm install @modular-blockchain/sdk
```

### 2. Connect to testnet
```ts
import { ModularClient, HttpProvider } from "@modular-blockchain/sdk";

const provider = new HttpProvider("https://rpc.testnet.modular-blockchain.io");
const client = new ModularClient(provider);

const status = await client.getNodeStatus();
console.log("Connected! Block height:", status.height);
```

### 3. Create a wallet
```ts
import { Wallet } from "@modular-blockchain/sdk";

const wallet = Wallet.generate();
console.log("Address:", wallet.address);  // 0x...
console.log("Private key:", wallet.getPrivateKey());  // store securely
```

### 4. Get test tokens
Visit `https://faucet.testnet.modular-blockchain.io` and enter your address.

Or use the CLI:
```bash
curl -X POST https://faucet.testnet.modular-blockchain.io \
  -H "Content-Type: application/json" \
  -d '{"address": "0x..."}'
```

### 5. Send a transaction
```ts
const tx = await wallet.signTransaction({
  to: "0x...",
  value: "1000000000000000000",  // 1 NBL
  chainId: 1,
});
const receipt = await client.sendTransaction(tx);
console.log("Tx hash:", receipt.hash);
```

---

## For Validators

### Hardware Requirements
| Role | CPU | RAM | Disk | Network |
|------|-----|-----|------|---------|
| Validator | 4+ cores | 8+ GB | 100+ GB SSD | 100 Mbps |
| RPC Node | 8+ cores | 16+ GB | 200+ GB SSD | 1 Gbps |

### Running a Validator Node

1. **Install the node binary:**
   ```bash
   # Download latest release
   curl -LO https://github.com/your-org/modular-blockchain/releases/latest/download/modular-node-linux-amd64.tar.gz
   tar xzf modular-node-linux-amd64.tar.gz
   sudo mv modular-node /usr/local/bin/
   ```

2. **Generate validator key:**
   ```bash
   modular-node keygen --output ~/.modular/validator_key.json
   ```

3. **Submit validator registration:**
   Register your validator's public key by submitting a transaction to the governance contract. The minimum stake is 100,000 NBL.

4. **Start the node:**
   ```bash
   modular-node start \
     --config /etc/modular/config.toml \
     --genesis /etc/modular/genesis.json
   ```

### Example systemd service
```ini
[Unit]
Description=Modular Blockchain Validator
After=network.target

[Service]
Type=simple
User=modular
ExecStart=/usr/local/bin/modular-node start \
  --config /etc/modular/config.toml \
  --genesis /etc/modular/genesis.json
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

---

## For dApp Developers

### SDK Packages
| Package | Description |
|---------|-------------|
| `@modular-blockchain/sdk` | TypeScript SDK (wallets, transactions, providers) |
| `@modular-blockchain/sdk-cli` | CLI for scaffolding and deployment |

### Starter Kits
| Kit | Description |
|-----|-------------|
| [DeFi](../sdk/javascript/examples/starter-kits/defi.ts) | AMM liquidity pool example |
| [NFT](../sdk/javascript/examples/starter-kits/nft.ts) | NFT mint and marketplace example |
| [DAO](../sdk/javascript/examples/starter-kits/dao.ts) | Governance proposal example |

### Smart Contract Templates
| Contract | Standards | Location |
|----------|-----------|----------|
| `ERC20.sol` | ERC-20 | `sdk/javascript/contracts/ERC20.sol` |
| `ERC721.sol` | ERC-721 | `sdk/javascript/contracts/ERC721.sol` |
| `DAO.sol` | Governance | `sdk/javascript/contracts/DAO.sol` |

### Deploy a contract
```bash
npx @modular-blockchain/sdk-cli deploy \
  --contract ERC20 \
  --args "MyToken,MTK,1000000" \
  --rpc https://rpc.testnet.modular-blockchain.io \
  --key 0x...
```

---

## RPC API Reference

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/status` | Node status |
| `GET` | `/mempool` | Pending transactions |
| `POST` | `/submit_tx` | Submit transaction |
| `GET` | `/block/{height}` | Block by height |
| `GET` | `/block/hash/{hash}` | Block by hash |
| `GET` | `/balance/{address}` | Account balance & nonce |
| `GET` | `/tx/{hash}` | Transaction receipt |
| `GET` | `/gas_price` | EIP-1559 gas prices |
| `POST` | `/estimate_gas` | Estimate transaction gas |
| `GET` | `/fee_history/{count}` | Historical fee data |
| `GET` | `/health` | Node health check |
| `GET` | `/peers` | Connected peers |
| `POST` | `/connect_peer` | Connect to peer |

---

## Network Parameters

| Parameter | Value |
|-----------|-------|
| Block gas limit | 30,000,000 |
| Base fee | Starts at 1 Gwei, adjusts per EIP-1559 |
| Transaction fee | `base_fee + priority_fee` |
| Minimum validator stake | 100,000 NBL |
| Slashing penalty | Double-sign: 5% of stake |
| Unbonding period | 21,600 blocks (~18 hours) |
| Governance voting period | 1,000 blocks (~50 minutes) |

---

## Faucet

The testnet faucet drips 1,000 NBL per request (once every 24 hours).

- **Web**: https://faucet.testnet.modular-blockchain.io
- **API**: `POST https://faucet.testnet.modular-blockchain.io` with body `{"address": "0x..."}`
- **CLI**: `curl -X POST https://faucet.testnet.modular-blockchain.io -H "Content-Type: application/json" -d '{"address": "0x..."}'`

---

## Monitoring

| Service | URL | Credentials |
|---------|-----|-------------|
| Grafana | https://grafana.testnet.modular-blockchain.io | `admin` / request access |
| Prometheus | https://prometheus.testnet.modular-blockchain.io | Internal |
| Block Explorer | https://explorer.testnet.modular-blockchain.io | Public |
| Status Page | https://status.testnet.modular-blockchain.io | Public |

---

## Support & Community

- **GitHub Issues**: https://github.com/your-org/modular-blockchain/issues
- **Discord**: https://discord.gg/modular-blockchain
- **Documentation**: https://docs.modular-blockchain.io
- **Telegram**: https://t.me/modular-blockchain

---

## Useful Links

| Resource | URL |
|----------|-----|
| GitHub | https://github.com/your-org/modular-blockchain |
| SDK Docs | https://github.com/your-org/modular-blockchain/tree/main/sdk/javascript |
| NPM Package | https://www.npmjs.com/package/@modular-blockchain/sdk |
| Contract Templates | https://github.com/your-org/modular-blockchain/tree/main/sdk/javascript/contracts |
| Explorer | https://explorer.testnet.modular-blockchain.io |
| Faucet | https://faucet.testnet.modular-blockchain.io |
