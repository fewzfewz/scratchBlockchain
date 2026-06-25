# Scratch Blockchain vs Other Chains

Detailed comparison of Scratch (Nebula) against Bitcoin, Ethereum, Polkadot, Solana, and Cosmos — covering architecture, consensus, execution, UIs, and what each chain is missing.

---

## 1. High-Level Overview

| | **Scratch** | **Bitcoin** | **Ethereum** | **Polkadot** | **Solana** | **Cosmos** |
|---|---|---|---|---|---|---|
| **Launched** | Dev only | 2009 | 2015 | 2020 | 2020 | 2019 |
| **Market cap** | — | ~$1T+ | ~$300B+ | ~$8B | ~$10B | ~$3B |
| **Consensus** | BABE-like BFT | PoW (SHA-256) | PoS (Gasper) | NPoS (BABE+GRANDPA) | PoH + Tower BFT | Tendermint BFT |
| **Finality** | ~6s (deterministic) | ~60min (probabilistic) | ~12-15min (probabilistic) | ~12-60s (deterministic) | ~2s (deterministic) | ~7s (deterministic) |
| **Block time** | ~6s | ~10min | ~12s | ~6s | ~400ms | ~7s |
| **Model** | Account | UTXO | Account (EVM) | Account (WASM) | Account | Account (CosmWASM) |
| **Smart contracts** | EVM + WASM | None (Script) | EVM (Solidity) | WASM (ink!) | BPF (Rust/C) | WASM (CosmWASM) |
| **Languages** | Rust | C++ | Go/Rust/Nim | Rust | Rust | Go |
| **Single binary?** | Yes | Yes | No (EL+CL) | No (relay+para) | Yes | Yes |
| **Frontend** | Built-in SPA | No standard UI | DApp ecosystem | Subscan/Polkadot.js | Solscan/Phantom | Mintscan/Keplr |
| **Docker** | Yes | Yes | Yes | Yes | Yes | Yes |

---

## 2. Consensus Deep Dive

| | **Scratch** | **Bitcoin** | **Ethereum** | **Polkadot** | **Solana** | **Cosmos** |
|---|---|---|---|---|---|---|
| **Type** | BFT (round-robin leader) | PoW (Nakamoto) | PoS (Casper FFG + LMD-GHOST) | NPoS (BABE + GRANDPA) | PoH + Tower BFT (PBFT) | BFT (Tendermint) |
| **Validators** | 3 (genesis), up to 100 | Miners (anyone) | 500K+ stakers, ~30 active proposers | ~300 active validators | ~2,000 active | 100-150 |
| **Finality** | 2/3 vote → final | 6 blocks ≈ 60min | 2/3 of staked ETH → final (~12-15min) | GRANDPA: 2/3 of NPoS → final | 2/3 of validator set → final | 2/3 of validator set → final |
| **Fork choice** | BFT round (no forks) | Heaviest chain | LMD-GHOST | BABE fork choice + GRANDPA finalize | PoH as clock, Tower fork choice | Tendermint (no forks at same height) |
| **Slashing** | Double-sign (5%), downtime (0.1%), invalid state (10%) | None (wasted electricity) | Slashing (offline, equivocation) | Slashing (offline, equivocation) | Slashing (equivocation, voting disagreement) | Slashing (double-sign, downtime) |
| **What Scratch is missing** | Dynamic validator set rotation (static genesis only) | — | 500K+ staker support | Native parachain auction/slot mechanism | PoH clock for sub-second block times | IBC integration for cross-chain |


## 3. Execution & Smart Contracts

| | **Scratch** | **Bitcoin** | **Ethereum** | **Polkadot** | **Solana** | **Cosmos** |
|---|---|---|---|---|---|---|
| **VM** | EVM (revm) + WASM (wasmtime) | Bitcoin Script (stack-based, non-Turing-complete) | EVM (Solidity, Yul, Huff) | WASM (ink!, Ask!) | BPF (Rust, C, C++) | WASM (CosmWASM) |
| **Gas model** | EIP-1559 (base fee + priority fee) | Fee per byte | EIP-1559 (base fee + priority fee) | Weight-based | Compute budget (compute units) | Gas per operation |
| **Parallel execution** | Yes (rayon-based) | No (single-threaded) | No (sequential EVM; after Dencun: blob parallel) | No (sequential WASM) | Yes (Sealevel, transaction read-set) | No (sequential) |
| **Account abstraction** | ERC-4337 code exists (dormant) | No | ERC-4337 (live on mainnet since 2023) | On-chain native (any call filter) | Native (any BPF program) | Native (CosmWASM) |
| **State model** | Patricia trie (Merkle) | UTXO set | Merkle Patricia Trie (hexary) | Merkle trie (Blake2) | Verifiable Delay Function + Bank state | IAVL tree |
| **What Scratch is missing** | Live EVM contract deployment from UI, active account abstraction | — | — | Production parachain slot | Sealevel-style parallelization over read/write sets | IBC, interchain accounts |

