# Modular Blockchain JavaScript/TypeScript SDK

Official TypeScript/JavaScript SDK for interacting with the Modular Blockchain — Ed25519 signing, EIP-1559 gas, HTTP/WS providers, and full type support.

## Features
- **Wallet Management** — Ed25519 key generation, BIP39 mnemonics, address derivation (SHA-256)
- **Transaction Building** — Signing matched to Rust node's `Transaction::hash()` binary format
- **Providers** — HTTP (REST) and WebSocket (JSON-RPC with polling fallback)
- **Real-time Events** — Subscribe to new blocks, transactions via WS or polling
- **Full TypeScript** — Complete type definitions for all operations

## Installation
```bash
npm install @modular-blockchain/sdk
```

## Quick Start
```ts
import { ModularClient, HttpProvider, Wallet } from "@modular-blockchain/sdk";

const provider = new HttpProvider("http://localhost:9933");
const client = new ModularClient(provider);

const status = await client.getNodeStatus();
console.log("Height:", status.height);

const wallet = Wallet.generate();
const tx = await wallet.signTransaction({ to: "0x...", value: "1000000000000000000" });
const receipt = await client.sendTransaction(tx);
console.log("Tx:", receipt.hash);
```

## CLI
```bash
npx @modular-blockchain/sdk-cli wallet --generate
npx @modular-blockchain/sdk-cli init --template defi --name my-app
npx @modular-blockchain/sdk-cli deploy --contract ERC20 --args "Token,TOK,1000000"
```

## Smart Contract Templates
Solidity contracts in `contracts/`:
| Template | Description |
|----------|-------------|
| `ERC20.sol` | Fungible token with mint/burn/approve |
| `ERC721.sol` | NFT with safe transfers and approvals |
| `DAO.sol` | Governance with propose/vote/execute |

## Starter Kits
Complete examples in `examples/starter-kits/`:
- `defi.ts` — AMM liquidity pool and swap flow
- `nft.ts` — NFT mint, approve, transfer marketplace flow
- `dao.ts` — DAO governance proposal and voting workflow

## API Reference

### ModularClient
| Method | Description |
|--------|-------------|
| `connect()` | Verify connection and chain ID |
| `getBlockNumber()` | Get latest block height |
| `getBlock(height)` | Get block by number |
| `getBalance(address)` | Get native balance |
| `getNonce(address)` | Get account nonce |
| `sendTransaction(tx)` | Submit signed transaction |
| `getTransactionReceipt(hash)` | Get receipt by tx hash |
| `waitForTransaction(hash)` | Poll until confirmed |
| `getGasPrice()` | Get EIP-1559 gas prices |
| `estimateGas(tx)` | Estimate gas cost |
| `getMempool()` | List pending transactions |
| `getNodeStatus()` | Get node status |

### Wallet
| Method | Description |
|--------|-------------|
| `Wallet.generate()` | Generate random Ed25519 keypair |
| `Wallet.fromPrivateKey(hex)` | Import from private key |
| `Wallet.fromMnemonic(phrase)` | Restore from mnemonic |
| `signTransaction(tx)` | Sign (matches Rust hash format) |
| `signMessage(msg)` | Sign arbitrary message |
| `verifySignature(msg, sig, pubKey)` | Verify Ed25519 signature |

### Providers
```ts
// HTTP (REST) — GET for queries, POST for mutations
const http = new HttpProvider("http://localhost:9933");

// WebSocket — JSON-RPC with auto-reconnect and polling fallback
const ws = new WebSocketProvider("ws://localhost:9934");
ws.subscribe("newBlocks", (block) => console.log(block));
```

## Events
```ts
client.on("connected", (chainId) => {});
client.on("transactionConfirmed", (receipt) => {});

ws.subscribe("newBlocks", (block) => {});
ws.subscribe("newTransactions", (tx) => {});
```

## RPC Endpoints
| Method | Path | Description |
|--------|------|-------------|
| GET | `/status` | Node status |
| GET | `/mempool` | Pending transactions |
| POST | `/submit_tx` | Submit transaction |
| GET | `/block/{height}` | Block by height |
| GET | `/block/hash/{hash}` | Block by hash |
| GET | `/balance/{address}` | Account balance+nonce |
| GET | `/tx/{hash}` | Transaction receipt |
| GET | `/gas_price` | EIP-1559 gas prices |
| POST | `/estimate_gas` | Estimate gas |
| GET | `/fee_history/{count}` | Historical fees |
| GET | `/health` | Health check |
| GET | `/peers` | Peer list |
| POST | `/connect_peer` | Connect to peer |

## Developer Portal
Open `sdk/portal/index.html` for a full developer portal with quick start guides, CLI docs, contract templates, and RPC API reference.

## License
MIT
