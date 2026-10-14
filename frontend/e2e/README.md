# Browser end-to-end checks

Drive the web UI in headless Chromium (Chrome DevTools Protocol, no extra
dependencies; Node 22+ for the built-in `WebSocket`) against a running local
devnet and the Vite dev server.

Prerequisites: `deployment/local` devnet up (validators, RPC nodes, Hardhat
with `Bridge.sol` deployed) and `npm run dev` serving http://127.0.0.1:5173.

| Script | What it checks |
|---|---|
| `ui-check.mjs [route …]` | Loads every route at desktop and phone width; reports exceptions, console errors, failed requests and horizontal overflow; saves screenshots. |
| `flow-wallet.mjs` | Wallet → faucet → transfer → history → explorer (address, blocks, validators). |
| `flow-features.mjs` | Contract deploy (ERC20, ERC721), contract read/write, NFT mint, DeFi, WASM, account abstraction, MEV commit, validator registration, governance propose + vote, bridge in both directions, staking tab. `ONLY=<regex>` runs a subset. |

```bash
node e2e/ui-check.mjs            # output in e2e/out/ (or E2E_OUT=<dir>)
node e2e/flow-wallet.mjs
node e2e/flow-features.mjs
```

The Ethereum side of the bridge uses a stand-in `window.ethereum` that
forwards to Hardhat's unlocked dev accounts (headless Chrome has no MetaMask).
