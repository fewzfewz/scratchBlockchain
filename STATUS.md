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

- **Validator set changes**: validators are still added through the dev-only `/validators/register` route, which writes one node's local state; nothing orders the change through consensus or delays it to an epoch boundary, so nodes can disagree on the set. Equivocation evidence is recorded only in each node's local slashing tracker, not slashed on-chain.
- **No real state root**: the header `state_root` hashes only `parent_root + extrinsics_root`, and blocks are committed before that check runs.
- **Bridge trust**: in dev mode the relayer routes (`/bridge/mint`, `/bridge/unlock`) still credit balances outside consensus, and the ETH lock check accepts any successful tx with logs and trusts the requested amount.

Fixed in Phase 1 (transactions carry an explicit `public_key`; tx/header hashes are domain-separated and canonical; the RPC, mempool and every executor branch verify signatures; governance/WASM txs consume nonces; state-writing RPC routes only exist with `api.dev_mode = true`, optionally behind `api.admin_token`). Rewards overflow fixed earlier.

Fixed in Phase 2: quorum is strictly `> 2/3` of stake; votes are verified before they can move the round, and a round skip needs `> 1/3` of stake; a conflicting second vote yields `DoubleSignEvidence` (fed to the slashing tracker) instead of replacing the first; the lock and a last-signed watermark are persisted atomically before every signed message is broadcast; finalized blocks carry a commit certificate (+2/3 precommits) that sync verifies; fork choice no longer reorgs to any higher block; the validator set is only reloaded between heights and compared by keys and stakes. Note: with `> 2/3`, the 3-validator local devnet (stakes 1e24 / 8e23 / 6e23) halts if validator1 or validator2 is down.

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
