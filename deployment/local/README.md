# Local Deployment Assets

Docker Compose stack for the full Nebula testnet.

## One-command start (from repo root)

```bash
./start-all.sh
```

This will:
1. Build `target/release/node`
2. Start Docker: 4 validators, 2 RPC nodes, Hardhat (ETH), frontend, Prometheus, Grafana, nginx
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
| **8546–8550** | Other node RPC ports (validator2, validator3, rpc1, rpc2, validator4) |
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

The node configs in `configs/` set `api.dev_mode = true` and `api.dev_funding_key`. The funding key belongs to an account that genesis funds with 1e9 NBL. Three dev routes submit real transactions signed by that account, so every node applies them through consensus:

- `/faucet/request` — a transfer of up to 10 NBL. Funds arrive with the next block.
- `/deploy_wasm` — a `WASMDEPLOY:` transaction. A contract name can only be registered once.
- `/bridge/mint` — an admin route; set `NODE_API_ADMIN_TOKEN` to require a bearer token. It's a transfer of at most ~18.4 NBL, the `u64` limit of a transaction's `value`.

Other admin routes are `/bridge/unlock`, `/bridge/unlock/ack` and `/connect_peer`. Clients pass the token with `NEBULA_ADMIN_TOKEN` (relayer and scripts) or `localStorage.setItem('nebula_admin_token', '<token>')` (browser).

Validator registration and delegation are signed transactions sent to `/submit_tx`. They bond the stake from the sender's balance:

- `{"action":"register_validator","stake":"<wei>","commission_rate":10}`
- `{"action":"delegate","validator":"0x…","amount":"<wei>"}`

The old `/validators/register`, `/delegate`, `/runtime/propose` and `/runtime/approve` routes return 410.

When `dev_mode` is off (the default), every dev and admin route returns 404.

The governance environment variables (`GOVERNANCE_*`) feed the genesis governance state, which the state root commits to. Every node, including the RPC nodes, must use the same values.

## Upgrading from an older devnet

Phases 1–3 changed the transaction, header and vote formats, the storage layout and the genesis state, so old data won't load. A node started on an old data directory refuses to start. Before starting again, wipe the node data volumes with `docker compose down -v`.
