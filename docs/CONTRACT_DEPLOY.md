# Contract Deploy — How It Works

The **Deploy Contract** page (`/deploy`) submits a **contract-creation transaction** to the Nebula node. This guide explains the flow, what “already in mempool” means, and how to deploy again safely.

## Overview

```
Wallet (browser)  →  POST /submit_tx  →  Mempool  →  Block  →  EVM execution  →  Receipt
```

1. You pick a template (ERC20 / ERC721) or paste custom **init bytecode**.
2. The page builds a signed transaction from your wallet keys (same keys as `/wallet`).
3. The transaction is submitted to the node RPC (`http://localhost:8545` by default).
4. Validators include it in a block and the **EVM** runs the bytecode as a **contract creation** (`to` is omitted).
5. On success, the receipt contains the new **contract address**.

## Transaction shape

Contract deploy uses a Nebula `Transaction` with:

| Field | Deploy value |
|-------|----------------|
| `sender` | Your 20-byte wallet address |
| `to` | **Omitted** — signals contract creation |
| `nonce` | Current account nonce from `GET /balance/{address}` |
| `value` | `0` |
| `payload` | `[32-byte Ed25519 public key] + [contract init bytecode]` |
| `signature` | Ed25519 signature over the transaction hash |
| `gas_limit` | From UI (default 500,000; use **Estimate** to refine) |

The public key prefix is required so the node can verify your Ed25519 signature before EVM execution.

### Hash & signing

The transaction hash matches the Rust node (`Transaction::hash()`):

```
SHA256(
  sender(20) + nonce(8 LE) + payload + gas_limit(8 LE)
  + max_fee(8 LE) + max_priority_fee(8 LE) + chain_id(8 LE)
  + value(8 LE)
)
```

Note: **`to` is not included** in the hash when absent (contract creation).

## UI flow

1. **Wallet required** — create one on `/wallet` and fund it via `/faucet`.
2. Choose **Template** or **Custom bytecode**.
3. Set **Gas limit** (or click **Estimate** → `POST /estimate_gas`).
4. Click **Deploy Contract** → `POST /submit_tx`.
5. The page polls `GET /tx/{hash}` until a receipt appears.
6. On success, the contract address is saved to browser storage (`nebula_deployed_contracts`) for use on `/contracts`.

## “Transaction already in mempool”

This is **not a bug** — it means you submitted the **exact same signed transaction** while the first copy is still waiting in the mempool.

Identical inputs produce an identical signature:

- Same wallet + same nonce + same bytecode + same gas → **same tx** → duplicate rejected.

### What to do

| Situation | Action |
|-----------|--------|
| Clicked **Deploy** twice quickly | Wait — the first tx is already pending. The UI treats this as “already pending” and keeps polling. |
| First deploy still confirming | Wait 10–30 seconds for a block, then check the receipt on the page. |
| Want to deploy **again** (second contract) | Wait until the first tx **confirms** — your **nonce increments by 1**, then deploy again. |
| Stuck / old pending tx | Refresh nonce from the node; do not spam Deploy with the same nonce. |

You **cannot** deploy two different contracts with the **same nonce**. Each successful deploy consumes one nonce.

## Redeploying the “same” contract

Deploying the **same bytecode** again is fine **after** the previous transaction confirms:

1. Nonce increases (e.g. 0 → 1).
2. New transaction → new signature → accepted by mempool.
3. A **new** contract address is created (EVM `CREATE`).

Each deploy is a separate on-chain contract instance.

## API reference

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/balance/{address}` | Balance + **nonce** for signing |
| `POST` | `/estimate_gas` | Suggest gas for bytecode |
| `POST` | `/submit_tx` | Submit signed transaction |
| `GET` | `/tx/{hash}` | Receipt (`success`, `created_address`) |

### Example: submit response

```json
{ "status": "success", "hash": "ebfecf3e6ac424de..." }
```

Duplicate (same signature still in mempool):

```json
{ "status": "error: Transaction already in mempool", "hash": "ebfecf3e..." }
```

The hash is still returned so the UI can track the pending deploy.

## Preset bytecode

The ERC20 and ERC721 templates ship **minimal init bytecode** for local testing. They are not full OpenZeppelin deployments — for production contracts, compile with Hardhat/Foundry and paste the hex under **Custom bytecode**.

## Troubleshooting

| Symptom | Likely cause |
|---------|----------------|
| `Deploy failed: Transaction already in mempool` | Duplicate click or retry before nonce advances — wait and poll. |
| `Create or import a wallet first` | No keys in localStorage — use `/wallet`. |
| Insufficient balance | Fund via `/faucet` (needs NBL for gas). |
| Receipt `success: false` | EVM revert (bad bytecode, out of gas, or insufficient EVM balance) — raise gas or fix bytecode. |
| No contract address in receipt | Tx failed or receipt not indexed yet — wait and refresh. |

## Related pages

- **Wallet** (`/wallet`) — keys, sends, balance
- **Faucet** (`/faucet`) — test NBL
- **Contracts** (`/contracts`) — interact with deployed addresses (see below)
- **History** (`/history`) — on-chain activity

---

# Contract Interaction — `/contracts`

After you deploy at **`/deploy`**, use **`http://localhost:5173/contracts`** to **read** and **write** deployed contracts without leaving the browser.

## End-to-end flow

```
/deploy  →  receipt with contract address  →  saved locally  →  /contracts  →  read or write
```

1. Deploy a contract on `/deploy` (ERC20, ERC721, or custom bytecode).
2. On success, the address is stored in browser `localStorage` under `nebula_deployed_contracts`.
3. Open `/contracts` — your deployed contracts appear as quick-select chips at the top.
4. Pick a contract (or paste any `0x…` address manually).
5. Use **Read**, **Estimate**, or **Write** depending on what you need.

