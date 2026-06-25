# Node

Main blockchain node binary and library — ties together all subsystems.

## Entry Point

`src/main.rs` — CLI (clap) with subcommands: `start`, `keygen`, `submit-tx`, `query-balance`, `get-block`, `connect-peer`, `faucet`, `status`.

## Key Modules

| Module | Description |
|---|---|
| `src/config.rs` | `NodeConfig` with `Network`, `Consensus`, `Validator`, `Storage`, `Api`, `Metrics`, `Logging`, `Security` configs (TOML + env var overrides) |
| `src/rpc.rs` | Warp-based RPC server: `/health`, `/status`, `/submit_tx`, `/block/{height}`, `/balance/{address}`, `/tx/{hash}`, `/gas_price`, `/mempool`, `/peers`, `/metrics`. Per-IP rate limiting (100 req/s) via `governor` |
| `src/block_producer.rs` | `BlockProducer` — pulls tx from mempool, builds/signs blocks, Merkle extrinsics root. `BlockExecutor` — EVM execution + atomic commit |
| `src/fork_choice.rs` | Longest finalized-chain fork choice with reorg support |
| `src/rewards.rs` | `RewardManager` — proposer rewards, vote rewards, fee distribution (20% proposer, rest burned), slashing (5% stake) |
| `src/metrics.rs` | Prometheus counters for blocks, tx, mempool, peers, finalized height, MEV, consensus round, latency |
| `src/circuit_breaker.rs` | Standard Closed → Open → HalfOpen circuit breaker for emergency halt + auto-recovery |
| `src/faucet.rs` | Testnet token faucet: configurable drip, 24h cooldown, max 10 requests/address |
| `src/light_client.rs` | Sync committee-based header verification, finality proofs, state proofs |
| `src/runtime_upgrade.rs` | `RuntimeUpgradeManager` — proposal/approval/activation/rollback |
| `src/test_utils.rs` | `TestNode` builder + mock factories for integration testing |
