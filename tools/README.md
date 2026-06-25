# Tools

CLI tools for blockchain infrastructure.

## Genesis Builder

`genesis-builder/` — generates `genesis.json` from TOML configuration.

### Subcommands

| Command | Description |
|---|---|
| `generate` | Generate genesis from a TOML config file |
| `validate` | Validate an existing genesis JSON |
| `example` | Print example TOML config |
| `show` | Pretty-print a genesis file |

### Example Configs

| File | Network |
|---|---|
| `examples/devnet.toml` | 3 validators, 3s block time, all precompiles + BLS, 10M+ accounts |
| `examples/testnet.toml` | 200 validators, 4s block time, 25% quorum |
| `examples/mainnet.toml` | 300 validators, 6s block time, 66.67% approval, 2.1B supply |

```bash
cargo run -p genesis-builder -- generate --config examples/testnet.toml --output genesis.json
```
