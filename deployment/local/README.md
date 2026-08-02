# Local Deployment Assets

This directory contains deployment-specific assets for the local testnet environment.

**Docker Compose:** 3 validators + 2 RPC nodes + Prometheus + Grafana + nginx.

**Node RPC:** `http://localhost:8545`–`8549` (one per node)  
**WebSocket:** `ws://localhost:8545/ws`  
**Faucet:** Built into node — `POST http://localhost:8545/faucet/request` (60s server-side cooldown per address)

**Legacy:** `faucet.html` may reference an old standalone faucet port; use the node RPC endpoint above.

See [deployment/local/docker-compose.yml](docker-compose.yml) and root [README.md](../../README.md).