---

## 4. Frontend & User Experience

| | **Scratch** | **Bitcoin** | **Ethereum** | **Polkadot** | **Solana** | **Cosmos** |
|---|---|---|---|---|---|---|
| **Official UI** | Built-in SPA (9 pages) | None (bitcoin-cli only) | None (MetaMask + Etherscan) | None (Polkadot.js) | None (Phantom + Solscan) | None (Keplr + Mintscan) |
| **Wallet UI** | Built-in (Ed25519, send, receive) | Bitcoin Core QT, electrum | MetaMask, Rabby, WalletConnect | Polkadot.js, Talisman, Nova | Phantom, Backpack, Solflare | Keplr, Cosmostation, Leap |
| **Explorer** | Built-in (dashboard, validators, staking) | Blockchain.com, mempool.space | Etherscan, Etherchain | Subscan, Polkaholic | Solscan, SolanaFM | Mintscan, Big Dipper |
| **API Docs** | Built-in interactive Swagger UI | Bitcoin RPC docs | Ethereum JSON-RPC spec | Substrate RPC docs | Solana RPC docs | Cosmos SDK docs |
| **Faucet** | Built-in (RPC endpoint) | None | Sepolia/Goerli faucets | Westend/Polkadot faucet | Solana devnet faucet | Cosmos testnet faucets |
| **Governance UI** | Built-in (proposals, voting) | BIP process (off-chain) | Tally, Snapshot (on/off-chain) | Polkadot.js gov | Realms (SPL Gov) | Mintscan gov |
| **Dev portal** | Built-in | None | Ethereum.org, Remix | Substrate docs, ink! hub | Solana dev docs | Cosmos SDK docs |
| **Dark mode** | Built-in (toggle, persisted) | Varies by 3rd party | Varies by dApp | Varies by tool | Varies by tool | Varies by tool |
| **Mobile** | Responsive (hamburger menu) | Varies | WalletConnect ecosystem | Nova wallet, Fearless | Phantom mobile | Keplr mobile |
| **What Scratch is missing** | Contract interaction UI, tx history, WebSocket push | Comprehensive wallet | DApp ecosystem (MetaMask, Uniswap, etc.) | Parachain-specific explorers | High-performance explorer at scale | IBC explorer |

---

## 5. Networking & P2P

| | **Scratch** | **Bitcoin** | **Ethereum** | **Polkadot** | **Solana** | **Cosmos** |
|---|---|---|---|---|---|---|
| **Library** | libp2p | Custom (BTC wire protocol) | DevP2P (RLPx) | libp2p | Custom (UDP-based Turbine) | libp2p / Tendermint |
| **Peer discovery** | Kademlia DHT | DNS seeds + addr messages | Discv5 (Kademlia) | Kademlia DHT (libp2p) | Gossip + stake-weighted | PEX + seed nodes |
| **Gossip** | Gossipsub | Inventory-based | DevP2P snap/syn | Gossipsub (libp2p) | Turbine (UDP, stake-weighted) | Tendermint P2P |
| **Encryption** | Noise (libp2p) | Optional (tor) | DevP2P auth | Noise (libp2p) | QUIC | TLS / Noise |
| **What Scratch is missing** | Production multi-node testnet with 10+ peers | Full Bitcoin node network | Discv5, ENR, snap sync | Parachain block gossip via relay chain | Turbine-style UDP broadcast at scale | IBC light client peer discovery |

---

## 6. Storage & Performance

