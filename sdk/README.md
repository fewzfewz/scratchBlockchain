# SDK

TypeScript/JavaScript SDK (`@modular-blockchain/sdk`) for interacting with the Modular Blockchain.

## Structure

| Path | Description |
|---|---|
| `javascript/` | SDK source code and build output |
| `portal/` | Standalone HTML developer portal |

## SDK Capabilities

- **Wallet** — Ed25519 key generation, BIP39 mnemonics, address derivation, transaction signing, message signing/verification
- **ModularClient** — 30+ RPC methods: blocks, transactions, accounts, gas (EIP-1559), mempool, peers, governance, health
- **Providers** — `HttpProvider` (REST), `WebSocketProvider` (JSON-RPC with auto-reconnect and polling fallback)
- **Events** — `connected`, `disconnected`, `transactionConfirmed`, `transactionPending`
- **Types** — Full TypeScript definitions for all request/response types including governance

## Dependencies

`@noble/curves` (Ed25519), `@noble/hashes` (SHA-256), `axios`, `bip39`, `eventemitter3`, `ws`
