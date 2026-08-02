# Frontend

Unified React SPA for the Nebula blockchain (Vite + Tailwind + React Router).

## Routes (port 5173)

| Route | Page |
|-------|------|
| `/` | Home — network overview |
| `/explorer` | Blocks, validators, staking (rewards estimator) |
| `/wallet` | Key generation, send tx, balances, on-chain tx history |
| `/deploy` | EVM contract deployment (ERC20/ERC721 presets) |
| `/faucet` | Test token requests (`POST /faucet/request`) |
| `/governance` | Proposals, voting, treasury, analytics |
| `/docs` | Human-readable API reference |
| `/api-docs` | Interactive Swagger UI |
| `/sdk` | JavaScript SDK portal |
| `/developer-portal` | Developer dashboard |

## Backend Integration

- Default API base: `http://localhost:8545`
- On-chain tx history: `GET /txs/{address}?limit=25`
- Contract deploy: `POST /submit_tx` with contract init bytecode (no `to` field)
- WebSocket available at `ws://localhost:8545/ws` (node); frontend may still poll for some views
- Dark/light theme persisted in `localStorage`

## Known Gaps

- OpenAPI/Swagger may lag behind latest RPC routes — see root [README.md](../README.md#rpc-api)
- Governance vote/propose requires a funded wallet with signed transactions

## Development

```bash
npm install
npm run dev
# http://localhost:5173
```
