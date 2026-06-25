# Storage

Persistent and in-memory storage layer for blockchain data.

## Components

| Module | Description |
|---|---|
| `src/db.rs` | `KeyValueStore` trait, `MemDb` (in-memory, 10K LRU cache), `RocksDb` (production — LZ4+Zstd compression, Bloom filters, column families), `ChainStore` (type-safe block/state/receipt storage with atomic `commit_block`), `WriteBatch` |
| `src/trie.rs` | Full Patricia Merkle Trie with `get`/`insert`/`delete`, Merkle proof generation + verification, node caching |
| `src/receipt_store.rs` | Bincode-serialized receipt persistence |
| `src/lib.rs` | `MemStore`, `StateStore`, `TrieStateStore`, optional `PersistentStore` (sled) / `BlockStore` (sled) |

## Column Families

| Prefix | Name | Contents |
|---|---|---|
| `0x01` | `Blocks` | Block data by hash |
| `0x02` | `BlockHeights` | Block hash by height |
| `0x03` | `State` | Account state key-value pairs |
| `0x04` | `Receipts` | Transaction receipts |
| `0x05` | `Meta` | Chain metadata (latest height, genesis hash, chain ID) |

## Features

- `rocksdb` (default) — RocksDB backend with configurable compression and compaction
- `sled-legacy` — Alternative sled-backed store
