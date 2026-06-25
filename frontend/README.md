# Frontend

Web frontend applications for the Modular Blockchain.

## Sub-projects

| Directory | Description |
|---|---|
| `governance/` | React + Vite governance dashboard (port 3002, dark theme, 5 tabs: Dashboard, Proposals, Create, Treasury, Analytics) |

## Notes

- The governance UI uses mock data currently.
- Vite dev server proxies `/api` to `http://localhost:26657`.
- Other frontends (explorer, wallet, faucet, developer-portal, docs) are at the workspace root level.
