#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

echo "============================================"
echo "  AWS Public Testnet Deployment"
echo "============================================"
echo ""

# ---- Configuration ----
AWS_REGION="${AWS_REGION:-us-east-1}"
NAME_PREFIX="${NAME_PREFIX:-modular-testnet}"
SSH_KEY_NAME="${SSH_KEY_NAME:-modular-testnet}"
DOMAIN_NAME="${DOMAIN_NAME:-}"
ACM_CERT_ARN="${ACM_CERT_ARN:-}"
VALIDATOR_COUNT="${VALIDATOR_COUNT:-4}"
RPC_COUNT="${RPC_COUNT:-2}"

# ---- Step 1: Terraform Apply ----
echo " Step 1: Provisioning cloud infrastructure..."
echo ""

cd "$PROJECT_ROOT/deployment/cloud/terraform/aws"

terraform init
terraform apply -auto-approve \
  -var="aws_region=$AWS_REGION" \
  -var="name_prefix=$NAME_PREFIX" \
  -var="ssh_key_name=$SSH_KEY_NAME" \
  -var="validator_count=$VALIDATOR_COUNT" \
  -var="rpc_count=$RPC_COUNT" \
  -var="domain_name=$DOMAIN_NAME" \
  -var="acm_certificate_arn=$ACM_CERT_ARN"

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
export SSH_KEY_PATH="$HOME/.ssh/${SSH_KEY_NAME}.pem"

echo " Bootstrap: $BOOTSTRAP_IP"
echo " Validator 1: $VALIDATOR1_IP"
echo " Validator 2: $VALIDATOR2_IP"
echo " Validator 3: $VALIDATOR3_IP"
echo " Validator 4: $VALIDATOR4_IP"
echo " RPC 1: $RPC1_IP"
echo " RPC 2: $RPC2_IP"

# ---- Step 3: Wait for SSH ----
echo ""
echo " Step 3: Waiting for instances to be ready..."
echo ""

for ip in "$BOOTSTRAP_IP" "$VALIDATOR1_IP" "$VALIDATOR2_IP" "$VALIDATOR3_IP" "$VALIDATOR4_IP" "$RPC1_IP" "$RPC2_IP"; do
  echo "  Waiting for $ip..."
  until ssh -o StrictHostKeyChecking=no -o ConnectTimeout=2 -i "$SSH_KEY_PATH" "ubuntu@$ip" "echo ready" 2>/dev/null; do
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
  -e "node_binary_path=${NODE_BINARY_PATH:-$PROJECT_ROOT/target/release/node}" \
  -e "bootstrap_nodes=['/dns4/$BOOTSTRAP_IP/tcp/26656']"

# ---- Step 6: Health check ----
echo ""
echo " Step 6: Verifying deployment..."
echo ""

sleep 10
for name in "bootstrap:$BOOTSTRAP_IP" "validator1:$VALIDATOR1_IP" "validator2:$VALIDATOR2_IP" "validator3:$VALIDATOR3_IP" "validator4:$VALIDATOR4_IP" "rpc1:$RPC1_IP" "rpc2:$RPC2_IP"; do
  label="${name%%:*}"
  ip="${name##*:}"
  if curl -sf "http://$ip:8545/health" > /dev/null 2>&1; then
    echo "  $label ($ip): HEALTHY"
  else
    echo "  $label ($ip): UNHEALTHY (expected :8545/health)"
  fi
done

echo ""
echo "============================================"
echo "  Testnet deployed!"
echo "============================================"
echo ""
echo " RPC endpoint: https://rpc.$DOMAIN_NAME"
echo " Bootstrap:    /dns4/$BOOTSTRAP_IP/tcp/26656"
echo ""
echo " To view logs:   ssh ubuntu@<ip> 'journalctl -u modular-node -f'"
echo " To stop all:    cd deployment/cloud/terraform/aws && terraform destroy"
echo "============================================"