| | **Scratch** | **Bitcoin** | **Ethereum** | **Polkadot** | **Solana** | **Cosmos** |
|---|---|---|---|---|---|---|
| **Database** | RocksDB / in-memory | LevelDB | LevelDB / Pebble | RocksDB | RocksDB | RocksDB / IAVL |
| **State size** | Tiny (dev) | ~5 GB (UTXO set) | ~500 GB+ (full) | ~50 GB+ | ~100 GB+ | ~10-50 GB |
| **TPS (theoretical)** | ~1,000 | ~7 | ~15 (L1), ~100K+ (L2) | ~1,000 (parachain) | ~5,000 (mainnet peak) | ~10,000 (Tendermint) |
| **TPS (actual)** | Untested | ~4-7 | ~12-15 (L1) | ~100-200 | ~2,500 | ~1,000-5,000 |
| **Pruning** | Not implemented | Configurable (txindex) | Configurable (snap sync) | Yes (state pruning) | Yes (epoch-based) | Yes (pruning options) |
| **What Scratch is missing** | Production load testing, state pruning, archival mode | — | Full sync at 500GB+ state | — | Sealevel parallel execution across 100+ cores | IBC state management |

---

## 7. Development Status

| | **Scratch** | **Bitcoin** | **Ethereum** | **Polkadot** | **Solana** | **Cosmos** |
|---|---|---|---|---|---|---|
| **Production ready** | No (pre-alpha) | Yes | Yes | Yes | Yes | Yes |
| **Security audit** | None | Constant (15+ years) | Multiple audits | Web3 Foundation audits | Multiple audits | Multiple audits |
| **Bug bounty** | None | Yes (up to $1M) | Yes (Immunefi) | Yes (Immunefi) | Yes (Immunefi) | Yes (Immunefi) |
| **Mainnet validators** | 3 (local) | ~1M miners | ~500K+ stakers | ~300 validators | ~2,000 validators | 100-150 per zone |
| **Public testnet** | No | Yes (testnet3, signet) | Yes (Sepolia, Holesky) | Yes (Westend, Rococo) | Yes (devnet, testnet) | Yes (various) |
| **SDKs** | JS (basic, not published) | bitcoinjs, rust-bitcoin | ethers.js, web3.js, viem | polkadot.js | @solana/web3.js | cosmjs, cosmos-kit |

---

## 8. Scratch Advantages (What's Better)

| Advantage | Details |
|-----------|---------|
| **Single binary** | One `node` binary runs everything — no EL/CL split, no relay/para chain, no separate RPC node. Start it and go. |
| **Built-in frontend** | Wallet, explorer, faucet, governance, API docs — all in one SPA. No need to install MetaMask, find an explorer, or search for faucets. |
| **Interactive API docs** | Swagger UI at `/api-docs` — try all 17 endpoints from the browser. Ethereum/Bitcoin don't have this built in. |
| **Dark mode everywhere** | Consistent dark/light theme across all pages, persisted, zero configuration. |
| **Direct faucet** | `POST /faucet/request` credits the state trie immediately — no separate faucet process, no waiting for block inclusion. |
| **All-in-one quick start** | `./start.sh` builds and starts everything — node + frontend. |
| **Code breadth** | Has code for EVM, WASM, ZK (Halo2), MEV (threshold encryption), DA (erasure coding), rollups, bridges, runtime upgrades — most projects only do one or two of these. |
| **Learning tool** | Designed for learning — all concepts (consensus, execution, networking, storage, frontend) in one repo with one language (Rust + React). |

---

## 9. Scratch Disadvantages (What's Missing vs Others)

| Missing Feature | Scratch | Bitcoin | Ethereum | Polkadot | Solana | Cosmos |
|---|---|---|---|---|---|---|
| **Security audit** | ❌ None | ✅ Audited | ✅ Audited | ✅ Audited | ✅ Audited | ✅ Audited |
| **Public testnet** | ❌ None | ✅ testnet3 | ✅ Sepolia | ✅ Westend | ✅ devnet | ✅ testnets |
| **>3 validators** | ❌ Static genesis | ✅ Unlimited | ✅ 500K+ | ✅ 300 | ✅ 2,000 | ✅ 100-150 |
| **Live EVM contracts** | ❌ No deployment UI | N/A | ✅ 1M+ contracts | ✅ via Moonbeam | ✅ via Neon EVM | ✅ via Evmos |
| **Tx history page** | ❌ Not built | ✅ Various | ✅ Etherscan | ✅ Subscan | ✅ Solscan | ✅ Mintscan |
| **WebSocket** | ❌ HTTP only | ✅ ZMQ | ✅ WebSocket | ✅ WebSocket | ✅ WebSocket | ✅ WebSocket |
| **DApp ecosystem** | ❌ None | ❌ Minimal | ✅ Large | ✅ Growing | ✅ Growing | ✅ Growing |
| **Mobile wallet** | ❌ None | ✅ Various | ✅ MetaMask | ✅ Nova | ✅ Phantom | ✅ Keplr |
| **Cross-chain IBC** | ❌ Code only | ❌ | ❌ | ✅ XCMP | ❌ Wormhole | ✅ IBC native |
| **Sub-second blocks** | ❌ ~6s | ❌ ~10min | ❌ ~12s | ❌ ~6s | ✅ ~400ms | ❌ ~7s |
| **Native stablecoin** | ❌ None | ✅ USDT (Omni) | ✅ USDC, USDT, DAI | ✅ aUSD, USDT | ✅ USDC, USDT | ✅ USDC, USDT |
| **Market cap / value** | ❌ $0 | ✅ $1T+ | ✅ $300B+ | ✅ $8B | ✅ $10B | ✅ $3B |

