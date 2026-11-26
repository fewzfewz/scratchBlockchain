# Deploying Nebula on Render + Vercel + Sepolia Bridge

## 1. Multi-Validator + Bridge Blueprint (`render.yaml`)
[render.yaml](../../render.yaml) provisions:
- `nebula-validator-1` .. `nebula-validator-4` (Render Private Services, P2P over internal DNS `:26656`, 10 GB `/data` disks)
- `nebula-rpc` (Render Web Service on `:8545`, `NODE_VALIDATOR_ENABLED=false`, `/health` check)
- `nebula-relayer-1` & `nebula-relayer-2` (ETH -> Nebula attestation/mint workers)
- `nebula-eth-unlock-relayer` (Nebula -> ETH unlock worker)

*(Private Services, Workers, and Persistent Disks require a Render Starter+ plan.)*

### Required Setup After Creating Blueprint
1. **Validator Keys**: On each `nebula-validator-1`..`4`, add a Render Secret File at `/data/validator_key.json` (`{"secret_key": "..."}`) matching its `public_key` in `deployment/local/configs/genesis.json` (or your custom genesis).
2. **RPC Secrets** (on `nebula-rpc`):
   - `NODE_API_ADMIN_TOKEN`: strong random bearer token
   - `NODE_API_DEV_FUNDING_KEY`: optional 64-hex-char Ed25519 seed for faucet/dev funding
   - `ETH_RPC_URL`, `ETH_BRIDGE_ADDRESS`: if bridge is enabled
3. **Bridge Deployment (Sepolia)**:
   ```bash
   cd interop && npm install
   SEPOLIA_RPC_URL="https://eth-sepolia.g.alchemy.com/v2/YOUR_KEY" \
   PRIVATE_KEY="0xYOUR_DEPLOYER_KEY" \
   BRIDGE_RELAYERS="0xRelayerAddr1,0xRelayerAddr2" \
   NEBULA_CHAIN_ID=1 \
   npx hardhat run scripts/deploy.js --network sepolia
   ```
   Copy the deployed `Bridge` address into `ETH_BRIDGE_ADDRESS` on Render and `VITE_BRIDGE_ADDRESS` on Vercel.
4. **Relayer Secrets**:
   - `nebula-relayer-1` / `nebula-relayer-2`: set `ETH_RPC_URL`, `ETH_BRIDGE_ADDRESS`, and `RELAYER_KEY` (matching the genesis `bridge.relayers` Ed25519 keys).
   - `nebula-eth-unlock-relayer`: set `ETH_RPC_URL`, `ETH_BRIDGE_ADDRESS`, and `RELAYER_PRIVATE_KEYS` (comma-separated Ethereum keys matching `BRIDGE_RELAYERS`).

## 2. Monitoring
Every Nebula node exposes Prometheus metrics at `:9090/metrics` inside the Render private network. You can scrape them with a Prometheus/Grafana agent running in the same Render region using `nebula-rpc:9090` and `nebula-validator-1:9090`..`4:9090`.

## 3. SDK Publishing
- **JavaScript SDK**: Push to `main` touching `sdk/javascript/**` with `NPM_TOKEN` configured in GitHub Secrets (`.github/workflows/sdk-ci.yml`).
- **Python SDK**: Tag a release `sdk-py-v*` with `PYPI_API_TOKEN` configured in GitHub Secrets (`.github/workflows/sdk-python-ci.yml`).
