# Config

Node configuration files in TOML format.

## Files

- `testnet.toml` — 3-validator testnet configuration with `[network]`, `[consensus]`, `[validator]`, `[storage]`, `[api]`, `[metrics]` sections.

See `node/src/config.rs` for the `NodeConfig` struct that deserializes this file.
