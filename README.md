# Nebula: High-Performance Modular Blockchain

[![Rust](https://img.shields.io/badge/built_with-Rust-dca282.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Docker](https://img.shields.io/badge/docker-ready-blue)](deployment/local/docker-compose.yml)

**Nebula** is a modular blockchain with BFT consensus, EVM + WASM execution, built-in React UI (3D dashboard), cross-chain Ethereum bridge, DeFi/NFT pages, and a full Docker testnet.

---

## Quick Start — Everything in One Command

### Prerequisites

- **Docker** & Docker Compose v2
- **Rust** (latest stable) — to build the node binary
- **Node.js** 18+ — only if using `--local-frontend`

### Start the full stack

From the **repository root**:

```bash
./start-all.sh
```

This single script:

| Step | What it does |
|------|----------------|
| 1 | `cargo build --release -p node` |
| 2 | `docker compose -f deployment/local/docker-compose.yml up` — validators, RPC, monitoring |
| 3 | Starts **Hardhat** on port **9545** (Ethereum side for Bridge.sol) |
| 4 | Starts **frontend** on port **5173** (3D UI, wallet, DeFi, bridge, NFT) |
| 5 | Waits for Nebula RPC health on **8545** |
| 6 | Deploys **Bridge.sol** → saves address to `frontend/public/bridge-config.json` |
| 7 | Sets `ETH_RPC_URL=http://hardhat:9545` on Nebula nodes for relayer mint verification |

### Open the app

| URL | Description |
|-----|-------------|
| **http://localhost:5173** | Main UI (Home, Explorer, Wallet, DeFi, Bridge, NFT) |
| **http://localhost:8545** | Nebula JSON-RPC |
| **http://localhost:9545** | Hardhat Ethereum RPC |
| **http://localhost:5173/bridge** | Cross-chain bridge (MetaMask + relayer mint) |
| **http://localhost:3000** | Grafana (`admin` / `admin`) |

### Stop everything

```bash
./stop-all.sh
```

### Options

```bash
./start-all.sh --no-build          # skip cargo build (binary must exist)
./start-all.sh --no-bridge         # skip Bridge.sol deployment
./start-all.sh --local-frontend    # run Vite on host instead of Docker container
```

---

## Architecture (Docker stack)

```
┌─────────────────────────────────────────────────────────────┐
│  Frontend :5173  ──►  Nebula RPC :8545  (validator1)      │
│       │                      │                               │
│       └── /bridge ──►  Bridge.sol on Hardhat :9545          │
│                              │                               │
│                    ETH_RPC_URL (relayer verify + mint)       │
└─────────────────────────────────────────────────────────────┘
```

**Important:** Nebula uses port **8545**. Hardhat uses **9545** (no port conflict).

---

## Manual start (without Docker)

### 1. Nebula node

```bash
cargo build -p node
./target/debug/node start --genesis genesis.json
# RPC → http://localhost:8545
```

### 2. Frontend

```bash
cd frontend && npm install --legacy-peer-deps && npm run dev
# → http://localhost:5173
```

### 3. Ethereum bridge (separate terminals)

```bash
# Terminal A — Hardhat (port 9545)
cd interop && npm install
npx hardhat node --port 9545

# Terminal B — Deploy Bridge.sol
ETH_RPC_URL=http://127.0.0.1:9545 ./deployment/local/scripts/deploy-bridge.sh

# Terminal C — Nebula with ETH verification (if running node locally)
ETH_RPC_URL=http://127.0.0.1:9545 cargo run -p node -- start --genesis genesis.json
```

Bridge address is auto-loaded in the UI from `/bridge-config.json`.

---

## Frontend routes (16 pages)

| Route | Page |
|-------|------|
| `/` | Home — 3D hero, live stats |
| `/explorer` | Blocks, address lookup, validators, staking |
| `/wallet` | Ed25519 wallet, send, tx history |
| `/history` | Address transaction search |
| `/deploy` | ERC20 / ERC721 deploy |
| `/contracts` | Contract read/write |
| `/defi` | On-chain swaps & liquidity |
| `/bridge` | Ethereum ↔ Nebula bridge |
| `/nft` | ERC721 mint & gallery |
| `/faucet` | Test tokens |
| `/governance` | Proposals & voting |
| `/api-docs` | Swagger UI (34 routes) |
| `/sdk` | JavaScript SDK |
| `/developer-portal` | Starter kits |

---

## Port mapping (Docker testnet)

| Port | Service |
|------|---------|
| 5173 | Frontend (Vite) |
| 8545 | Nebula RPC — **validator1** (primary) |
| 8546–8547 | validator2, validator3 |
| 8548–8549 | rpc1, rpc2 |
| 9545 | Hardhat Ethereum node |
| 26656–26663 | P2P / internal RPC |
| 3000 | Grafana |
| 9095 | Prometheus |
| 80 | nginx gateway |

---

## Key RPC endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/status` | Chain height, mempool, `chain_id` |
| GET | `/balance/{address}` | Balance & nonce |
| GET | `/txs/{address}` | Transaction history |
| POST | `/submit_tx` | Submit signed transaction |
| POST | `/call_contract` | Read-only EVM call |
| POST | `/estimate_gas` | Gas estimation |
| POST | `/faucet/request` | Test tokens |
| GET | `/bridge/status` | Bridge vault, relayers, ETH RPC |
| POST | `/bridge/mint` | Mint on Nebula after ETH lock |
| POST | `/deploy_wasm` | Deploy WASM contract |
| GET | `/governance` | Proposals & treasury |

Full spec: [`docs/openapi.yaml`](docs/openapi.yaml) · Interactive UI: http://localhost:5173/api-docs

---

## Cross-chain bridge flow

1. **ETH → Nebula**
   - Connect MetaMask on `/bridge`
   - Lock ETH on `Bridge.sol` (Hardhat :9545)
   - Click **Relayer mint on Nebula** → `POST /bridge/mint` credits NBL

2. **Nebula → ETH**
   - Lock NBL to on-chain bridge vault via signed tx
   - Relayers process unlock on Ethereum (requires live relayer ops)

Deploy Bridge manually:

```bash
./deployment/local/scripts/deploy-bridge.sh
```

---

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| Port 8545 in use | Stop other services; Nebula validator1 needs 8545 |
| Port 9545 in use | Hardhat conflict — use `9545` only for Ethereum |
| Bridge address empty | Run `./start-all.sh` or `deploy-bridge.sh` |
| `ETH_RPC_URL not configured` | Set on node env or pass `eth_rpc_url` in `/bridge/mint` |
| Docker build slow | First run builds Rust inside Docker; use pre-built `target/release/node` mount |
| Frontend 5173 empty | `docker logs nebula-frontend` or use `--local-frontend` |
| MetaMask wrong network | Add Hardhat localhost chainId `1337`, RPC `http://127.0.0.1:9545` |

---

## Project structure

```
.
├── start-all.sh              # ← One command to start everything
├── stop-all.sh               # Stop Docker + host processes
├── deployment/local/         # Docker Compose testnet
├── frontend/                 # React SPA (Three.js 3D UI)
├── interop/                  # Bridge.sol + Hardhat
├── node/                     # Rust node + RPC
├── sdk/javascript/           # TypeScript SDK
└── docs/openapi.yaml         # OpenAPI spec
```

---

## Legacy scripts

| Script | Purpose |
|--------|---------|
| `./start.sh` | Simple local node + frontend (no Docker) |
| `./scripts/start-testnet.sh` | Old root docker-compose |
| `./scripts/start-frontends.sh` | Frontend only |

**Recommended:** use `./start-all.sh` for the complete experience.

---

## License

MIT
