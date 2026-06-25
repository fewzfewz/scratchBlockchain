# Node DB

Runtime on-disk database for a single node instance (not source code).

## Files

| File | Description |
|---|---|
| `conf` | Sled 0.34 config: 512 KB segment size, compression disabled |
| `db` | Main sled database file (blocks, state, receipts) |
| `snap.*` | Crash-consistent snapshot checkpoint |

Managed at runtime by the `storage` crate. Contents vary per node instance.
