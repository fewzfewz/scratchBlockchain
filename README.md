# Nebula: High-Performance Modular Blockchain

[![Rust](https://img.shields.io/badge/built_with-Rust-dca282.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Docker](https://img.shields.io/badge/docker-ready-blue)](docker-compose.yml)

**Nebula** is a next-generation blockchain built for high-performance enterprise applications. It combines the security of a validator-based network with the flexibility of a modular architecture, supporting both EVM and WASM smart contracts.

## Quick Start

### Prerequisites
- **Rust** (latest stable)
- **Node.js** 18+ and npm

### 1. Build & Start the Node

```bash
cargo build
./target/debug/node start --genesis genesis.json
```

The node will start on `http://localhost:9933` with 3 genesis accounts and 3 genesis validators pre-configured.

### 2. Start the Frontend

```bash
cd frontend
npm install
npm run dev
```

Open **http://localhost:5173** to access the unified UI with all 8 pages:

| Route | Page | Description |
|-------|------|-------------|
| `/` | **Home** | Network overview & status |
| `/explorer` | **Explorer** | Blocks, validators, staking & delegations |
| `/wallet` | **Wallet** | Ed25519 keypair management & signed transactions |
| `/faucet` | **Faucet** | Test token dispenser (local simulation if backend offline) |
| `/governance` | **Governance** | Proposal voting, treasury & analytics |
| `/docs` | **Docs** | Full architecture & RPC API documentation |
| `/api-docs` | **API Reference** | Interactive Swagger UI — try endpoints from the browser |
| `/sdk` | **SDK Portal** | JavaScript SDK reference & contract templates |
| `/developer-portal` | **Dev Portal** | Starter kits & CLI reference |

### 3. Faucet Backend (Optional)

For on-chain token distribution, start the faucet service:

```bash
./target/debug/node faucet
```

This starts a warp HTTP server on port 3006. Without it, the faucet frontend falls back to local simulation (tokens won't appear on-chain).

## Port Mapping

| Port | Service | Notes |
|------|---------|-------|
| 9933 | Node RPC | JSON-RPC API |
| 5173 | Frontend | Unified Vite/React SPA |
| 3006 | Faucet | Optional token dispenser backend |

## RPC API

The node exposes RESTful JSON endpoints on `http://localhost:9933`:

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/status` | Node status (height, peers, mempool) |
| GET | `/health` | Health check |
| GET | `/gas_price` | Current gas price (EIP-1559) |
| GET | `/mempool` | Pending transactions |
| GET | `/peers` | Connected peers |
| GET | `/validators` | Active validator set (from genesis) |
| GET | `/block/latest` | Latest block |
| GET | `/block/{height}` | Block by height |
| GET | `/block/hash/{hash}` | Block by 32-byte hash |
| GET | `/tx/{hash}` | Transaction receipt by hash |
| GET | `/balance/{address}` | Account balance & nonce |
| GET | `/nonce/{address}` | Account nonce |
| GET | `/delegations/{address}` | Delegations by address |
| POST | `/submit_tx` | Submit signed transaction |
| POST | `/connect_peer` | Connect to a peer |
| GET | `/estimate_gas` | Estimate gas for a transaction |
| GET | `/fee_history` | Historical fee data |

### Rate Limiting

All RPC endpoints are rate-limited per IP (default: **200 requests/second**, configurable via `api.rate_limit` in the node config). If you see `{"error":"Rate limit exceeded"}`, your client is sending requests too quickly — either increase the limit in the config or add client-side throttling.

## API Documentation

- **Interactive Swagger UI**: Open `http://localhost:5173/api-docs` in your browser — try all 17 endpoints interactively with the "Try it out" button
- **OpenAPI 3.0 spec**: [`docs/openapi.yaml`](docs/openapi.yaml) — machine-readable spec with schemas and example responses
- **Frontend docs page**: `http://localhost:5173/docs` for human-readable reference with real `curl` examples
- **Frontend SDK page**: `http://localhost:5173/sdk` for the JavaScript SDK reference

## Transaction Flow

1. Generate an Ed25519 keypair (in the Wallet page or via CLI)
2. Fund the address via the Faucet page
3. Construct a transaction with sender, recipient, value, nonce, and gas params
4. Sign the transaction payload with your private key
5. Submit via `POST /submit_tx` and get back a hash for tracking

## Configuration

The node uses a TOML config file (default: `config.toml`). Key settings:

| Setting | Default | Description |
|---------|---------|-------------|
| `api.rate_limit` | 200 | Requests per second per IP |
| `network.rpc_port` | 9933 | RPC server port |
| `network.p2p_port` | 9000 | P2P networking port |
| `consensus.max_validators` | 32 | Max active validators |
| `mempool.max_tx_per_block` | 500 | Max transactions per block |

## Troubleshooting

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| `{"error":"Rate limit exceeded"}` | Too many requests from the same IP | Increase `api.rate_limit` in config, or throttle your client |
| `TweetNaCl library not loaded` | Missing npm dependency | Run `npm install` in the `frontend/` directory |
| `POST http://localhost:3006/faucet` connection refused | Faucet backend not running | Start with `./target/debug/node faucet`, or use the frontend's local simulation mode |
| `GET /validators` returns empty array | Validators not in state trie | Ensure `genesis.json` has validators and restart the node |
| `Endpoint not found` 404 | RPC endpoint doesn't exist | Check the RPC API table above or open `http://localhost:5173/api-docs` for the full interactive reference; may need to rebuild after adding new endpoints |

## Project Structure

```
.
├── common/             # Shared types (Block, Tx, traits)
├── consensus/          # BFT consensus + GRANDPA finality
├── execution/          # Multi-VM (EVM + WASM + parallel)
├── network/            # P2P networking (libp2p, Gossipsub)
├── node/               # Node binary, RPC server, faucet
├── storage/            # RocksDB / Sled + Patricia trie
├── frontend/           # Unified React SPA (all 8 UIs)
├── sdk/javascript/     # JavaScript SDK (RxJS-based)
├── faucet/             # Standalone faucet crate
├── docs/               # Markdown docs + OpenAPI 3.0 spec
├── scripts/            # Deployment & operation scripts
└── monitoring/         # Prometheus & Grafana config
```

## Features

| Feature | Status |
|---------|--------|
| Validator-based consensus w/ GRANDPA finality | Ready |
| P2P networking via libp2p + Gossipsub | Ready |
| Multi-VM (EVM + WASM) parallel execution | Ready |
| RocksDB persistent storage + Patricia trie | Ready |
| Ed25519 cryptographic signatures (via tweetnacl) | Ready |
| Governance (proposals, voting, treasury) | Ready |
| Light/dark theme with glassmorphism UI | Ready |
| OpenAPI 3.0 spec for all 17 RPC endpoints | Ready |
| Genesis validators in state trie | Ready |
| Rate-limited RPC (configurable) | Ready |
| Prometheus + Grafana monitoring | Ready |

## License

MIT