The contracts page also **scans your wallet’s on-chain history** (`GET /txs/{address}`) for contract-creation transactions and auto-discovers addresses from receipts, even if you didn’t deploy from this browser session.

## Page layout

| Field | Purpose |
|-------|---------|
| **Contract address** | Target contract (`0x` + 40 hex chars) |
| **From (caller)** | Address the EVM simulates as `msg.sender` — defaults to your wallet |
| **Calldata (hex)** | ABI-encoded function call (function selector + arguments) |
| **Value (wei)** | NBL attached to the call (usually `0` for token reads) |

### Quick presets

The UI includes buttons for common **view** calls:

| Button | Selector | What it does |
|--------|----------|--------------|
| `balanceOf` | `0x70a08231` | ERC20 balance of the **From** address |
| `totalSupply` | `0x18160ddd` | ERC20 total supply |

Calldata is standard Ethereum ABI encoding: **4-byte selector** + **32-byte padded arguments**.

Example `balanceOf(0x742d35…)` calldata:

```
0x70a08231 + 000…000742d35Cc6634C0532925a3b844Bc9e7595f2bD18
```

## Read vs Write vs Estimate

### Read (call) — no transaction, no gas spent

- Button: **Read (call)**
- API: `POST /call_contract`
- Runs an **EVM static call** (`eth_call` style) — state is **not** changed
- Returns hex result; the UI decodes it as `uint256` when possible

```json
POST /call_contract
{
  "from": "0xYourAddress…",
  "to": "0xContractAddress…",
  "data": "0x70a08231…",
  "value": "0"
}
```

Response:

```json
{ "success": true, "result": "0x000…amount" }
```

Use **Read** to check balances, `totalSupply`, `ownerOf`, etc. before sending a write.

### Estimate — predict gas for a write

- Button: **Estimate**
- API: `POST /estimate_gas`
- Returns suggested `estimated_gas` and `total_cost_estimate`
- Does not submit a transaction

### Write — state-changing transaction

- Button: **Write**
- Requires a wallet on `/wallet` (private key in browser)
- Builds a signed Nebula transaction:
  - `to` = contract address
  - `payload` = `[32-byte public key] + [calldata hex]`
  - `value` = optional wei sent with the call
- Submits via `POST /submit_tx`, then polls `GET /tx/{hash}` for the receipt

Write calls consume **nonce + gas** like any other transaction. Wait for confirmation before sending another write with the same wallet.

## Where contract addresses come from

The **Deployed contracts** chip list is built from two sources:

1. **Local saves** — entries written by `/deploy` via `saveContract()` (`nebula_deployed_contracts` in localStorage).
2. **On-chain scan** — `scanDeployedContracts()` loads your wallet’s tx history, finds `is_contract_creation` entries, and reads `contract_address` / `created_address` from receipts.

Click a chip to fill the **Contract address** field.

## Typical workflow (ERC20 preset)

1. `/wallet` — generate wallet  
2. `/faucet` — fund with test NBL  
3. `/deploy` — deploy **ERC20 Token** template, wait for contract address  
4. `/contracts` — select the new contract chip  
5. Click **balanceOf** preset → **Read (call)** — see your token balance (may be `0` on a fresh deploy)  
6. For a state change (e.g. `transfer`), build calldata (or use helpers in `chain.js` like `encodeTransfer`) → **Estimate** → **Write**

## Calldata helpers (for developers)

`frontend/src/lib/chain.js` exports encoding helpers used by the contracts page:

| Function | Use |
|----------|-----|
| `encodeBalanceOf(holder)` | ERC20 `balanceOf` |
| `encodeTransfer(to, amountWei)` | ERC20 `transfer` |
| `encodeMint(to, tokenId)` | ERC721 `mint` |
| `encodeOwnerOf(tokenId)` | ERC721 `ownerOf` |

Selectors live in `SELECTORS` (`0x70a08231`, `0xa9059cbb`, etc.).

## API summary (`/contracts` actions)

| Action | Method | Path |
|--------|--------|------|
| Read (static call) | `POST` | `/call_contract` |
| Estimate gas | `POST` | `/estimate_gas` |
| Write (submit tx) | `POST` | `/submit_tx` |
| Tx receipt | `GET` | `/tx/{hash}` |
| Discover deploys | `GET` | `/txs/{walletAddress}` |

## Troubleshooting (`/contracts`)

| Symptom | Likely cause |
|---------|----------------|
| No contracts in chip list | Nothing deployed yet, or deploy failed — try `/deploy` first |
| Read returns `success: false` | Wrong calldata, wrong contract type, or EVM revert |
| Write fails / receipt `success: false` | Insufficient balance, bad calldata, or nonce issue |
| `Create a wallet on /wallet first` | Write requires a local wallet keypair |
| balanceOf shows 0 | Token minted supply may be 0 on minimal preset bytecode — expected for demo templates |

## Deploy + interact diagram

```
┌─────────────┐     deploy tx      ┌──────────────┐     EVM CREATE     ┌────────────────┐
│  /deploy    │ ─────────────────► │   Mempool    │ ─────────────────► │ Contract at    │
│  (bytecode) │                    │  + block     │                    │ 0xABC…         │
└─────────────┘                    └──────────────┘                    └────────┬───────┘
       │ save address to localStorage                                          │
       └──────────────────────────────────────────────────────────────────────┘
                                          │
                                          ▼
                               ┌─────────────────────┐
                               │     /contracts      │
                               │  Read  │ Write       │
                               │  call_contract │ tx │
                               └─────────────────────┘
```

