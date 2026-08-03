# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

### SDK & developer docs (Aug 2026)
- SDK `HttpProvider` aligned to all 32 node RPC routes
- `ModularClient`: tx history, WASM, faucet, validators, MEV, AA methods
- `/status` returns `chain_id` for SDK `getChainId()`
- `BLOCKCHAIN_COMPARISON.md` rewritten for v0.3.4
- Pre-mainnet marked **100% code-complete**

---

## [0.3.4] - 2026-08-03

### Added

#### Tier A — Polish
- OpenAPI synced (32 routes including WASM, MEV, AA)
- CI integration job with Docker testnet smoke tests
- WASM: fuel metering (wasmtime 21), `POST /deploy_wasm`, `POST /call_wasm`, `GET /wasm/contracts`
- Block producer executes `WASM:<name>:<func>:<arg>` transaction payloads
- Bridge integration test improvements
- Rollup fraud proofs re-execute via EVM and compare `state_root()`

#### Tier B — Production
- `ChainStoreEvmStore` — EVM accounts/storage/code persist in RocksDB
- MEV Shamir secret sharing via `sharks` crate
- DA Merkle-root blob commitments with opening proofs

#### Tier C — Ops
- Adversarial slashing tests
- `deployment/cloud/configs/alertmanager.prod.yml.example`

### Changed
- wasmtime upgraded 14 → 21 for fuel metering
- Docs updated: ROADMAP, da/README, interop/README, FEATURES_EXHAUSTIVE

---

## [0.3.3] - 2026-08-03

### Added

#### BFT Validator Hot-Reload
- `BftEngine::update_validator_set()` and `FinalityGadget::update_validator_set()`
- `governance_store::load_consensus_validators()` — active validators with stake + delegated weight
- Node syncs validator set after block finalize and every 5s metrics tick
- Startup loads validators from state trie when `chain_tip > 0`

#### Patricia Trie GC
- `PatriciaTrie::gc_orphan_nodes()` — removes unreachable 32-byte state nodes
- Runs automatically after block/receipt prune pass

#### Integration Tests
- `12-vote-proposal.js`, `13-execute-proposal.js`
- `17-unstake-tokens.js`, `18-register-validator.js`
- `run-all-tests.sh` updated with governance vote/execute and staking lifecycle

#### Crypto Hardening
- DA: Reed–Solomon erasure coding via `reed-solomon-erasure`
- MEV: AES-256-GCM encryption replaces XOR at rest
- ZK: block hash bound into state-transition proofs; batch verify checks 32-byte proofs

#### Public Testnet & Audit Prep
- `PUBLIC_TESTNET.md` and `deployment/cloud/scripts/deploy-public-testnet.sh`
- `AUDIT_READINESS.md` and `MAINNET_CHECKLIST.md`

### Fixed
- `PatriciaTrie::gc_orphan_nodes()` batch delete key type (`Vec<u8>` vs `&Vec<u8>`)
- `node` binary: use `node::governance_store` module path consistently

---

## [0.3.2] - 2026-08-03

### Added

#### State Pruning
- `storage/src/pruner.rs` with archive/full/minimal modes
- Automatic prune after block finalize (`blocks_to_keep`, `prune_every_n_blocks` config)

#### Testing
- Localhost integration tests: governance propose, stake/delegate, bridge readiness, upgrade proposal
- Load test (`40-load-test.js`) and chaos smoke test (`41-chaos-smoke.js`)
- Shared test helpers under `tests/localhost/scripts/lib/`

#### Validator Ops
- `/validators/onboard` frontend page with health checks and registration form
- Grafana `validator-onboarding` dashboard
- Alertmanager in local Docker stack; Prometheus alerts on `blockchain_*` metrics

---

## [0.3.1] - 2026-08-03

### Added

#### Node RPC
- `GET /txs/{address}?limit=N` — on-chain transaction history (scans recent blocks)

#### Frontend
- `/deploy` — contract deployment page (ERC20/ERC721 presets, gas estimate)
- Wallet merges on-chain history from `GET /txs/{address}` with local pending txs
- Staking explorer rewards estimator (5.2% APR minus validator commission)

#### Stub Improvements
- Interop token registry: real Ethereum mainnet addresses + mapped Nebula addresses
- Network sync: deterministic state roots, state chunks, pending block request tracking
- `WasmExecutor::execute_i32` for i32 arg/return WASM calls
- SDK deploy CLI: preset ERC20/ERC721 init bytecode

---

## [0.3.0] - 2026-08-03

### Added

#### Node Integration (`node/src/tx_pool.rs`)
- **`TxPool`** — unified transaction pool wrapping `MevMempool` + `AccountAbstractionExecutor`
- Block producer pulls AA bundles first, then MEV-ready and regular mempool transactions

