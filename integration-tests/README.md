# Integration Tests

End-to-end tests exercising multiple blockchain subsystems together.

## Test Files

| File | Description |
|---|---|
| `tests/integration_test.rs` | Storage+consensus, governance lifecycle, rollup batch, multi-component flow, persistence |
| `tests/consensus_integration.rs` | Multi-validator consensus (4 validators), finality voting |
| `tests/multi_node_consensus.rs` | 3 nodes via libp2p, tx propagation, network partition recovery |
| `tests/mempool_integration.rs` | Mempool capacity limits (10 tx max) |
| `tests/network_integration.rs` | P2P tx propagation, block production |
| `tests/storage_integration.rs` | Persistent store survives restart |
| `tests/sync_test.rs` | Block sync protocol codec |
| `tests/load_test.rs` | 3 nodes, 100 tx, TPS measurement |
| `tests/chaos_test.rs` | 5 nodes, random disconnects, recovery verification |

## Usage

```bash
cargo test -p integration-tests --test <test_name>
```
