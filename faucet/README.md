# Nebula Faucet

Test token faucet for the Scratch Blockchain.

## Primary: Built-in Node Faucet (recommended)

The node credits tokens directly via RPC — no separate service required:

```bash
curl -X POST http://localhost:8545/faucet/request \
  -H "Content-Type: application/json" \
  -d '{"address":"0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18","amount":100}'
```

- **Server-side cooldown:** 60 seconds per address (enforced in `node/src/rpc.rs`)
- **Frontend:** `/faucet` page in the unified SPA (`http://localhost:5173/faucet`)

## Legacy: Standalone Faucet Crate

This directory also contains a standalone warp server used by `node faucet` CLI subcommand:

```bash
cargo run --release --bin node faucet
# Listens on http://localhost:3006/faucet (env: FAUCET_PORT)
```

Prefer the built-in `POST /faucet/request` on port **8545** for local development and Docker testnet.
