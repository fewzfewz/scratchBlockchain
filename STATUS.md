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

- **Validator set changes**: registration is now an ordered, signed transaction, but it takes effect at the next height rather than an epoch boundary, there is no unbonding, and equivocation evidence is recorded only in each node's local slashing tracker, not slashed on-chain.
- **Bridge trust**: the ETH lock check accepts any successful tx with logs and trusts the requested amount; mints are paid from the dev funding account, so the bridge only exists in dev mode.
- **Consensus parameters from local config**: the max block gas and the governance params (`GOVERNANCE_*` env) feed consensus-critical checks and genesis state but are read per node; they belong in genesis.
- **Amounts**: a transaction's `value` is a `u64` (about 18.4 tokens).
- **Rewards and proposer choice**: vote rewards are split across the whole active validator set rather than paid for actual participation (that needs the previous commit included in the block, as CometBFT's LastCommit); proposer selection ignores stake, so a registered validator that runs no node still gets ~1/N of rounds and they time out.

Fixed in Phase 1 (transactions carry an explicit `public_key`; tx/header hashes are domain-separated and canonical; the RPC, mempool and every executor branch verify signatures; governance/WASM txs consume nonces; state-writing RPC routes only exist with `api.dev_mode = true`, optionally behind `api.admin_token`). Rewards overflow fixed earlier.

Fixed in Phase 2: quorum is strictly `> 2/3` of stake; votes are verified before they can move the round, and a round skip needs `> 1/3` of stake; a conflicting second vote yields `DoubleSignEvidence` (fed to the slashing tracker) instead of replacing the first; the lock and a last-signed watermark are persisted atomically before every signed message is broadcast; finalized blocks carry a commit certificate (+2/3 precommits) that sync verifies; fork choice no longer reorgs to any higher block; the validator set is only reloaded between heights and compared by keys and stakes. The local devnet has 4 validators, so it keeps finalizing with any one of them down.

Fixed in Phase 3: the header `state_root` commits to all state after the parent block — an LtHash over the flat state (accounts, EVM, WASM) combined with the governance/validator trie root. Validators reject proposals whose root (or parent, slot, extrinsics root, base fee, or any transaction signature) doesn't match, and a node that cannot apply a finalized block stops instead of diverging. Each block's whole transition (execution, rewards, governance, upgrades, root) commits as one atomic batch. Genesis balances and rewards now live where balances are read. Faucet, bridge mint and WASM deploy are transactions from a genesis-funded dev account; validator registration and delegation are signed staking transactions that bond tokens. Nondeterministic `HashMap` serialization in governance and runtime-upgrade state was replaced with `BTreeMap`. The mempool no longer loses transactions selected for a block.

Done in Phase 4 (robustness): production crates may not `unwrap`/`expect` (clippy `unwrap_used`/`expect_used` deny via workspace lints; allowed in tests), with the remaining ~50 sites fixed (DB iterator and decode errors no longer panic) and shared state on `parking_lot` locks (no lock poisoning). Gossip messages are validated before they are forwarded (gossipsub `validate_messages`): transactions, votes, proposals and blocks with bad signatures or certificates are rejected and the sender penalized. Fuzz-style tests feed random and mutated gossip to the decoders/validators and random messages to the BFT engine. `rpc.rs` is split into modules by domain.

Verified on the 4-validator Docker devnet (plus 2 RPC nodes): all six nodes agree on every state root; stopping a validator keeps the chain finalizing and it re-syncs on restart; the staking scripts pass. That run caught a real fork — each node paid vote rewards to the precommits it happened to collect — now fixed.

Not done in Phase 4: per-crate typed errors (`thiserror`) instead of `Box<dyn Error>`, and moving `Node` to an actor model so RPC reads a snapshot instead of sharing a dozen `Arc<Mutex<_>>` with consensus.

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
