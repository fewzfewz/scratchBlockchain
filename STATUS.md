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
- **Bridge relayers**: the relayer set can only change through a new genesis; Nebula → Ethereum unlocks are still relayed by one process holding several keys (`interop/scripts/eth-relayer-daemon.js`).
- **Consensus parameters from local config**: the max block gas and the governance params (`GOVERNANCE_*` env) feed consensus-critical checks and genesis state but are read per node; they belong in genesis.
- **Amounts**: a transaction's `value` is a `u64` (about 18.4 tokens).
- **Proposer fairness**: proposer selection is stake-weighted but random per (height, round), not CometBFT's priority rotation, so short-run proposer counts can deviate from stake shares.

Fixed in Phase 1 (transactions carry an explicit `public_key`; tx/header hashes are domain-separated and canonical; the RPC, mempool and every executor branch verify signatures; governance/WASM txs consume nonces; state-writing RPC routes only exist with `api.dev_mode = true`, optionally behind `api.admin_token`). Rewards overflow fixed earlier.

Fixed in Phase 2: quorum is strictly `> 2/3` of stake; votes are verified before they can move the round, and a round skip needs `> 1/3` of stake; a conflicting second vote yields `DoubleSignEvidence` (fed to the slashing tracker) instead of replacing the first; the lock and a last-signed watermark are persisted atomically before every signed message is broadcast; finalized blocks carry a commit certificate (+2/3 precommits) that sync verifies; fork choice no longer reorgs to any higher block; the validator set is only reloaded between heights and compared by keys and stakes. The local devnet has 4 validators, so it keeps finalizing with any one of them down.

Fixed in Phase 3: the header `state_root` commits to all state after the parent block — an LtHash over the flat state (accounts, EVM, WASM) combined with the governance/validator trie root. Validators reject proposals whose root (or parent, slot, extrinsics root, base fee, or any transaction signature) doesn't match, and a node that cannot apply a finalized block stops instead of diverging. Each block's whole transition (execution, rewards, governance, upgrades, root) commits as one atomic batch. Genesis balances and rewards now live where balances are read. Faucet, bridge mint and WASM deploy are transactions from a genesis-funded dev account; validator registration and delegation are signed staking transactions that bond tokens. Nondeterministic `HashMap` serialization in governance and runtime-upgrade state was replaced with `BTreeMap`. The mempool no longer loses transactions selected for a block.

Done in Phase 4 (robustness): production crates may not `unwrap`/`expect` (clippy `unwrap_used`/`expect_used` deny via workspace lints; allowed in tests), with the remaining ~50 sites fixed (DB iterator and decode errors no longer panic) and shared state on `parking_lot` locks (no lock poisoning). Gossip messages are validated before they are forwarded (gossipsub `validate_messages`): transactions, votes, proposals and blocks with bad signatures or certificates are rejected and the sender penalized. Fuzz-style tests feed random and mutated gossip to the decoders/validators and random messages to the BFT engine. `rpc.rs` is split into modules by domain.

Verified on the 4-validator Docker devnet (plus 2 RPC nodes): all six nodes agree on every state root; stopping a validator keeps the chain finalizing and it re-syncs on restart; the staking scripts pass. That run caught a real fork — each node paid vote rewards to the precommits it happened to collect — now fixed.

Done in Phase 5 so far: proposers are chosen in proportion to stake (a validator with a tiny stake, or no node behind it, only costs the rounds its stake is worth). Each block carries its parent's commit certificate (`last_commit`, committed to by `header.last_commit_hash` and checked for a 2/3+ quorum against the set that certified the parent), and vote rewards go to the validators that signed it — rewards follow actual participation and are identical on every node.

Also in Phase 5: bridge mints are `bridge_mint` transactions that must carry Ed25519 attestations from `threshold` distinct relayers of the genesis relayer set (`bridge` in genesis), signed over (chain id, ETH lock tx hash, recipient, amount); consensus verifies them, mints new supply, and records the ETH tx hash in state, so a lock mints once on every node. The relayer reads amount and recipient from the bridge contract's native-ETH `TokensLocked` event rather than the request, and the bridge contract address is node configuration. Missed-block stats were computed for the wrong height (slot instead of BFT height) and counted most blocks as missed; fixed. A deterministic multi-node simulation (`consensus/tests/multi_node_sim.rs`) checks safety and liveness with reordered delivery, a 2|2 partition that heals, validators crashing mid-round and restarting from their persisted safety state, and an equivocating Byzantine validator (which every honest node records evidence against).

