# SDK

TypeScript/JavaScript SDK (`@modular-blockchain/sdk`) for Nebula.

## Structure

| Path | Description |
|---|---|
| `javascript/` | SDK source, build output, CLI |
| `javascript/cli/` | `modular` CLI (wallet, deploy, scaffold) |
| `javascript/contracts/` | ERC20, ERC721, DAO Solidity templates |
| `javascript/examples/` | Starter kits (DeFi, NFT, DAO) |

## Capabilities (v0.3.4)

- **Wallet** — Ed25519, BIP39, signing matched to Rust node
- **ModularClient** — 40+ methods; all 32 HTTP RPC routes covered
- **Providers** — `HttpProvider`, `WebSocketProvider`
- **Events** — `connected`, `transactionConfirmed`, `transactionPending`

## Dependencies

`@noble/curves`, `@noble/hashes`, `axios`, `bip39`, `eventemitter3`, `ws`

## Publish

```bash
cd sdk/javascript
npm run build && npm test
npm publish --access public  # requires NPM_TOKEN in CI
```

See `javascript/README.md` for API reference.
