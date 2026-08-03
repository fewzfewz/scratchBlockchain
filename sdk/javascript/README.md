# Modular Blockchain JavaScript/TypeScript SDK

Official TypeScript/JavaScript SDK for Nebula — Ed25519 signing, EIP-1559 gas, full RPC coverage, HTTP/WebSocket providers.

## Features
- **Wallet** — Ed25519 keys, BIP39 mnemonics, transaction signing (matches Rust `Transaction::hash()`)
- **ModularClient** — 40+ methods covering all node RPC routes
- **Providers** — HTTP (REST) and WebSocket (`/ws` newHead)
- **Governance, staking, WASM, MEV, AA** — first-class API methods
- **TypeScript** — full type definitions

## Installation
```bash
npm install @modular-blockchain/sdk
# or from repo:
cd sdk/javascript && npm install && npm run build
```

## Quick Start
```ts
import { ModularClient, HttpProvider, Wallet } from "@modular-blockchain/sdk";

const client = new ModularClient(new HttpProvider("http://localhost:8545"));
await client.connect();

const status = await client.getNodeStatus();
console.log("Height:", status.height, "Chain:", status.chain_id);

const wallet = Wallet.generate();
const tx = await wallet.signTransaction({
  to: "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18",
  value: "1000000000000000000",
});
const result = await client.sendTransaction(tx);
console.log("Tx hash:", result.hash);

// Tx history
const history = await client.getTxHistory(wallet.address, 20);

// WASM
await client.deployWasm("counter", base64Wasm);
const out = await client.callWasm("counter", "increment", 1);
```

## CLI
```bash
npx @modular-blockchain/sdk-cli wallet --generate
npx @modular-blockchain/sdk-cli deploy --contract ERC20 --args "Token,TOK,1000000"
```

## ModularClient API (selected)

| Category | Methods |
|----------|---------|
| **Chain** | `getChainId()`, `getBlockNumber()`, `getBlock()`, `getLatestBlock()`, `getNodeStatus()`, `healthCheck()` |
| **Account** | `getBalance()`, `getAccount()`, `getNonce()`, `getTxHistory()` |
| **Tx** | `sendTransaction()`, `getTransactionReceipt()`, `waitForTransaction()` |
| **Gas** | `getGasPrice()`, `estimateGas()`, `getFeeHistory()` |
| **Staking** | `getValidators()`, `delegate()`, `registerValidator()`, `getDelegations()` |
| **Governance** | `getProposals()`, `getProposal()`, `getTreasury()`, `getGovParams()` |
| **Faucet** | `requestFaucet()` |
| **WASM** | `deployWasm()`, `callWasm()`, `listWasmContracts()` |
| **MEV / AA** | `submitUserOperation()`, `getPendingUserOperations()` |
| **Network** | `getPeers()`, `connectPeer()`, `getMempool()`, `getMetrics()` |

> Governance **write** actions (propose, vote, execute) use signed `POST /submit_tx` with governance payloads — use `Wallet.signTransaction` + governance extrinsics from the frontend or integration test helpers.

## RPC route mapping

The SDK `HttpProvider` maps logical methods to node paths:

| SDK method | HTTP |
|------------|------|
| `getNodeStatus` | `GET /status` |
| `getTxHistory` | `GET /txs/{address}?limit=N` |
| `getProposals` | `GET /governance` |
| `getProposal` | `GET /proposal/{id}` |
| `delegate` | `POST /delegate` |
| `registerValidator` | `POST /validators/register` |
| `requestFaucet` | `POST /faucet/request` |
| `deployWasm` | `POST /deploy_wasm` |
| `callWasm` | `POST /call_wasm` |
| `submitUserOperation` | `POST /submit_user_operation` |

Full spec: `docs/openapi.yaml` and `/api-docs` in the frontend.

## Development
```bash
cd sdk/javascript
npm install
npm run build
npm test
npm run lint
```

CI: `.github/workflows/sdk-ci.yml` (build + test; npm publish on main with `NPM_TOKEN`).

## License
MIT
