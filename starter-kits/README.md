# Modular Blockchain Starter Kits

Ready-to-use templates for building on Modular Blockchain.

## Available Kits

| Kit | Description | Directory |
|-----|-------------|-----------|
| **Token** | ERC-20 and ERC-721 token deployment | [`token/`](./token/) |
| **NFT Marketplace** | NFT minting, listing, buying, selling | [`nft-marketplace/`](./nft-marketplace/) |
| **DeFi** | AMM DEX with liquidity pools | [`defi/`](./defi/) |
| **DAO** | Governance with proposal/vote/execute | [`dao/`](./dao/) |

## Prerequisites

- Node.js 18+
- A running Modular Blockchain node (local or testnet)
- SDK: `npm install @modular-blockchain/sdk`

## Quick Start

```bash
# Install the CLI tool
npm install -g @modular-blockchain/cli

# Scaffold a new project from a kit
modular init my-dapp --kit token

# Deploy to local node
modular deploy --network localhost
```

## Kit Structure

Each kit contains:

```
kit-name/
├── README.md          # Kit-specific instructions
├── contracts/         # Solidity source contracts
├── scripts/           # Deployment and interaction scripts
└── test/              # Test cases
```

## Manual Use

Each kit directory contains standalone scripts you can run directly:

```bash
cd starter-kits/token
npx tsx scripts/deploy-token.ts
```

See individual kit READMEs for details.