#### New RPC Endpoints (28 total: 27 HTTP + WebSocket)
- `POST /submit_user_operation` — ERC-4337 account abstraction
- `GET /user_operations/pending` — pending AA operation count
- `POST /mev/commit`, `POST /mev/reveal` — commit-reveal MEV protection
- `POST /mev/encrypted`, `POST /mev/decryption_share` — threshold-encrypted mempool
- `GET /slashing/events` — slashed validator list
- `POST /delegate` — delegate stake to validator
- `POST /validators/register` — dynamic validator registration
- `GET /ws` — WebSocket `newHead` events (every 2s)
- `GET /governance`, `GET /proposal/{id}` — on-chain governance queries (documented)

#### Economic Engine
- 50% fee burn logged per finalized block
- 10% of block fees credited to on-chain treasury

#### Consensus
- BFT height re-anchor every 10s when drifted from chain tip
- Deterministic state roots: `SHA256(parent_state_root || extrinsics_root)`

### Changed
- RpcServer uses `TxPool` instead of bare `Mempool`
- `SlashingTracker` wired into node (height tracking on finalize)
- Server-side faucet cooldown enforced on `POST /faucet/request` (60s per address)
- State root verification uses deterministic hash formula (non-zero roots only)

### Known Limitations
- OpenAPI spec (`docs/openapi.yaml`) not yet updated for all new routes
- MEV threshold encryption uses simplified XOR (not full Shamir)
- WASM executor, ZK/KZG, DA erasure coding, bridge crypto still stubs
- No EVM contract deployment UI; no on-chain tx history page
- BFT engine does not hot-reload after `POST /validators/register`

---

## [0.2.0] - 2026-06-25

### Added

#### Frontend (Unified React SPA)
- **Single-page application**: All 7 separate frontends unified into one React SPA (Vite + Tailwind + React Router)
- **Dark/light theme**: Toggle via Sun/Moon icon, persisted in localStorage, CSS variables on `:root` / `.dark`
- **Mobile sidebar**: Hamburger menu with all navigation links
- **Wallet page**: Ed25519 key pair generation via TweetNaCl, 20-byte address derivation (SHA-256 of pubkey), token transfer signing, EIP-1559 gas params, pre-filled test addresses
- **Faucet page**: Requests tokens directly from node's `POST /faucet/request` endpoint (no separate backend), offline detection banner, local request limit tracking
- **Explorer page**: Dashboard (chain status), Validators (genesis validators from state trie), Staking tab — all with light/dark mode
- **Governance page**: Proposals list, vote creation modal, create proposal form — light/dark compatible
- **Docs page**: 9-section human-readable API reference with real `curl` response examples
- **API Docs page**: Interactive Swagger UI (lazy-loaded) — try all 17 RPC endpoints from the browser
- **SDK Portal page**: JavaScript SDK reference
- **Navbar**: Chain status indicator, theme toggle, links to all 9 pages

#### Node RPC
- `GET /validators` — Returns genesis validators from state trie (seeded at startup)
- `GET /block/latest` — Retrieve the most recent block
- `GET /delegations/{address}` — Query delegations for an address
- `POST /faucet/request` — Directly credit an account with test tokens in the state trie

#### OpenAPI Specification
- `docs/openapi.yaml` — Complete OpenAPI 3.0 spec for all 17 endpoints with schemas and example responses

### Changed

- **Rate limiting**: Now reads from `config.api.rate_limit` instead of hardcoded 100 req/s (default 200 req/s)
- **Rejection handler**: Rate-limit rejections return 429; all other unrecognized rejections return 400 (was incorrectly defaulting to 429)
- `submit_tx` endpoint now has rate limiting applied
- Genesis validators are written to the state trie at startup (`/validators` returns 3 genesis validators from `genesis.json`)
- Wallet address: Now correctly uses SHA-256(pubkey)[0..20] as the 20-byte address instead of the raw 32-byte public key
- `start-frontends.sh` — Only serves unified SPA + faucet (which is now built into the node)
- `README.md` — Fully rewritten with quick start, port mapping, RPC table, troubleshooting table, project structure

### Fixed

- **TweetNaCl library not loaded**: Added `tweetnacl` import in WalletPage instead of relying on global `nacl` variable
- **Faucet never credited tokens**: `POST /faucet/request` now writes directly to the state trie (old separate faucet backend at port 3006 only checked cooldown limits, never modified state)
- **429 on transaction submit**: JSON parse errors returned 429 due to catch-all in rejection handler; now correctly returns 400; `submit_tx` also had no rate limiter applied
- **Balance always 0**: Wallet was using 32-byte public key as address; Rust `Address` is 20 bytes; now derives 20-byte address

### Dependencies

- Frontend: Added `tweetnacl` (key generation), `swagger-ui-react` (API docs), `lucide-react` (icons)
- Rust: No new crate dependencies

### Known Limitations

