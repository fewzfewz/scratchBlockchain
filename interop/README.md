# Interop — Cross-Chain Bridge

Bridge contracts and relayer logic for Ethereum ↔ Nebula asset transfers.

> **Status (August 2026):** Bridge code in `interop/` includes Ed25519 relayer signature verification and unit tests for lock/unlock. Run `cargo test -p interop` and `node 21-bridge-lock.js` against a live testnet. Set `ETH_RPC_URL` for on-chain Hardhat tests.

## Components

| Module | Description |
|--------|-------------|
| `ethereum_bridge.rs` | Lock/unlock with multi-relayer Ed25519 verification |
| `relayer.rs` | Relayer message relay |
| `token_registry.rs` | USDC/USDT/WETH address mapping |

## Testing

```bash
cargo test -p interop
cd tests/localhost/scripts && node 21-bridge-lock.js
```

See [WHATS_LEFT.md](../WHATS_LEFT.md) for mainnet deployment status.
