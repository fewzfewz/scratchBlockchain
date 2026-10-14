# Benchmarks

Criterion benchmarks of the node's real hot paths (`benches/node_paths.rs`):

| Group | Measures |
|---|---|
| `execute_block/{100,1000,5000}` | A block of signed native transfers through `BlockExecutor::execute_and_commit` on the on-disk store, staged as the node stages a block, plus the LtHash over the changed state. Reported in transactions/second. |
| `mempool_admit/1000` | `TxPool::add_transaction`, including signature verification. |
| `commit_certificate_verify/{4,21,100}` | Checking a block's +2/3 precommits (sync, proposal validation, light client). |
| `bft_height/{4,10,31}` | CPU cost of one BFT height across N in-process validators: a lower bound on finality latency before network delay. |

```bash
cargo bench -p benchmarks
```

End-to-end throughput and finality latency on the running devnet:
`tests/localhost/scripts/42-throughput.js` (see the script header for knobs).

Reference numbers (one laptop core, release build, October 2026):

| Benchmark | Result |
|---|---|
| execute_block/1000 | ~113 ms (~8.9k tx/s) |
| execute_block/5000 | ~670 ms (~7.5k tx/s) |
| mempool_admit/1000 | ~50 ms (~20k tx/s) |
| commit_certificate_verify/4 · /100 | ~0.2 ms · ~5.2 ms |
| bft_height/4 · /31 | ~1.5 ms · ~81 ms |

Devnet end to end (4 validators + 2 RPC nodes in Docker on one laptop,
`42-throughput.js`, October 2026):

| Load | Included | Throughput | Submit → finalized |
|---|---|---|---|
| 500 transfers in 0.8 s | 500/500 | ~340 tx/s | p50 0.7 s, p95 0.9 s |
| 5,000 transfers in 5.4 s | 5,000/5,000 | ~240 tx/s sustained | p50 11.5 s, p95 16.4 s (queueing) |

Under sustained load blocks fill to the gas limit (~1,400 transfers) and
each full block takes a few seconds end to end: JSON encoding, gossip, and
every validator re-verifying every signature. Those are the next things to
optimize (binary encoding, parallel or cached signature checks).
