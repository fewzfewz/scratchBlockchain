# Benchmarks

Criterion-based performance benchmarks for core blockchain subsystems.

## Benchmarks

| Target | File | Measures |
|---|---|---|
| `consensus_bench` | `benches/consensus_bench.rs` | Ed25519 header signature verification throughput |
| `mempool_bench` | `benches/mempool_bench.rs` | Mempool `add_transaction` latency at 10K capacity |
| `storage_bench` | `benches/storage_bench.rs` | Sled/RocksDB write latency with 100-byte values |
| `throughput` | `benches/throughput.rs` | Transaction execution (10–500), block production, mempool operations |

## Usage

```bash
cargo bench -p benchmarks
```

All benchmarks use `criterion` 0.5 with `harness = false`.
