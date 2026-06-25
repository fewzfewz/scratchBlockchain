# Scripts

Shell scripts for testnet management, deployment, backup, and validator setup.

## Scripts

| Script | Description |
|---|---|---|
| `start-testnet.sh` | Docker Compose testnet build + start (3 nodes) |
| `stop-testnet.sh` | Docker Compose stop + cleanup |
| `deploy.sh` | Production Docker deploy with health check retry loop |
| `deploy_testnet.sh` | Native (non-Docker) testnet: cargo build + per-node configs |
| `deploy-complete.sh` | Full environment: testnet + faucet + explorer + wallet + monitoring |
| `start-frontends.sh` | Start all 8 frontend services (wallet, faucet, explorer, etc.) |
| `setup-genesis.sh` | Genesis file generation for mainnet/testnet/devnet |
| `setup-validator.sh` | Ubuntu 22.04 validator setup: deps, systemd, firewall, auto-updates |
| `backup.sh` | Docker volume backup (tar.gz), cleans backups older than 7 days |
