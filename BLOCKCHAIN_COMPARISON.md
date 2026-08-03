# Nebula (Scratch) vs Other Chains

Comparison of **Nebula** against Bitcoin, Ethereum, Polkadot, Solana, and Cosmos — updated August 2026 after v0.3.4 pre-mainnet work.

**Status:** PRE-MAINNET **100% code-complete** — local + Docker testnet ready; mainnet blocked only on audit, public testnet soak, and ops deploy.

---

## 1. High-Level Overview

| | **Nebula** | **Bitcoin** | **Ethereum** | **Polkadot** | **Solana** | **Cosmos** |
|---|---|---|---|---|---|---|
| **Launched** | Pre-mainnet (local + Docker) | 2009 | 2015 | 2020 | 2020 | 2019 |
| **Consensus** | BFT (2/3 finality) | PoW | PoS (Gasper) | NPoS (BABE+GRANDPA) | PoH + Tower BFT | Tendermint BFT |
| **Finality** | ~6s deterministic | ~60min probabilistic | ~12–15min probabilistic | ~12–60s deterministic | ~2s deterministic | ~7s deterministic |
| **Block time** | ~6s | ~10min | ~12s | ~6s | ~400ms | ~7s |
| **Model** | Account | UTXO | Account (EVM) | Account (WASM) | Account | Account (CosmWASM) |
| **Smart contracts** | EVM + WASM | Script only | EVM (Solidity) | WASM (ink!) | BPF (Rust/C) | WASM (CosmWASM) |
| **Single binary** | ✅ Yes | ✅ Yes | ❌ EL+CL | ❌ Relay+para | ✅ Yes | ✅ Yes |
| **Built-in frontend** | ✅ 10-route SPA | ❌ | ❌ (MetaMask ecosystem) | ❌ | ❌ | ❌ |
| **Official SDK** | ✅ `@modular-blockchain/sdk` | bitcoinjs | ethers/viem | polkadot.js | @solana/web3.js | cosmjs |
| **Docker testnet** | ✅ 5-node compose | ✅ | ✅ | ✅ | ✅ | ✅ |

---

## 2. Consensus

| | **Nebula** | **Others (typical gap)** |
|---|---|---|
| **Type** | BFT round-robin + finality gadget | Bitcoin: PoW; ETH: Gasper; Polkadot: GRANDPA; Solana: Tower; Cosmos: Tendermint |
| **Validators** | 3 genesis + **dynamic register** + **hot-reload** from state trie | Production chains: 100–2000+ live validators |
| **Slashing** | Double-sign, downtime, invalid state (tracker + RPC `/slashing/events`) | All major PoS chains have live slashing |
| **Delegation** | ✅ `POST /delegate`, staking UI | Standard on PoS chains |
| **Nebula gap vs production** | Public testnet with 10+ validators for 30+ days; third-party audit | — |

**Fixed since June 2026 doc:** static genesis-only validator set → **BFT hot-reload** after `POST /validators/register` and block finalize.

---

## 3. Execution & Smart Contracts

| | **Nebula** | **Bitcoin** | **Ethereum** | **Polkadot** | **Solana** | **Cosmos** |
|---|---|---|---|---|---|---|
| **EVM** | ✅ revm, persistent `ChainStoreEvmStore` | ❌ | ✅ | via parachains | via Neon | via Evmos |
| **WASM** | ✅ wasmtime + fuel metering + `/deploy_wasm` | ❌ | ❌ | ✅ ink! | ❌ | ✅ CosmWASM |
| **Gas** | ✅ EIP-1559 | Fee/byte | ✅ EIP-1559 | Weight | Compute units | Gas |
| **Parallel exec** | ✅ rayon (block-level) | ❌ | ❌ L1 sequential | ❌ | ✅ Sealevel | ❌ |
| **Account abstraction** | ✅ `POST /submit_user_operation` | ❌ | ✅ ERC-4337 | Native filters | Native | Native |
| **Deploy UI** | ✅ `/deploy` (ERC20/721) + SDK CLI | N/A | ✅ Remix/Hardhat | ✅ | ✅ | ✅ |
| **State** | Patricia trie + EVM RocksDB keys | UTXO | Full MPT | Blake trie | Accounts | IAVL |

**Fixed since June 2026 doc:** in-memory EVM → **persistent EVM state**; WASM scaffold → **metered deploy/call RPC**; no deploy UI → **`/deploy` page**.

**Remaining vs Ethereum mainnet:** full KZG/Halo2 ZK circuits (Merkle + SHA-256 proofs today); Sealevel-grade parallel scheduling.

---

## 4. Frontend & Developer Experience

| Feature | **Nebula** | **Typical production chain** |
|---|---|---|
| Wallet UI | ✅ Ed25519, send, **tx history** (`GET /txs/{address}`) | MetaMask / Keplr / Phantom |
| Explorer | ✅ blocks, validators, staking estimator | Etherscan / Mintscan |
| Faucet | ✅ `POST /faucet/request` (in-node) | Public testnet faucets |
| Governance UI | ✅ proposals, vote via signed txs | Tally / Polkadot.js |
| Validator onboarding | ✅ `/validators/onboard` + Grafana | Operator runbooks |
| API docs | ✅ Swagger at `/api-docs` (32 routes) | Chain-specific RPC docs |
| SDK | ✅ TypeScript `@modular-blockchain/sdk` — 40+ methods, CI + npm publish workflow | Mature npm packages |
| Integration tests | ✅ 15+ JS scripts + `run-all-tests.sh` + CI job | Chain-specific testnets |
| Dev portal | ✅ `/sdk`, `/docs`, starter kits | docs.site |

