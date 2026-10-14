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
| 5,000 transfers in 5.4 s (before) | 5,000/5,000 | ~240 tx/s sustained | p50 11.5 s, p95 16.4 s (queueing) |
| 5,000 transfers in 6–7 s (now) | 5,000/5,000 | ~415–445 tx/s sustained | p50 4.0–7.1 s, p95 5.8–8.2 s |

What changed between the two 5,000-transfer rows, in order of effect:

1. Per-block disk syncs: every trie node written while a block was staged
   forced an fsync of the database, although staged writes are only in
   memory; now there is one sync per block, after it is written.
2. Consensus timeouts: a timeout that expired at one height could be
   handled after the engine had moved to the next and throw away that
   height's round 0; timeouts now carry their height/round and fire once,
   and every new height schedules its own propose timeout. The prevote and
   precommit windows went from 1 s to 3 s / 2 s: a full block takes peers
   1–2.5 s to receive and check, so 1 s failed exactly the busiest rounds.
   (Runs at ~600 tx/s were measured before the propose timeout was
   scheduled per height; that version stalled for good whenever a round-0
   proposer was offline.)
3. Signature checks: each transaction's Ed25519 signature is verified once
   per node (RPC/mempool) and cached; the remaining checks in a block run in
   parallel. Proposals already validated are not re-validated when the
   proposer rebroadcasts them.
4. Gossip and sync use MessagePack instead of JSON (~1.7× smaller blocks).

A full block (~1,400 transfers) now costs ~60 ms to build, ~100 ms to
validate and ~200–300 ms to execute and commit on each node. The submitting
client (one Node.js process, ~500–950 tx/s) is now a large share of the
measured time.
