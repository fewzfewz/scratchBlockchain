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
