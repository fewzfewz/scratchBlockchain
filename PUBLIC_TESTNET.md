# Public Testnet Deployment

Quick path from local Docker to a cloud-hosted Nebula testnet.

## Prerequisites

- AWS / GCP / Azure account
- Terraform >= 1.5
- Ansible (for node provisioning)
- Domain name (optional, for HTTPS)

## One-command AWS plan

```bash
bash deployment/cloud/scripts/deploy-public-testnet.sh
cd deployment/cloud/terraform/aws && terraform apply tfplan
```

## Architecture

| Component | Count | Role |
|-----------|-------|------|
| Validators | 3 | BFT consensus, block production |
| RPC nodes | 2 | Public JSON-RPC behind load balancer |
| Prometheus | 1 | Metrics (internal) |
| Grafana | 1 | Dashboards + Validator Onboarding |
| Alertmanager | 1 | Slack/PagerDuty alerts |

## Post-deploy checklist

1. **Provision nodes** — `deployment/cloud/ansible/playbook.yml`
2. **DNS** — `deployment/cloud/scripts/setup-dns.sh`
3. **TLS** — `deployment/cloud/scripts/setup-ssl.sh` or Cloudflare proxy
4. **Smoke test** — `curl https://rpc.YOUR_DOMAIN/health`
5. **Integration tests** — point `NEBULA_API_URL` at public RPC and run `tests/localhost/scripts/*.js`

## Environment variables

| Variable | Purpose |
|----------|---------|
| `AWS_REGION` | Terraform region (default `us-east-1`) |
| `TF_VAR_domain_name` | Public RPC hostname base |
| `NODE_BINARY_PATH` | Local `target/release/node` copied by Ansible (default in `deploy-aws.sh`) |
| `ETH_RPC_URL` | Ethereum JSON-RPC for bridge relayer |
| `ETH_BRIDGE_ADDRESS` | Deployed `Bridge.sol` on ETH |
| `RELAYER_PRIVATE_KEYS` | Comma-separated ECDSA keys for `npm run relayer` |
| `NEBULA_CHAIN_ID` / `ETH_CHAIN_ID` | Bridge message chain IDs (default 100 / 1) |
| `SLACK_WEBHOOK_URL` | Alertmanager receiver |
| `PAGERDUTY_ROUTING_KEY` | Critical alerts |

## Cost estimate

See [deployment/cloud/README.md](README.md) — ~$520/mo AWS, ~$180/mo DigitalOcean (manual).

## Rollback

Keep local Docker testnet running. If cloud deploy fails, point DNS back to local nginx and debug.

See also: [MONITORING_GUIDE.md](../../MONITORING_GUIDE.md), [docs/validator-onboarding.md](../../docs/validator-onboarding.md)
