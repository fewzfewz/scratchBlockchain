# Node

Main blockchain node binary and library — ties together all subsystems.

## Entry Point

`src/main.rs` — CLI (clap) with subcommands: `start`, `keygen`, `submit-tx`, `query-balance`, `get-block`, `connect-peer`, `faucet`, `status`.

## Key Modules

| Module | Description |
|---|---|
| `src/config.rs` | `NodeConfig` with `Network`, `Consensus`, `Validator`, `Storage`, `Api`, `Metrics`, `Logging`, `Security` configs (TOML + env var overrides) |
| `src/tx_pool.rs` | **`TxPool`** — unified pool: `MevMempool` + `AccountAbstractionExecutor`; used by RPC and block producer |
| `src/rpc.rs` | Warp RPC server — **29 endpoints** (28 HTTP + WebSocket `/ws`). Rate limiting via `governor` (configurable, default 200 req/s) |
| `src/governance_store.rs` | On-chain governance trie wiring, delegation, validator registration, treasury fee collection |
| `src/block_producer.rs` | `BlockProducer` — pulls txs from `TxPool`, deterministic state root, signs blocks. `BlockExecutor` — EVM + atomic commit |
| `src/fork_choice.rs` | Longest finalized-chain fork choice with reorg support |
| `src/rewards.rs` | `RewardManager` — proposer rewards, vote rewards, fee distribution, slashing penalties |
| `src/metrics.rs` | Prometheus counters for blocks, tx, mempool, peers, finalized height, consensus round |
| `src/circuit_breaker.rs` | Closed → Open → HalfOpen circuit breaker |
| `src/faucet.rs` | Standalone faucet crate logic (testnet drip, cooldown) |
| `src/light_client.rs` | Sync committee header verification, finality proofs |
| `src/runtime_upgrade.rs` | `RuntimeUpgradeManager` — proposal/approval/activation/rollback |
| `src/test_utils.rs` | `TestNode` builder + mock factories |

## RPC Endpoints (summary)

See [README.md](../README.md#rpc-api) or [STATUS.md](../STATUS.md) for the full list. Highlights:

- **Core**: `/status`, `/submit_tx`, `/block/*`, `/balance/*`, `/faucet/request`
- **AA**: `/submit_user_operation`, `/user_operations/pending`
- **MEV**: `/mev/commit`, `/mev/reveal`, `/mev/encrypted`, `/mev/decryption_share`
- **Staking**: `/delegate`, `/validators/register`, `/delegations/{address}`, `/slashing/events`
- **Real-time**: `/ws` (WebSocket)
- **Governance**: `/governance`, `/proposal/{id}`

## Integration Notes

- Block producer uses `TxPool::get_transactions_for_block()` (AA bundles → MEV-ready → regular mempool)
- `SlashingTracker` advances on each finalized block; slashed keys exposed via RPC
- BFT height re-anchors every 10s if drifted from chain tip
- State root in headers: `SHA256(parent_state_root || extrinsics_root)`
