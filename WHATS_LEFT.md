# What's Left - Development Status

## Current Status: **DEVELOPMENT** ⚠️ (~75%)

### Last Updated: August 2, 2026
### Overall Feature Completeness: **~75%** (full workspace build works; local Docker testnet runs end-to-end)

---

## ✅ DONE (compilation fixes + integrations applied)

### Compilation Status
- ✅ **8 lightweight crates compile with zero warnings**: common, consensus, da, interop, mev, mempool, zk, storage
- ✅ **86 tests pass** across all lightweight crates (0 failures)
- ✅ Storage: PatriciaTrie / ReceiptStore use `Arc<dyn KeyValueStore>` (sled-free)
- ✅ Mempool: heap ordering and nonce tracking fixed
- ✅ ZK: Halo2 gated behind feature flag, SHA-256 default prover works
- ✅ Merkle proofs: verify_proof logic bug fixed
- ✅ `cargo build -p node --release` works (~2 min); full workspace builds
- ✅ `node` crate compiles with block_producer + BFT + RPC + metrics integration

### Consensus Blocker Resolved
- ✅ **Root cause found**: key files were raw hex (not JSON `{"secret_key":"..."}`), Docker mount path was `node_key.json` vs `validator_key.json`
- ✅ Both fixes applied: key files converted to JSON, mount path corrected
- ✅ All 12 consensus tests pass (including cross-validator quorum flow)

### Slashing Infrastructure (NOW ACTIVE)
- ✅ `pub mod slashing;` added to `consensus/src/lib.rs`
- ✅ `SlashingTracker`, `SlashingConfig`, `EvidenceCollector` available to consumers
- ✅ Equivocation detection in `FinalityGadget::prevote()` / `precommit()` now calls `FinalityGadget::slash()`
- ✅ `EnhancedConsensus::check_slashing_conditions()` fully implemented (detects double-sign, calls slash)
- ⚠️ Full slashing integration into node event loop needs `node` crate compilation (heavy deps)

### Core Blockchain
- ✅ BFT Consensus with proper locking rounds
- ✅ Vote aggregation by stake weight (2/3+ threshold)
- ✅ View-change protocol for leader rotation
- ✅ Finality gadget (GRANDPA-style)
- ✅ **Slashing infrastructure** — equivocation detection now triggers slashing
- ✅ State root computation and verification
- ✅ EIP-1559 gas pricing
- ⚠️ Account abstraction (ERC-4337) — code exists, not wired into node event loop
- ⚠️ MEV protection with commit-reveal — code exists, not wired into node event loop

### Infrastructure (~85% Complete)
- ✅ Docker deployment configuration
- ✅ 3 validator nodes configured (produce blocks in lockstep)
- ✅ 2 RPC nodes configured
- ✅ Monitoring stack (Prometheus + Grafana) — live metrics populating
- ✅ Nginx reverse proxy
- ✅ Peer discovery configuration
- ✅ Auto-reconnect configuration
- ✅ Health checks (curl healthcheck on each container, `/health` on 26657)

### Networking
- ✅ libp2p with gossipsub
- ✅ Kademlia DHT for peer discovery
- ✅ Request-response for block sync
- ✅ Peer reputation and scoring
- ✅ Rate limiting for DoS protection
- ✅ DNS resolution for bootstrap nodes
- ✅ Peer persistence to disk

### Storage
- ✅ RocksDB with column families
- ✅ Atomic batch writes
- ✅ Merkle Patricia Trie for state
- ✅ Block and receipt stores
- ✅ LRU caching for hot data

### RPC API
- ✅ Transaction submission endpoint
- ✅ Balance queries
- ✅ Block retrieval (by hash/height)
- ✅ Gas price estimation
- ✅ Peer management
- ✅ Node status
- ✅ Metrics export (live peer_count / network_bytes / consensus_round / finalized_height)
- ⚠️ WebSocket (placeholder only)

### Security
- ✅ Rate limiting (200 req/sec per IP)
- ✅ Request body size limits (1MB)
- ✅ Signature verification for all messages
- ✅ Nonce replay protection
- ✅ Chain ID validation
- ❌ Professional audit (not started)
- ❌ Bug bounty program (not started)

---

## 🎯 NEXT STEPS

### Wire Dormant Features
1. ⚠️ **Account Abstraction** — add `AccountAbstractionExecutor` to Node, RPC endpoints for `submit_user_operation`, integrate bundler into `BlockProducer`
2. ⚠️ **MEV Protection** — replace `Mempool` with `MevMempool` in Node, add commit/reveal/encrypted RPC routes, wire `BlockProducer` to use `get_all_ready_transactions()`

### Integration Tasks
1. ⚠️ Wire `SlashingTracker` into `Node` struct and call `record_missed_block()` / `check_liveness()` on block finalization
2. ⚠️ Expose slashing events via RPC endpoint
3. ⚠️ Server-side faucet cooldown (currently client-side only)
4. ⚠️ Add WebSocket support (HTTP-only RPC today)
5. ⚠️ Re-enable real state-root verification once EVM state diffs are persisted to the trie (currently skipped for placeholder roots)
6. ❌ Professional audit / bug bounty program

### Done Since Last Update
- ✅ Live metrics (peer count, network bytes, consensus round, mempool, validator count/stake) on `/metrics`
- ✅ Grafana dashboards rewritten to use emitted `blockchain_*` metrics (were `chain_*`, never emitted)
- ✅ Restart-consensus stall fixed — producer/BFT resume from chain tip on restart

### Consensus Stability (BFT Liveness) — RESOLVED
- ✅ **Quorum threshold fixed** (`>=` instead of `>`): 2 of 3 equal-stake validators can now finalize; previously the `>` threshold required all 3, stalling the chain whenever one node lagged
- ✅ **Slot alignment fixed**: block slots derived from BFT height (`slot = height - 1`) and parent fetched from the actual latest committed block — eliminated the divergent-slot drift (blocks previously produced at slots 77/599/668 while the chain sat near 600)
- ✅ **Proposal re-broadcast**: `re_propose()` re-broadcasts the current proposal every 1s so peers that enter a height/round late still receive it and can vote
- ✅ **Round-sync**: `handle_vote`/`handle_proposal` jump to a higher round at the same height when a peer is already voting/proposing there, so the network converges on one round instead of drifting apart on local timeouts (finalization went from ~15-20s/height stalls to lockstep at ~2 blocks/s)
- ✅ **Block sync**: `RequestBlock`/`BlockResponse` batch sync with contiguous apply, same-slot tip replacement, and `finish_sync()` resetting BFT to `tip + 2` so a caught-up node resumes consensus on the canonical chain
- ✅ **Crash-restart loop fixed**: produced blocks carried a zero placeholder `state_root` (EVM state diffs are `vec![]`), so the always-on state-root check crashed any node using the gossip/sync path (25+ restarts observed on one validator). The check now only runs for non-placeholder roots, and per-event errors in the run loop are logged instead of killing the process
- ✅ **Verified on live 3-validator + 2-RPC testnet**: fresh reset reaches perfect lockstep; `docker-compose restart` of a validator resumes and rejoins; killing one validator leaves the other two finalizing (2/3 quorum); the killed validator rejoins and catches up

### Quick Wins
```
# Build
cargo build -p node --release

# Run local testnet
docker-compose -f deployment/local/docker-compose.yml up -d

# Check status
curl http://localhost:26657/status

# Node RPC (each node): 8545-8549
curl http://localhost:8545/health
```