Verified on the devnet after a clean restart: all six nodes agree on every state root; a lock on Hardhat's Bridge.sol mints on every node, an inflated mint request and a replay are refused; with a validator stopped the chain keeps finalizing and that validator drops out of the blocks' `last_commit` (no vote rewards) until it re-syncs; proposer counts track stake.

Phase 5, continued: a load test on the devnet found a second state fork. The block hash did not cover the proposer, so two proposers in different rounds could build byte-identical empty blocks, and nodes credited block rewards to whichever proposal they had stored for that hash. The header now carries `proposer` (inside the hash and signature), and rewards and block stats come from it. Sync now rotates across connected peers and keeps responses that arrive after the watchdog gave up, so one peer that stops answering cannot stall catch-up; the sync codec bounds message size before allocating. `RUST_LOG` is honored. The load test also halted the chain: a per-peer rate limiter banned peers that relayed a burst of user transactions, and the ban dropped their consensus messages too. Rate limits now only shed the excess, bans never block (self-authenticating) consensus messages, and the per-peer transaction rate is realistic. Blocks have a byte budget (`MAX_BLOCK_BYTES`, enforced by the producer and by validation) that fits the gossip message limit — previously any block over ~60 transactions exceeded the 64 KB gossip limit and could never be proposed successfully. Round timeouts grow linearly (capped at 30 s) instead of exponentially, so recovery after a long stall takes seconds per round, not minutes. On the devnet, 5,000 transfers submitted in 5 s were all finalized (~240 tx/s sustained, sub-second latency at light load) with all six nodes agreeing on every state root. The light client is now a real header follower (`node/src/light_client.rs`): it checks each block's link and 2/3+ commit certificate against a trusted validator set. It cannot follow validator-set changes on its own yet, because headers do not commit to the next set. Criterion benchmarks of the node's hot paths live in `benchmarks/` (about 8–9k transfers/s execution per core, 20k/s mempool admission, 1.5 ms of CPU per BFT height with 4 validators), and `tests/localhost/scripts/42-throughput.js` measures end-to-end throughput and finality latency on the devnet.

Web UI pass (October 2026): every page and user flow was driven in a headless browser against the devnet (`frontend/e2e/`), which found and fixed several node bugs. EVM execution only ever wrote to an in-memory cache, so contract state never reached the store or the state root, was lost on restart, and was invisible to RPC reads; it now executes directly against the store. The EVM kept its own copy of balances and nonces (synced one way, taking the larger balance — EVM spending reappeared on the native side); there is now one ledger. `/estimate_gas` simulates the transaction instead of counting bytes (it under-estimated every contract call). Contract-creation receipts reported `Some(0x…)` as the address. Sync could wedge on a hung libp2p request: nodes no longer pile up duplicate connections (redialing connected peers, storing ephemeral inbound ports) and a stuck request now resets that peer's connections. `/txs/<address>` uses a per-address index instead of scanning the last 100 blocks (about 20 seconds of chain). `/status` reports the real peer count; `/bridge/status` reports the relayer set. The devnet governance voting period is 300 blocks (test mode no longer overrides an explicit value). In the UI: real compiled ERC20/ERC721 presets (the old bytecode was not valid), the configured RPC URL is used everywhere, a local Hardhat account option for the bridge when MetaMask is absent, and corrected copy (no unmeasured performance claims).

Standalone bridge relayer (`relayer/`, `nebula-relayer`): each relayer independently follows `Bridge.sol` (with a confirmation depth), signs an attestation per native-ETH lock, posts it to the nodes' attestation mailbox (`POST/GET /bridge/attestations`, signature-checked against the genesis relayer set), and the relayers take turns submitting the `bridge_mint` once the threshold is met; progress survives restarts. The devnet runs two relayer containers plus the node's dev key as a 2-of-3 set: an ETH lock mints on Nebula in ~7 s with no manual step, a lock with only one relayer online does not mint, and forged attestations are refused. Also fixed: proposals re-proposing a block built by another validator (Tendermint's valid-value rule) were rejected after the proposer-in-header change, costing rounds; and the faucet's nonce counter could get stuck ahead of the chain.

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
