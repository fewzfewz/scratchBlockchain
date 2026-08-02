# Mempool

Pending transaction pool with priority ordering, per-sender nonce enforcement, and MEV protection.

## Components

| Module | Description |
|---|---|
| `src/lib.rs` | `Mempool` — `BinaryHeap`-based priority queue (highest fee first, FIFO tiebreaker), 10K default capacity |
| `src/mev_protection.rs` | `MevProtection` — commit-reveal scheme helpers |
| `MevMempool` | Wraps standard mempool with commit-reveal + threshold encryption from the `mev` crate |

## Node Integration

The node does **not** use bare `Mempool` directly. It uses **`TxPool`** (`node/src/tx_pool.rs`), which wraps:

- `MevMempool` — regular txs + commit/reveal + encrypted txs
- `AccountAbstractionExecutor` — ERC-4337 UserOperation bundler

Block production calls `MevMempool::get_all_ready_transactions()` after draining AA bundles.

## RPC Routes (via node)

- `POST /submit_tx` → `TxPool::add_transaction`
- `POST /mev/commit`, `/mev/reveal`, `/mev/encrypted`, `/mev/decryption_share`
- `POST /submit_user_operation` → AA bundler (separate from Mempool struct)
