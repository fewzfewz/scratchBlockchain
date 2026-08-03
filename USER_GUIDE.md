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
| Node metrics | http://localhost:26657/metrics |
| Faucet (test tokens) | Faucet page or `POST /faucet/request` |
| Validator onboarding | http://localhost:5173/validators/onboard |
| Grafana dashboards | http://localhost:3000 (`admin` / `admin`) |
| Prometheus | http://localhost:9095 |
| Alertmanager | http://localhost:9093 |

## 4. Pages in the Web App

| Route | Page | What it's for |
|-------|------|---------------|
| `/` | Home | Network overview & live status |
| `/explorer` | Explorer | Blocks, validators, staking & rewards estimator |
| `/wallet` | Wallet | Create wallets, send tokens, on-chain tx history |
| `/deploy` | Deploy | Deploy ERC20/ERC721 contracts |
| `/faucet` | Faucet | Get free test tokens |
| `/governance` | Governance | View proposals, vote, propose (signed txs) |
| `/validators/onboard` | Validators | Operator checklist, health checks, register |
| `/docs` | Docs | Plain-text docs & `curl` examples |
| `/api-docs` | API Reference | Interactive API — try endpoints in the browser |
| `/sdk` | SDK Portal | Developer SDK reference |

## 5. Create a Wallet

1. Go to **Wallet** (`/wallet`).
2. Click **Generate** to create a new wallet.
3. You get:
   - **Address** — your public account (`0x...`). Share this to receive tokens.
   - **Public key** — your identity on the network.
   - **Private key** — the secret that lets you spend. **Never share it.**

Your wallet is saved in your browser (localStorage).

## 6. Get Test Tokens (Faucet)

1. Go to **Faucet** (`/faucet`).
2. Paste your wallet address.
3. Click **Request Tokens**.

The node enforces a **60-second cooldown** per address.

## 7. Send Tokens

1. In **Wallet**, enter recipient address and amount.
2. Click **Send**. The wallet signs and submits the transaction.
3. View history in the Wallet panel (merged with on-chain data via `GET /txs/{address}`).

## 8. Explore & Stake

- **Explorer** — block height, validators, delegation lookup, rewards estimator.
- **Governance** — proposals and voting (requires a funded wallet).

## 9. Run a Validator (operators)

1. Open **Validators** (`/validators/onboard`).
2. Run the pre-flight health checks.
3. Follow the 7-step checklist (build node, keys, genesis, peers, register).
4. Monitor via Grafana **Validator Onboarding** dashboard.
5. Alerts fire via Alertmanager when validators go down or consensus stalls.

After registration, the node **hot-reloads** the BFT validator set from on-chain state — no restart required.

## 10. Integration Tests (developers)

With Docker testnet running:

```bash
cd tests/localhost && npm install
cd scripts
node 11-create-proposal.js
node 12-vote-proposal.js
node 13-execute-proposal.js
node 16-stake-tokens.js
node 17-unstake-tokens.js
node 18-register-validator.js
node 40-load-test.js
node 41-chaos-smoke.js
```

Or run the full suite:

```bash
cd tests/localhost && bash run-all-tests.sh
```

## 11. Common Questions

**"The page says Node offline."**  
Start Docker: `cd deployment/local && docker-compose up -d`, then `curl http://localhost:8545/health`.

**"Faucet succeeded but balance is 0."**  
Wait for the next block (~3s) and refresh Wallet.

**"I lost my private key."**  
Generate a new wallet — this is a testnet only.

See `/api-docs` or [README RPC table](README.md#rpc-api) for all 29 endpoints.
