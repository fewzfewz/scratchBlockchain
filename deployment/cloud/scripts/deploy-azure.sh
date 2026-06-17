#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

echo "============================================"
echo "  Azure Public Testnet Deployment"
echo "============================================"
echo ""

# ---- Configuration ----
AZURE_SUBSCRIPTION_ID="${AZURE_SUBSCRIPTION_ID:-}"
AZURE_LOCATION="${AZURE_LOCATION:-eastus}"
NAME_PREFIX="${NAME_PREFIX:-modular-testnet}"
SSH_KEY_PATH="${SSH_KEY_PATH:-$HOME/.ssh/id_rsa.pub}"
DOMAIN_NAME="${DOMAIN_NAME:-}"
VALIDATOR_COUNT="${VALIDATOR_COUNT:-4}"
RPC_COUNT="${RPC_COUNT:-2}"

if [ -z "$AZURE_SUBSCRIPTION_ID" ]; then
  echo "Error: AZURE_SUBSCRIPTION_ID is required."
  echo "Set it or run: az account show --query id -o tsv"
  exit 1
fi

if [ ! -f "$SSH_KEY_PATH" ]; then
  echo "Error: SSH public key not found at $SSH_KEY_PATH"
  exit 1
fi

# ---- Step 1: Terraform Apply ----
echo " Step 1: Provisioning Azure infrastructure..."
echo ""

cd "$PROJECT_ROOT/deployment/cloud/terraform/azure"

terraform init
terraform apply -auto-approve \
  -var="azure_subscription_id=$AZURE_SUBSCRIPTION_ID" \
  -var="azure_location=$AZURE_LOCATION" \
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

export BOOTSTRAP_IP=$(terraform output -raw bootstrap_public_ip)
export VALIDATOR1_IP=$(terraform output -json validator_public_ips | jq -r '.[0]')
export VALIDATOR2_IP=$(terraform output -json validator_public_ips | jq -r '.[1]')
export VALIDATOR3_IP=$(terraform output -json validator_public_ips | jq -r '.[2]')
export VALIDATOR4_IP=$(terraform output -json validator_public_ips | jq -r '.[3]')
export RPC1_IP=$(terraform output -json rpc_public_ips | jq -r '.[0]')
export RPC2_IP=$(terraform output -json rpc_public_ips | jq -r '.[1]')
export LB_IP=$(terraform output -raw lb_public_ip)

echo " Bootstrap: $BOOTSTRAP_IP"
echo " Validator 1: $VALIDATOR1_IP"
echo " Validator 2: $VALIDATOR2_IP"
echo " Validator 3: $VALIDATOR3_IP"
echo " Validator 4: $VALIDATOR4_IP"
echo " RPC 1: $RPC1_IP"
echo " RPC 2: $RPC2_IP"
echo " LB IP:  $LB_IP"

# ---- Step 3: Wait for SSH ----
echo ""
echo " Step 3: Waiting for instances to be ready..."
echo ""

for ip in "$BOOTSTRAP_IP" "$VALIDATOR1_IP" "$VALIDATOR2_IP" "$VALIDATOR3_IP" "$VALIDATOR4_IP" "$RPC1_IP" "$RPC2_IP"; do
  echo "  Waiting for $ip..."
  until ssh -o StrictHostKeyChecking=no -o ConnectTimeout=2 "ubuntu@$ip" "echo ready" 2>/dev/null; do
    sleep 5
  done
done

# ---- Step 4: Build genesis ----
echo ""
echo " Step 4: Generating genesis file..."
echo ""

cd "$PROJECT_ROOT"
cargo run --release -p genesis-builder -- \
  --config deployment/cloud/configs/genesis.toml \
  --output deployment/cloud/configs/genesis.json

# ---- Step 5: Ansible deploy ----
echo ""
echo " Step 5: Deploying nodes with Ansible..."
echo ""

cd "$PROJECT_ROOT/deployment/cloud/ansible"

ansible-playbook -i inventory.yml playbook.yml \
  -e "chain_id=modular-testnet-1" \
  -e "data_dir=/data/blockchain" \
  -e "genesis_path=../configs/genesis.json" \
  -e "bootstrap_nodes=['/dns4/$BOOTSTRAP_IP/tcp/26656']" \
  -e "ansible_user=ubuntu"

# ---- Step 6: Health check ----
echo ""
echo " Step 6: Verifying deployment..."
echo ""

sleep 10
for name in "bootstrap:$BOOTSTRAP_IP" "validator1:$VALIDATOR1_IP" "validator2:$VALIDATOR2_IP" "validator3:$VALIDATOR3_IP" "validator4:$VALIDATOR4_IP" "rpc1:$RPC1_IP" "rpc2:$RPC2_IP"; do
  label="${name%%:*}"
  ip="${name##*:}"
  if curl -sf "http://$ip:26657/health" > /dev/null 2>&1; then
    echo "  $label ($ip): HEALTHY"
  else
    echo "  $label ($ip): UNHEALTHY"
  fi
done

echo ""
echo "============================================"
echo "  Azure Testnet deployed!"
echo "============================================"
echo ""
echo " RPC endpoint:   https://rpc.$DOMAIN_NAME"
echo " LB public IP:   $LB_IP"
echo " Bootstrap peer: /dns4/$BOOTSTRAP_IP/tcp/26656"
echo ""
echo " To view logs:   ssh ubuntu@<ip> 'journalctl -u modular-node -f'"
echo " To teardown:    cd deployment/cloud/terraform/azure && terraform destroy"
echo "============================================"
