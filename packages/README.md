# Packages

Developer tooling — the official CLI for the Modular Blockchain.

## Package: `@modular-blockchain/cli`

The `modular` CLI provides project scaffolding and blockchain interaction commands.

### Commands

| Command | Description |
|---|---|
| `modular init` | Scaffold a new project from starter kits (token, nft-marketplace, defi, dao) |
| `modular deploy` | Deploy a pre-compiled contract (ERC20/ERC721) |
| `modular deploy-wizard` | Interactive step-by-step deployment UI |
| `modular generate` | Generate Solidity contract scaffolding (ERC20, ERC721) |
| `modular wallet` | Generate keys, query balance, send NBL tokens |
| `modular network` | Status, peer listing, faucet requests |

### Dependencies

Runtime: `chalk`, `commander`, `inquirer`, `ora` · Blockchain: `@modular-blockchain/sdk`