- OpenAPI spec may lag behind latest RPC routes (see README RPC table)
- MEV/ZK/DA crypto uses simplified implementations in places
- No EVM contract deployment UI
- No transaction history page
- Rust build requires ~5 GB disk space

---

## [0.1.0] - 2025-11-24

### 🎉 Initial Testnet Release

This is the first public release of the Modular Blockchain. The system is **testnet-ready** with production-grade infrastructure.

### Added

#### Core Infrastructure
- **Consensus**: BFT consensus with GRANDPA finality gadget
- **Execution**: Multi-VM support (EVM, Native, WASM-ready)
- **Storage**: Persistent storage with `sled` database
- **Network**: libp2p-based P2P networking (gossipsub, kademlia, request-response)
- **Mempool**: Transaction pool with MEV protection (threshold encryption)
- **Fork Choice**: Fork detection and chain reorganization logic

#### Security & Operations
- **Rate Limiting**: API rate limiting per IP address
- **Peer Reputation**: Reputation system with blacklist/whitelist
- **Request Validation**: Input sanitization and validation
- **Circuit Breaker**: Graceful shutdown mechanism
- **Monitoring**: Prometheus metrics and Grafana dashboards
- **Docker**: Multi-node Docker Compose deployment
- **Scripts**: Automated deployment, backup, and restore scripts

#### User Interfaces
- **Block Explorer**: Real-time blockchain visualization
  - Live block and transaction display
  - Network statistics (TPS, peer count, block height)
  - Dark mode with glassmorphism design
- **Web Wallet**: Browser-based wallet
  - Ed25519 key generation and management
  - Balance checking
  - Transaction signing and submission
- **Documentation Site**: Comprehensive API documentation
  - Installation guide
  - Quick start tutorial
  - RPC API reference

#### Tokenomics & Governance
- **Block Rewards**: Fixed block reward distribution (10 tokens per block)
- **Staking Contract**: Basic validator staking structure
- **Governance**: Proposal creation and voting system
- **Genesis Config**: Initial token distribution and validator set

#### Interoperability
- **Bridge Contract**: Cross-chain asset locking/unlocking
- **Relayer Service**: Mock relayer for cross-chain messaging
- **Message Format**: Standardized cross-chain message structure

#### Testing & Quality
- **Unit Tests**: Comprehensive unit test coverage
- **Integration Tests**: Multi-node consensus tests, load tests, chaos tests
- **Benchmarks**: Performance benchmarking suite
- **Security Tests**: Rate limiting, reputation, and validation tests

### Technical Specifications

- **Language**: Rust 1.75+
- **Consensus**: BFT + GRANDPA (2/3 finality threshold)
- **Block Time**: ~6 seconds (configurable)
- **Finality**: < 3 seconds
- **Target TPS**: 10,000+ (theoretical)
- **Signature Scheme**: Ed25519
- **Hash Function**: SHA-256
- **Database**: sled (embedded key-value store)

### Dependencies

#### Core
- `tokio` - Async runtime
- `serde` - Serialization
- `ed25519-dalek` - Cryptography
- `libp2p` - P2P networking
- `sled` - Storage
- `warp` - RPC server

#### Execution
- `revm` - EVM implementation
- `wasmtime` - WASM runtime

#### Zero-Knowledge
- `halo2` - ZK proofs

#### Monitoring
- `prometheus` - Metrics
- `grafana` - Dashboards

### Known Limitations

- **Single Validator**: Currently optimized for single-node testing
- **No TLS**: RPC endpoints run without TLS (commented out)
- **Basic Tokenomics**: Fixed block rewards, no dynamic inflation
- **Mock Bridges**: Bridge contracts exist but relayers not deployed
- **Limited Governance**: Basic voting, no runtime upgrades

### Breaking Changes

N/A (initial release)

---

## Release Notes

### v0.1.0 Highlights

**What Works**:
- ✅ Node starts and runs consensus
- ✅ Transactions can be submitted via RPC
- ✅ Blocks are produced and finalized
- ✅ Network layer handles peer discovery
- ✅ UIs connect to RPC and display data
- ✅ Metrics are collected and exportable

**What's Next** (see [ROADMAP.md](ROADMAP.md)):
- 🚧 Dynamic tokenomics and staking
- 🚧 Live bridge deployment
- 🚧 Security audit
- 🚧 Developer SDKs
- 🚧 Public testnet campaign
- 🚧 Mainnet launch

### Migration Guide

N/A (initial release)

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to contribute.

---

## Links

- **Repository**: https://github.com/YOUR_USERNAME/modular-blockchain
- **Documentation**: `docs/index.html`
- **Roadmap**: [ROADMAP.md](ROADMAP.md)
- **Issues**: https://github.com/YOUR_USERNAME/modular-blockchain/issues

---

[Unreleased]: https://github.com/YOUR_USERNAME/modular-blockchain/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/YOUR_USERNAME/modular-blockchain/releases/tag/v0.1.0
