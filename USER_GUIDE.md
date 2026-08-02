# Nebula User Guide

A plain-English guide for using the Nebula blockchain as a regular user — creating a wallet, getting test tokens, sending payments, and exploring the network.

> This is a **local testnet**. All tokens are fake, for learning and testing only. Never store real funds here.

## 1. What Is This?

Nebula is a blockchain network. Like Bitcoin or Ethereum, it keeps a shared record of transactions, but it is run for you on your own machine. You interact with it through a web app (the "frontend") and an API that talks to the network's nodes.

## 2. Getting Started

### Start the network (one time)

```bash
cd deployment/local
docker-compose up -d
```

Wait ~30 seconds for the nodes to start producing blocks. Check that it's alive:

```bash
curl http://localhost:8545/status
```

You should see a JSON response with a `height` number that keeps increasing.

### Start the web app (frontend)

```bash
cd frontend
npm install   # only the first time
npm run dev
```

Open your browser at **http://localhost:5173**.

## 3. Where Is Everything?

| What | Where |
|------|-------|
| **Web app (frontend)** | http://localhost:5173 |
| Node API (blockchain) | http://localhost:8545 |
| Node RPC (health) | http://localhost:26657 |
| Faucet (test tokens) | via the Faucet page in the web app |
| Network activity charts | http://localhost:9095 (Prometheus) |
| Dashboards | http://localhost:3000 (Grafana, login `admin` / `admin`) |

## 4. Pages in the Web App

| Route | Page | What it's for |
|-------|------|---------------|
| `/` | Home | Network overview & live status |
| `/explorer` | Explorer | Browse the chain: dashboard, validators, staking |
| `/wallet` | Wallet | Create wallets, check balances, send tokens |
| `/faucet` | Faucet | Get free test tokens |
| `/governance` | Governance | View proposals & voting (demo data) |
| `/docs` | Docs | Plain-text docs & `curl` examples |
| `/api-docs` | API Reference | Interactive API — try endpoints in the browser |
| `/sdk` | SDK Portal | Developer SDK reference |

## 5. Create a Wallet

1. Go to **Wallet** (`/wallet`).
2. Click **Generate** (or similar) to create a new wallet.
3. You get three things:
   - **Address** — your public "account number" (`0x...`). Share this to receive tokens.
   - **Public key** — your identity on the network.
   - **Private key** — the secret that lets you spend. **Never share it.**

Your wallet is saved in your browser (localStorage). You can switch between several wallets you've created.

> ⚠️ Because the private key lives in your browser, clearing browser data removes it. For a local testnet that's fine — just generate a new one.

## 6. Get Test Tokens (Faucet)

1. Go to **Faucet** (`/faucet`).
2. Paste a wallet address (or use the pre-filled demo address).
3. Click **Request Tokens**.

Tokens are credited to that address almost immediately (the node's `/faucet/request` endpoint adds them on-chain). Each address is rate-limited to one request per 24 hours.

## 7. Check a Balance

- **In the Wallet page**: the balance of the selected wallet is shown automatically and refreshes every few seconds.
- **Via the API**:

```bash
curl http://localhost:8545/balance/0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18
```

## 8. Send Tokens

1. In **Wallet**, make sure you're on the account that holds the tokens.
2. Go to the **Transfer/Send** section.
3. Enter:
   - **Recipient address** (`0x...`)
   - **Amount**
4. Click **Send**. Your wallet signs the transaction and submits it to the node.

The transaction is added to the network and included in the next block. You can track it by its transaction hash (returned after sending) — view it on the Explorer or via:

```bash
curl http://localhost:8545/tx/<hash>
```

## 9. Explore the Network

Go to **Explorer** (`/explorer`):

- **Dashboard** — current block height, finalized height, pending transactions (mempool).
- **Validators** — the nodes that secure the network and produce blocks.
- **Staking** — stake totals and delegation overview.

## 10. Common Questions

**"The page says Node offline."**
The frontend talks to the node API at `http://localhost:8545`. Make sure the Docker network is running (`docker-compose up -d`) and that the API responds:

```bash
curl http://localhost:8545/status
```

**"The faucet says success but I have no tokens."**
Wait a few seconds for the next block, then refresh the Wallet. If it still shows zero, confirm the node is running and check the balance via `curl`.

**"I forgot my private key / lost my wallet."**
For this testnet, just generate a new wallet in the Wallet page. Real networks would require a backup of your private key — keep yours safe.

## 11. Useful API Commands

```bash
# Node status (height, finalized, mempool)
curl http://localhost:8545/status

# Latest block
curl http://localhost:8545/block/latest

# Block by height
curl http://localhost:8545/block/42

# Transaction by hash
curl http://localhost:8545/tx/<hash>

# Validator set
curl http://localhost:8545/validators

# Health check
curl http://localhost:8545/health
```

See `/api-docs` in the web app for the full interactive API reference.
