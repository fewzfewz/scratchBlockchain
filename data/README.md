# Data

Persistent on-disk blockchain state (runtime data, not source code).

## Databases

| Directory | Backend | Contents |
|---|---|---|
| `state_db/` | Sled 0.34 | Account state trie |
| `block_db/` | Sled 0.34 | Block headers and bodies |
| `receipts_db/` | Sled 0.34 | Transaction execution receipts |

All databases use 512 KB segment size with compression disabled. Generated and managed by the `storage` crate at runtime.