**Fixed since June 2026 doc:** no tx history → **`GET /txs/{address}`**; SDK “basic, not published” → **aligned with all RPC routes**, npm-ready.

---

## 5. Networking & Storage

| | **Nebula** | **Gap vs production** |
|---|---|---|
| **P2P** | libp2p (Kademlia, Gossipsub, Noise) | Multi-region peering at scale |
| **Database** | RocksDB + in-memory fallback | — |
| **Pruning** | ✅ archive / full / minimal + trie GC | Tune for archive nodes at mainnet scale |
| **Load testing** | ✅ `40-load-test.js`, chaos smoke | Public testnet soak 30+ days |
| **Monitoring** | ✅ Prometheus, Grafana, Alertmanager template | Prod Slack/PagerDuty webhooks |

**Fixed since June 2026 doc:** “pruning not implemented” → **full/minimal/archive + Patricia orphan GC**.

---

## 6. Advanced Features (breadth)

| Feature | **Nebula** | **Notes** |
|---|---|---|
| MEV protection | ✅ commit-reveal + threshold encryption (sharks + AES-GCM) | ETH: MEV everywhere |
| ZK | ✅ proof binding; Halo2 feature stub | ETH: live ZK rollups |
| DA | ✅ Reed–Solomon + Merkle commitments | ETH: blobs / KZG |
| Rollups | ✅ optimistic + ZK code paths; fraud re-exec | ETH: live L2s |
| Bridge | ✅ interop + Ed25519 relayers + tests | Cosmos: IBC native |
| Governance + treasury | ✅ on-chain | Standard PoS |

---

## 7. Development Status

| | **Nebula (Aug 2026)** | **Production chains** |
|---|---|---|
| **Pre-mainnet code** | ✅ **100%** | — |
| **Local testnet** | ✅ 5-node Docker | — |
| **Public testnet** | ⚠️ Scripts ready (`PUBLIC_TESTNET.md`) | Live |
| **Security audit** | ⚠️ Prep docs (`AUDIT_READINESS.md`) | Multiple audits |
| **Bug bounty** | ❌ Post-audit | Immunefi etc. |
| **SDK on npm** | ✅ CI publish workflow (`NPM_TOKEN`) | Published |

---

## 8. What Nebula Does Better (for learning & prototyping)

| Advantage | Details |
|---|---|
| **All-in-one repo** | Node + frontend + SDK + tests + Docker in one place |
| **Single binary** | `./target/debug/node start` — no EL/CL split |
| **Built-in SPA** | Wallet, explorer, faucet, governance, deploy, API docs — no MetaMask required |
| **Interactive API docs** | Swagger UI wired to live node |
| **Breadth** | EVM, WASM, ZK, MEV, DA, rollups, bridge, governance in one codebase |
| **Fast local start** | `./start.sh` or `docker compose up` |

---

## 9. What Still Blocks Mainnet (not pre-mainnet code)

These are **operational / economic** blockers — not missing local features:

| Blocker | Status |
|---|---|
| Public testnet deployed 30+ days | Scripts in `deployment/cloud/` |
| Professional security audit | `AUDIT_READINESS.md` ready |
| Bug bounty program | Post-audit |
| Full KZG/Halo2 ZK | Merkle + SHA-256 today |
| Production alerting (Slack/PagerDuty) | Template in `deployment/cloud/configs/` |
| Mainnet genesis ceremony | `tools/genesis-builder/examples/mainnet.toml` |
| Multi-region RPC + DDoS | Terraform/Ansible scaffold |

---

## 10. Feature Matrix

| Feature | Nebula | Bitcoin | Ethereum | Polkadot | Solana | Cosmos |
|---|:---:|:---:|:---:|:---:|:---:|:---:|
| BFT / PoS finality | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| EVM | ✅ | ❌ | ✅ | ◐ | ◐ | ◐ |
| WASM contracts | ✅ | ❌ | ❌ | ✅ | ❌ | ✅ |
| EIP-1559 | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ |
| Built-in wallet UI | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| On-chain governance | ✅ | ❌ | ✅ | ✅ | ◐ | ✅ |
| MEV protection (code) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Staking + delegation | ✅ | ◐ | ✅ | ✅ | ✅ | ✅ |
| State pruning | ✅ | ◐ | ✅ | ✅ | ✅ | ✅ |
| libp2p | ✅ | ❌ | ❌ | ✅ | ❌ | ✅ |
| Public mainnet | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |

◐ = partial / via extension / parachain

---

## 11. Verdict

**Nebula is best for:** learning blockchain internals, local development, rapid prototyping, and running a full stack (node + UI + SDK) on one machine.

**Nebula pre-mainnet is complete:** all planned local features, SDK alignment, tests, and docs are done.

**Nebula is not yet for:** holding real value, public mainnet, or competing with mature ecosystems on validator count, audit history, or sub-second blocks.

**vs Ethereum:** Nebula wins on built-in UX and feature breadth in one repo; Ethereum wins on ecosystem size, audits, and production scale.

**vs Cosmos/Polkadot:** Similar BFT + WASM ideas; they win on live cross-chain (IBC/XCMP) and years of mainnet operation.

---

*Comparison updated: August 3, 2026 (v0.3.4)*
