# Nebula: High-Performance Modular Blockchain

[![Rust](https://img.shields.io/badge/built_with-Rust-dca282.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Docker](https://img.shields.io/badge/docker-ready-blue)](docker-compose.yml)

**Nebula** is a next-generation blockchain built for high-performance enterprise applications. It combines the security of a validator-based network with the flexibility of a modular architecture, supporting both EVM and WASM smart contracts.

## 🚀 Why Nebula?

- **⚡ Unmatched Speed**: Parallel transaction execution engine (via `rayon`) processes thousands of transactions per second (TPS), unlike serial execution in legacy chains.
- **🛡️ Instant Finality**: Powered by a GRANDPA-style finality gadget, ensuring that once a block is finalized, it is irreversible. No waiting for confirmations.
- **🧩 Modular Design**: Plug-and-play components for Consensus, Execution, and Data Availability. Customize the chain to your needs.
- **🔐 Enterprise Security**: Built-in TLS for RPC, API rate limiting, and a robust peer reputation system to prevent abuse.
- **🌐 Multi-VM Support**: Deploy existing Solidity contracts (EVM) or write high-performance modules in Rust (WASM).

---

## 🛠️ Features

| Feature | Description | Status |
|---------|-------------|--------|
| **Consensus** | Validator-based with Ed25519 signatures & GRANDPA finality | ✅ Ready |
| **Networking** | P2P discovery via libp2p + Gossipsub | ✅ Ready |
| **Execution** | Multi-VM (EVM + WASM) with Parallel Processing | ✅ Ready |
| **Storage** | High-performance persistent storage using Sled | ✅ Ready |
| **Monitoring** | Real-time metrics via Prometheus & Grafana | ✅ Ready |
| **Security** | TLS, Rate Limiting, DDoS Protection | ✅ Ready |
| **L2 Support** | Built-in Optimistic Rollup & ZK Prover infrastructure | 🚧 Beta |

---

## 🏁 Quick Start

### Prerequisites
- **Docker** & **Docker Compose**
- **Rust** (latest stable) - *Optional, for development*

### 1. Launch the Network
Deploy a fully configured 3-node validator network with monitoring in one command:

```bash
./scripts/deploy.sh
```

This will start:
- **3 Validator Nodes** (Ports 9933, 9934, 9935)
- **Prometheus** (Metrics Scraper)
- **Grafana** (Dashboard)

### 2. Access the Dashboard
Open your browser to view the real-time network status:
- **Grafana Dashboard**: [http://localhost:3000](http://localhost:3000) (User: `admin`, Pass: `blockchain2024`)

### 3. Use the Tools
We provide a built-in Block Explorer and Wallet for interacting with the chain.

- **Block Explorer**: Open `explorer/index.html` in your browser.
  - View live blocks, transactions, and network stats.
- **Web Wallet**: Open `wallet/index.html` in your browser.
  - Generate a secure Ed25519 keypair.
  - Send transactions to the network.

---

## 💻 Developer Guide

### API Reference
Interact with the node via JSON-RPC on `http://localhost:9933`.

**Get Status**
```bash
curl http://localhost:9933/status
```

**Submit Transaction**
```bash
curl -X POST http://localhost:9933/submit_tx -H "Content-Type: application/json" -d '{
  "sender": "...",
  "to": "...",
  "value": 100,
  "nonce": 1,
  "signature": "..."
}'
```

### Running Tests
Ensure the system is stable by running the integration test suite:

```bash
# Run multi-node consensus tests
cargo test --test multi_node_consensus

# Run load tests
cargo test --test load_test
```

---

## 📂 Project Structure

```
.
├── common/          # Shared types (Block, Tx) and traits
├── consensus/       # Consensus logic (GRANDPA, Slashing)
├── execution/       # VM implementation (EVM, WASM, Parallel)
├── network/         # P2P networking (libp2p)
├── node/            # Main node binary & RPC server
├── storage/         # Database layer (Sled)
├── explorer/        # Web-based Block Explorer
├── wallet/          # Web-based Wallet
└── scripts/         # Deployment & Operation scripts
```

## 🤝 Contributing
Contributions are welcome! Please check out the `docs/` directory for detailed architecture documentation.

## 📄 License
MIT License
