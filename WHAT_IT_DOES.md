# Scratch Blockchain — What It Does

## Quick Answer

A **modular Rust blockchain** with a **unified React frontend** for local development and testing. Run a single node, open a browser, and you have a working blockchain environment: wallet, explorer, faucet, governance, and interactive API docs.

---

## Working Features

### Frontend (http://localhost:5173 — 9 pages)

| Page | What you can do |
|------|----------------|
| **Wallet** | Generate Ed25519 keypair → derive 20-byte address → check balance/nonce → send tokens with gas params → copy test addresses |
| **Explorer** | View chain status (height, peers, mempool) → browse validators (3 from genesis) → staking tab |
| **Faucet** | Paste any 0x address → get 100 test tokens → tokens credited directly to node state → cooldown tracking |
| **Governance** | View mock proposals → vote for/against → create new proposals |
| **Docs** | 9 sections (intro through RPC reference) with real `curl` response examples |
| **API Docs** | Interactive Swagger UI — click any endpoint → "Try it out" → see real responses from your node |
| **SDK Portal** | JavaScript SDK reference (client setup, tx building, error handling) |
| **Dev Portal** | Developer tools overview |
| **Home** | Quick links to all pages |

### Node RPC (http://localhost:8545 — 17 endpoints)

| Method | Endpoint | What it does |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/status` | Height, finalized height, mempool size, peer count |
| GET | `/mempool` | Pending transactions |
| POST | `/submit_tx` | Submit a signed transaction |
| GET | `/block/{height}` | Block by number |
| GET | `/block/hash/{hash}` | Block by hash |
| GET | `/block/latest` | Most recent block |
| GET | `/balance/{address}` | Account balance + nonce |
| GET | `/tx/{hash}` | Transaction receipt |
| GET | `/gas_price` | EIP-1559 gas price suggestions |
| POST | `/estimate_gas` | Estimate gas for a transaction |
| GET | `/fee_history/{count}` | Historical base fees |
| GET | `/validators` | Active validators (from genesis) |
| GET | `/delegations/{address}` | Delegations for an address |
| POST | `/connect_peer` | Connect to a libp2p peer |
| GET | `/peers` | Connected peers list |
| GET | `/metrics` | Prometheus metrics |
| POST | `/faucet/request` | Credit an address with test tokens |

### Blockchain Backend

- **Consensus**: BFT with GRANDPA finality (3 validators, 2/3 majority)
- **Execution**: EVM (revm), Native, WASM-ready (wasmtime)
- **Storage**: RocksDB with column families + in-memory fallback
- **Networking**: libp2p (gossipsub, Kademlia, request-response)
- **Mempool**: Priority-ordered, fee-based eviction, per-sender limits
- **Gas**: EIP-1559 style (base fee, priority fee, fee burning)
- **Staking/Slashing**: Commission rates, delegation, slashing (double-sign, downtime, invalid state)
- **Governance**: Proposals, voting, treasury
- **Runtime upgrades**: Hot-swap, migration, rollback
- **MEV**: Threshold encryption, commit-reveal, builder auction
- **ZK**: Halo2 circuit for state transition proofs
- **Rollups**: Optimistic and ZK rollup support
- **Bridges**: Ethereum and Cosmos IBC bridge code (not deployed)
- **DA**: KZG commitments (simplified), erasure coding

---

## Quick Start

```bash
# 1. Build Rust node
cargo build

# 2. Start node with genesis state
./target/debug/node start --genesis genesis.json

# 3. Start frontend dev server (separate terminal)
cd frontend && npm install && npm run dev

# 4. Open http://localhost:5173
```

---

## Architecture

```
Frontend (port 5173) ←──→ Node RPC (port 8545)
  React SPA                 warp HTTP server
  9 pages                   17 endpoints
  dark/light mode           rate-limited
  Swagger UI docs           CORS-enabled
```

The frontend is a single-page React app. The node is a single Rust binary. No separate services needed (faucet is an RPC endpoint).

---

## Comparison to Ethereum / Bitcoin

| | Scratch | Ethereum | Bitcoin |
|---|---|---|---|
| **Model** | Account-based | Account-based (EVM) | UTXO |
| **Consensus** | BABE-like (round-robin) | PoS (Gasper) | PoW (SHA-256) |
| **Block time** | ~6s | ~12s | ~10min |
| **Smart contracts** | WASM + EVM | EVM (Solidity) | None |
| **Storage** | RocksDB / in-memory | LevelDB | LevelDB |
| **Frontend** | Unified SPA (built-in) | DApp ecosystem (MetaMask, Etherscan) | No standard dapp UI |
| **Single binary?** | Yes (one `node` binary) | No (EL + CL separate) | Yes (`bitcoind`) |
| **Learning curve** | Low | High (EL/CL split, Geth/Lighthouse, etc.) | Medium |

---

## Known Issues

- **Faucet offline**: Node must be running at port 8545; frontend shows amber warning if unreachable
- **No tx history**: Wallet shows balance/nonce but no list of past transactions
- **No EVM contract UI**: Can't deploy or query smart contracts from the frontend
- **No WebSocket**: Frontend polls every 10-15s instead of receiving push events
- **Rust build**: Requires ~5 GB disk space for compilation

---

*Version: 1.0.0-alpha | Last Updated: June 25, 2026 | Status: Development*
