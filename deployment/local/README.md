# Local Deployment Assets

Docker Compose stack for the full Nebula testnet.

## One-command start (from repo root)

```bash
./start-all.sh
```

This will:
1. Build `target/release/node`
2. Start Docker: 3 validators, 2 RPC nodes, Hardhat (ETH), frontend, Prometheus, Grafana, nginx
3. Wait for Nebula RPC (`8545`) and Hardhat (`9545`)
4. Deploy `Bridge.sol` and write `frontend/public/bridge-config.json`
5. Print all URLs

## Stop

```bash
./stop-all.sh
```

## Services & ports

| Port | Service |
|------|---------|
| **5173** | Frontend (React + 3D UI) |
| **8545** | Nebula RPC (validator1 — use this in the wallet) |
| **8546–8549** | Other node RPC ports |
| **9545** | Hardhat / Ethereum (Bridge.sol) |
| **3002** | Grafana (`admin` / `admin`) |
| **9095** | Prometheus |
| **80** | nginx gateway |

## Options

```bash
./start-all.sh --no-build          # skip cargo build
./start-all.sh --no-bridge         # skip Bridge.sol deploy
./start-all.sh --local-frontend    # run Vite on host instead of Docker
```

## Manual bridge deploy

```bash
./deployment/local/scripts/deploy-bridge.sh
```

Requires Hardhat running on port **9545** (not 8545 — that is Nebula).

## Environment

Nebula nodes inside Docker receive `ETH_RPC_URL=http://hardhat:9545` for `/bridge/mint` ETH tx verification.

## Dev mode and the admin token

The node configs in `configs/` set `api.dev_mode = true`. That turns on routes that change state directly, without going through consensus:

- **Dev routes** (open in dev mode): `/faucet/request`, `/deploy_wasm`
- **Admin routes**: `/validators/register`, `/delegate`, `/runtime/propose`, `/runtime/approve`, `/bridge/mint`, `/bridge/unlock`, `/bridge/unlock/ack`, `/connect_peer`

To protect the admin routes, set `NODE_API_ADMIN_TOKEN=<token>` on the nodes. Then pass the token from each client:

- **Relayer and test scripts:** set `NEBULA_ADMIN_TOKEN`
- **Browser:** run `localStorage.setItem('nebula_admin_token', '<token>')`

When `dev_mode` is off (the default), every one of these routes returns 404. `/bridge/mint` also has these rules:

- It always requires an `eth_tx_hash`.
- It only checks Ethereum through the node's own `ETH_RPC_URL`.
- It stores each processed hash on disk, so the same Ethereum transaction can't mint twice, even after a restart.

## Upgrading from an older devnet

The transaction and header hash formats changed in Phase 1, and transactions now carry a `public_key` field. Blocks from an existing devnet won't decode under the new formats. Before starting again, wipe the node data volumes with `docker compose down -v`.
