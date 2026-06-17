#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

echo "============================================"
echo "  GCP Public Testnet Deployment"
echo "============================================"
echo ""

# ---- Configuration ----
GCP_PROJECT="${GCP_PROJECT:-}"
GCP_REGION="${GCP_REGION:-us-central1}"
NAME_PREFIX="${NAME_PREFIX:-modular-testnet}"
SSH_KEY_PATH="${SSH_KEY_PATH:-$HOME/.ssh/id_rsa.pub}"
DOMAIN_NAME="${DOMAIN_NAME:-}"
VALIDATOR_COUNT="${VALIDATOR_COUNT:-4}"
RPC_COUNT="${RPC_COUNT:-2}"

if [ -z "$GCP_PROJECT" ]; then
  echo "Error: GCP_PROJECT is required."
  exit 1
fi

# ---- Step 1: Terraform Apply ----
echo " Step 1: Provisioning GCP infrastructure..."
echo ""

cd "$PROJECT_ROOT/deployment/cloud/terraform/gcp"

terraform init
terraform apply -auto-approve \
  -var="gcp_project_id=$GCP_PROJECT" \
  -var="gcp_region=$GCP_REGION" \
  -var="name_prefix=$NAME_PREFIX" \
  -var="ssh_public_key_path=$SSH_KEY_PATH" \
  -var="validator_count=$VALIDATOR_COUNT" \
  -var="rpc_count=$RPC_COUNT" \
  -var="domain_name=$DOMAIN_NAME"

echo ""
echo " Infrastructure provisioned."

# ---- Step 2: Export node IPs ----
echo ""
echo " Step 2: Exporting node IPs for Ansible..."
echo ""

export BOOTSTRAP_IP=$(terraform output -json bootstrap_external_ip | jq -r '.')
export VALIDATOR1_IP=$(terraform output -json validator_external_ips | jq -r '.[0]')
export VALIDATOR2_IP=$(terraform output -json validator_external_ips | jq -r '.[1]')
export VALIDATOR3_IP=$(terraform output -json validator_external_ips | jq -r '.[2]')
export VALIDATOR4_IP=$(terraform output -json validator_external_ips | jq -r '.[3]')
export RPC1_IP=$(terraform output -json rpc_external_ips | jq -r '.[0]')
export RPC2_IP=$(terraform output -json rpc_external_ips | jq -r '.[1]')

# ---- Step 3: Build genesis ----
echo ""
echo " Step 3: Generating genesis file..."
echo ""

cd "$PROJECT_ROOT"
cargo run --release -p genesis-builder -- \
  --config deployment/cloud/configs/genesis.toml \
  --output deployment/cloud/configs/genesis.json

# ---- Step 4: Ansible deploy ----
echo ""
echo " Step 4: Deploying nodes with Ansible..."
echo ""

cd "$PROJECT_ROOT/deployment/cloud/ansible"

ansible-playbook -i inventory.yml playbook.yml

# ---- Step 5: Health check ----
echo ""
echo " Step 5: Verifying deployment..."
echo ""

sleep 10
for label in "bootstrap:$BOOTSTRAP_IP" "validator1:$VALIDATOR1_IP" "rpc1:$RPC1_IP"; do
  ip="${label##*:}"
  if curl -sf "http://$ip:26657/health" > /dev/null 2>&1; then
    echo "  $label: HEALTHY"
  else
    echo "  $label: UNHEALTHY"
  fi
done

echo ""
echo "============================================"
echo "  GCP Testnet deployed!"
echo "============================================"
echo ""
echo " RPC endpoint: https://rpc.$DOMAIN_NAME"
echo ""
echo " To teardown: terraform destroy"
echo "============================================"
