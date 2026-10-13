# Blockchain Status Summary

## Current Status: Devnet MVP — **not testnet-ready** (see blocking defects below)

**Last Updated**: August 3, 2026  
**Version**: 0.3.4  
**Node RPC**: `http://localhost:8545`  
**Frontend**: `http://localhost:5173`  
**WebSocket**: `ws://localhost:8545/ws`  
**SDK**: `sdk/javascript` → `@modular-blockchain/sdk`

---

## What's Working Now

### Frontend (React SPA, port 5173)
- **Wallet** — Ed25519 keys, send, **on-chain tx history** (`GET /txs/{address}`)
- **Deploy** — EVM ERC20/721 at `/deploy`
- **Explorer** — validators, staking, rewards estimator
- **Faucet** — `POST /faucet/request`
- **Governance** — proposals, voting (signed txs)
- **Validators** — onboarding wizard at `/validators/onboard`
- **Docs / API / SDK** — Swagger UI, dev portal

### Node RPC (32 HTTP routes + `/ws`)

**Core:** status (with `chain_id`), health, blocks, balance, txs, mempool, metrics  
**Staking:** validators, delegate, register, delegations, slashing events  
**Governance:** `/governance`, `/proposal/{id}`  
**Advanced:** MEV (4 routes), AA (2 routes), WASM (3 routes), faucet

### Backend
- BFT + validator hot-reload, pruning, trie GC
- Persistent EVM (`ChainStoreEvmStore`)
- WASM fuel metering
- Reed–Solomon DA, sharks MEV, Merkle commitments

### Developer tooling
- **SDK** — TypeScript client aligned to all RPC routes
- **CLI** — `sdk/javascript/cli` (wallet, deploy, scaffold)
- **Tests** — `tests/localhost/run-all-tests.sh` + CI
- **OpenAPI** — `docs/openapi.yaml` (32 routes)

### Docker local testnet
- 3 validators + 2 RPC nodes (ports 8545–8549)
- Prometheus `:9095`, Grafana `:3000`

---

## Known blocking code defects (October 2026 review)

These must be fixed before any public testnet. Operational items further down come after them.

- **Consensus safety**: quorum is `>= floor(2N/3)` instead of `> 2/3` (`consensus/src/bft.rs`); round jumps happen before signature checks; conflicting votes overwrite each other with no evidence; lock state is not persisted across restarts; synced blocks are not checked against a +2/3 commit certificate.
- **Transaction authentication**: only native transfers verify signatures. EVM, governance and WASM transactions trust `tx.sender`, and the mempool does not verify either (`mempool/src/lib.rs` `recover_signer`).
- **RPC writes state outside consensus**: `/bridge/mint` (mints without an ETH proof when `eth_tx_hash` is omitted), `/validators/register` (any stake, hot-reloaded into BFT), `/faucet/request`, `/delegate`, `/deploy_wasm`, `/runtime/propose`. None require authentication.
- **No real state root**: the header `state_root` hashes only `parent_root + extrinsics_root`, and blocks are committed before that check runs.
- **Hashing**: the transaction hash has no length prefixes, so `chain_id` can be stripped without breaking the signature. The header hash omits `base_fee` and `gas_used`.
- **Rewards**: the default base reward (`1e19`) overflows `i64` and is never paid.

## Mainnet blockers (operational)

- Public testnet 30+ day soak
- Professional security audit
- Bug bounty program
- Production alerting webhooks + multi-region deploy

See `MAINNET_CHECKLIST.md`.

---

## Quick Start

```bash
cd deployment/local && docker compose up -d
cd frontend && npm install && npm run dev
# http://localhost:5173

cd sdk/javascript && npm install && npm run build
```

---

## Architecture

```
┌──────────────────────────────────────────┐
│  Frontend (5173) — 10 routes             │
├──────────────────────────────────────────┤
│  Node RPC (8545) — 32 HTTP + /ws         │
│  SDK @modular-blockchain/sdk             │
├──────────────────────────────────────────┤
│  Rust: consensus │ execution │ storage   │
│  governance │ mev │ network │ interop    │
└──────────────────────────────────────────┘
```

**Status**: Devnet MVP — fix the blocking code defects above before public testnet or audit.