---

## 10. Summary Table — What Scratch Has vs Each Chain

| Feature | Scratch | Bitcoin | Ethereum | Polkadot | Solana | Cosmos |
|---|---|---|---|---|---|---|
| BFT consensus | ✅ | ❌ (PoW) | ✅ (PoS) | ✅ (NPoS) | ✅ (Tower) | ✅ (Tendermint) |
| Account model | ✅ | ❌ (UTXO) | ✅ | ✅ | ✅ | ✅ |
| Smart contracts | ✅ (EVM+WASM) | ❌ (Script) | ✅ (EVM) | ✅ (WASM) | ✅ (BPF) | ✅ (WASM) |
| EIP-1559 gas | ✅ | ❌ | ✅ | ❌ (weight) | ❌ (CU) | ❌ (gas) |
| Parallel execution | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ |
| Slashing | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| Staking delegation | ✅ | ✅ (pooling) | ✅ | ✅ | ✅ | ✅ |
| Treasury | ✅ | ❌ | ✅ | ✅ | ❌ | ✅ |
| Governance on-chain | ✅ | ❌ (BIP) | ✅ | ✅ | ✅ (SPL) | ✅ |
| MEV protection | ✅ | ❌ | ❌ (MEV everywhere) | ❌ | ❌ | ❌ |
| ZK proofs | ✅ (Halo2) | ❌ | ✅ (ZK rollups) | ❌ (ZK in progress) | ❌ | ❌ |
| Rollups | ✅ (opt+ZK) | ❌ | ✅ (opt+ZK) | ❌ | ❌ | ❌ |
| Bridge code | ✅ (Eth+Cosmos) | ❌ | ✅ (many) | ✅ (XCMP) | ❌ (Wormhole) | ✅ (IBC) |
| Built-in frontend | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Interactive API docs | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Dark mode | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Single binary | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ |
| RocksDB | ✅ | ❌ (LevelDB) | ❌ (LevelDB/Pebble) | ✅ | ✅ | ✅ |
| libp2p | ✅ | ❌ | ❌ (DevP2P) | ✅ | ❌ (UDP) | ✅ |
| Docker | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Prometheus/Grafana | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |

---

## 11. Verdict

**Scratch is best for**: Learning blockchain internals, rapid prototyping, local development, education.

**Scratch is not ready for**: Production, mainnet, real value, public access, large-scale deployment.

**vs Bitcoin**: Scratch has smart contracts, faster blocks, BFT finality, and UIs — but Bitcoin has 15+ years of security, $1T+ market cap, and a global mining network.

**vs Ethereum**: Scratch has a built-in frontend, parallel execution, MEV protection, and ZK code — but Ethereum has a massive DApp ecosystem, 500K+ validators, mature tooling (MetaMask, Etherscan, Hardhat), and years of battle-testing.

**vs Polkadot**: Scratch has similar architecture (BABE+GRANDPA, libp2p, WASM) but Polkadot has parachain security, XCMP cross-chain messaging, and a live mainnet with 300+ validators.

**vs Solana**: Scratch has a built-in frontend and broader feature code — but Solana has sub-second block times, 2,000+ validators, Sealevel parallel execution at scale, and a $10B+ ecosystem.

**vs Cosmos**: Scratch has similar BFT consensus and IBC bridge code — but Cosmos has a live IBC network of 50+ connected chains, production-grade SDKs, and years of real-world operation.

---

*Comparison date: June 25, 2026*
