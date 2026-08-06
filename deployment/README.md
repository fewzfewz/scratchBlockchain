# Deployment

Infrastructure-as-code for local and cloud deployments of the Modular Blockchain.

## Local testnet (August 2026)

| Service | URL |
|---------|-----|
| Node RPC | `http://localhost:8545`–`8549` |
| WebSocket | `ws://localhost:8545/ws` |
| Frontend | `http://localhost:5173` |
| Prometheus | `http://localhost:9095` |
| Grafana | `http://localhost:3002` (or `http://localhost/grafana/` via nginx) |

```bash
cd deployment/local && docker-compose up -d
```

## Structure

| Path | Description |
|---|---|
| `local/` | Docker Compose testnet (3 validators, 2 RPC nodes, Prometheus, Grafana, Nginx, Faucet) |
| `cloud/terraform/aws/` | AWS: VPC, EC2 (1 bootstrap + 4 validators + 2 RPC), ALB, Route53 (~$520/mo) |
| `cloud/terraform/azure/` | Azure: VNet, VMs (Standard_D4s_v5), LB, Azure DNS (~$1,115/mo) |
| `cloud/terraform/gcp/` | GCP: VPC, n1-standard-4/8 instances, global HTTPS LB, Cloud DNS (~$608/mo) |
| `cloud/ansible/` | Ansible playbook + systemd service + Jinja2 config templates |
| `cloud/configs/` | Cloud-specific node configs, genesis, Prometheus, Nginx |
| `cloud/scripts/` | `deploy-aws.sh`, `deploy-gcp.sh`, `deploy-azure.sh`, `setup-dns.sh`, `setup-ssl.sh` |

## Usage

```bash
# Local
cd deployment/local && ./setup.sh

# Cloud (e.g. AWS)
cd deployment/cloud && ./scripts/deploy-aws.sh
```
