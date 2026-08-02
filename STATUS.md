# Blockchain Status Summary

## Current Status: DEVELOPMENT

**Last Updated**: August 2, 2026  
**Node RPC**: `http://localhost:8545`  
**Frontend**: `http://localhost:5173`  
**Faucet**: Built into node RPC at `POST /faucet/request` (no separate service needed)

---

## What's Working Now

### Frontend (Unified React SPA, port 5173)
- **Wallet** — Ed25519 key generation via TweetNaCl, address derivation (20-byte), balance/nonce queries, send tx with gas params, test address presets; redesigned with gradient balance hero, aurora/grid backdrop, and icon-chipped sections
- **Explorer** — Dashboard (chain status + live recent-blocks feed), Validators (3 genesis validators), Staking tab — redesigned to match Home's design language (aurora blobs, gradient tabs, glass cards, loading skeletons); all with light/dark mode
- **Faucet** — Direct `POST /faucet/request` to node RPC; credits the state trie; no separate faucet backend needed; offline banner when node unreachable
- **Governance** — Proposals with status filters + live search, vote modal with quorum progress, create proposal form, treasury, analytics — redesigned to match Home's design language (aurora/grid backdrop, gradient hero, glass cards, network strip, skeletons); dark/light mode compatible
- **Docs** — Human-readable API reference with real `curl` examples; redesigned to match Home's design language (aurora/grid backdrop, gradient hero, glass sidebar with search, live network strip, reading progress bar, prev/next + keyboard navigation, skeletons)
- **API Docs** — Interactive Swagger UI (lazy-loaded), try-all-17-endpoints from the browser; redesigned shell to match the design language (aurora/grid backdrop, gradient hero, live network strip, quick stats, loading skeleton)
- **SDK Portal** — JavaScript SDK reference; redesigned to match the design language (aurora/grid backdrop, gradient hero, live network strip, copyable code blocks, quick stats, quick links, skeletons)
- **Dev Portal** — Developer dashboard; redesigned to match the design language (aurora/grid backdrop, gradient hero, live network strip, SDK/starter-kit/CLI sections, CTA, skeletons)

### Node RPC (port 8545, 17 endpoints)
- `GET /health`, `/status`, `/mempool`, `/metrics`
- `GET /block/{height}`, `/block/hash/{hash}`, `/block/latest`
- `GET /balance/{address}`, `/tx/{hash}`
- `GET /gas_price`, `/fee_history/{count}`
- `POST /submit_tx`, `/estimate_gas`, `/connect_peer`
- `GET /peers`, `/validators`
- `GET /delegations/{address}`
- `POST /faucet/request` — credits address directly in state trie

### Docker Local Testnet (5 nodes)
- 3 validators + 2 RPC nodes, all producing blocks in lockstep
- Node APIs: `8545`–`8549`; P2P: `26656`–`26657`; metrics scrape on `26657` per node
- Prometheus: `http://localhost:9095` (container 9090); Grafana: `http://localhost:3000` (`admin`/`admin`); nginx: `80`
- Live metrics: `blockchain_peer_count`, `blockchain_network_bytes_rx_total/tx_total`, `blockchain_consensus_round`, `blockchain_finalized_height`, `blockchain_mempool_size` all populate on `/metrics`

### Key Bug Fixes Applied
- Rate limiter uses config value (was hardcoded 100 req/s) — default 200 req/s
- Rejection handler no longer defaults to 429 for non-rate-limit errors (e.g. JSON parse → 400)
- `submit_tx` now has rate limiting
- Genesis validators written to state trie at startup (`/validators` returns real data)
- **`/validators` fixed**: handler read `chain_store.get_state(b"validators")` which never matched (the trie persists only node hashes). The RPC server now reads the live `state_trie` directly, so all 3 genesis validators (address, stake, commission) are returned
- Wallet address fixed: derives 20-byte address from 32-byte public key
- Faucet credits the state trie directly instead of just returning success
- "No validators configured" fixed: `get_validator_list()` loads validators from the state trie instead of returning `vec![]`
- Live network/consensus/mempool metrics wired up (`NetworkCommand::GetStats`, per-5s refresh)
- Frontend API URLs fixed from stale `9933` to `8545` (node API port)
- **Restart-stall fixed**: `BlockProducer.current_slot` and BFT start height now resume from the persisted chain tip instead of restarting at slot 0, so `docker-compose restart` no longer breaks consensus (verified across repeated restarts)
- **Consensus stall fixed (BFT liveness)**: quorum threshold `>=` 2/3 so 2 of 3 equal-stake validators can finalize; block slots derived from BFT height with parent from the real chain tip; proposals re-broadcast every 1s; round-sync jumps to a higher round when peers are already voting there. Verified lockstep across all 5 nodes (previously stalled at height 3)
- **Crash-restart loop fixed**: the always-on state-root check crashed any node using the gossip/sync path (produced blocks carry a zero `state_root` placeholder). The check now only runs for non-placeholder roots, and per-event run-loop errors are logged instead of exiting the process (previously 25+ Docker restarts on one validator)
- **Recovery verified**: `docker-compose restart` resumes in lockstep; killing one validator leaves the other two finalizing (2/3 quorum); the restarted node syncs and rejoins without a fork

---

## What Compiles
- **Frontend**: Vite build succeeds (354 KB main + 1.3 MB lazy-loaded API docs)
- **Rust node**: Compiles with `cargo build` (all crates, including heavy deps like rocksdb/libp2p/revm/wasmtime)

---

## Known Issues
- Coin code still building; balance for faucet-credited addresses shows immediately via `/balance` endpoint
- No WebSocket support (HTTP-only RPC)
- No EVM contract deployment UI (needs contract interaction page)
- No transaction history view on the chain (wallet keeps a local history only)
- Delegation/staking RPC returns empty until the delegation feature is implemented (explorer shows an empty state)
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
│  Node RPC (port 8545)                    │
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
