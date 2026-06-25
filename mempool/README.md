# Mempool

Pending transaction pool with priority ordering, per-sender nonce enforcement, and MEV protection.

## Components

| Module | Description |
|---|---|
| `src/lib.rs` | `Mempool` — `BinaryHeap`-based priority queue (highest fee first, FIFO tiebreaker), 10K default capacity, evicts lowest-fee at capacity |
| `src/mev_protection.rs` | `MevProtection` — commit-reveal scheme to prevent front-running; transactions commit by hash, reveal after minimum block delay |
| `MevMempool` | Wraps standard mempool with commit-reveal + threshold-encrypted mempool from the `mev` crate |
