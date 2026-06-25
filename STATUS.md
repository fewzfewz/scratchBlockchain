# Blockchain Status Summary

## Current Status: DEVELOPMENT

**Last Updated**: June 25, 2026  
**Node RPC**: `http://localhost:9933`  
**Frontend**: `http://localhost:5173`  
**Faucet**: Built into node RPC at `POST /faucet/request` (no separate service needed)

---

## What's Working Now

### Frontend (Unified React SPA, port 5173)
- **Wallet** — Ed25519 key generation via TweetNaCl, address derivation (20-byte), balance/nonce queries, send tx with gas params, test address presets
- **Explorer** — Dashboard (chain status), Validators (3 genesis validators), Staking tab — all with light/dark mode
- **Faucet** — Direct `POST /faucet/request` to node RPC; credits the state trie; no separate faucet backend needed; offline banner when node unreachable
- **Governance** — Proposals list, vote modal, create proposal form — dark/light mode compatible
- **Docs** — Human-readable API reference with real `curl` examples
- **API Docs** — Interactive Swagger UI (lazy-loaded), try-all-17-endpoints from the browser
- **SDK Portal** — JavaScript SDK reference
- **Dev Portal** — Developer dashboard

### Node RPC (port 9933, 17 endpoints)
- `GET /health`, `/status`, `/mempool`, `/metrics`
- `GET /block/{height}`, `/block/hash/{hash}`, `/block/latest`
- `GET /balance/{address}`, `/tx/{hash}`
- `GET /gas_price`, `/fee_history/{count}`
- `POST /submit_tx`, `/estimate_gas`, `/connect_peer`
- `GET /peers`, `/validators`
- `GET /delegations/{address}`
- `POST /faucet/request` — credits address directly in state trie

### Key Bug Fixes Applied
- Rate limiter uses config value (was hardcoded 100 req/s) — default 200 req/s
- Rejection handler no longer defaults to 429 for non-rate-limit errors (e.g. JSON parse → 400)
- `submit_tx` now has rate limiting
- Genesis validators written to state trie at startup (`/validators` returns real data)
- Wallet address fixed: derives 20-byte address from 32-byte public key
- Faucet credits the state trie directly instead of just returning success

---

## What Compiles
- **Frontend**: Vite build succeeds (354 KB main + 1.3 MB lazy-loaded API docs)
- **Rust node**: Compiles with `cargo build` (all crates, including heavy deps like rocksdb/libp2p/revm/wasmtime)

---

## Known Issues
- Coin code still building; balance for faucet-credited addresses shows immediately via `/balance` endpoint
- No WebSocket support (HTTP-only RPC)
- No EVM contract deployment UI (needs contract interaction page)
- No transaction history view
- Rust build requires ~5 GB disk space

---

## Quick Start
```bash
# 1. Build Rust node
cargo build

# 2. Start the node with genesis validators
./target/debug/node start --genesis genesis.json

# 3. Install frontend deps & start dev server
cd frontend && npm install && npm run dev

# 4. Open http://localhost:5173
```

---

## Architecture
```
┌──────────────────────────────────────────┐
│  Frontend (port 5173)                    │
│  React SPA ─ 9 routes ─ light/dark       │
│  Wallet │ Explorer │ Faucet │ Governance  │
│  Docs │ API Docs │ SDK │ Dev Portal      │
├──────────────────────────────────────────┤
│  Node RPC (port 9933)                    │
│  17 HTTP endpoints ─ warp server          │
│  Rate-limited ─ CORS-enabled              │
├──────────────────────────────────────────┤
│  Rust Backend                            │
│  consensus │ execution │ network │ storage│
│  mempool │ governance │ mev │ runtime     │
└──────────────────────────────────────────┘
```

**Version**: 1.0.0-alpha  
**Status**: Development — local testnet ready
