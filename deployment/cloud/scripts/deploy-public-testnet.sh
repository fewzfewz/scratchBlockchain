#!/usr/bin/env bash
# Deploy Nebula public testnet on AWS using Terraform.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TF_DIR="$ROOT/deployment/cloud/terraform/aws"

echo "==> Nebula public testnet deploy (AWS)"
echo "    Terraform: $TF_DIR"

if ! command -v terraform >/dev/null 2>&1; then
  echo "ERROR: terraform not installed"
  exit 1
fi

export TF_VAR_name_prefix="${TF_VAR_name_prefix:-nebula-testnet}"
export TF_VAR_domain_name="${TF_VAR_domain_name:-testnet.nebula.local}"

cd "$TF_DIR"
terraform init
terraform plan \
  -var="aws_region=${AWS_REGION:-us-east-1}" \
  -out=tfplan

echo ""
echo "Review the plan above. Apply with:"
echo "  cd $TF_DIR && terraform apply tfplan"
echo ""
echo "After apply:"
echo "  1. Run Ansible: ansible-playbook -i $ROOT/deployment/cloud/ansible/inventory.yml $ROOT/deployment/cloud/ansible/playbook.yml"
echo "  2. DNS: bash $ROOT/deployment/cloud/scripts/setup-dns.sh"
echo "  3. SSL:  bash $ROOT/deployment/cloud/scripts/setup-ssl.sh"
echo "  4. Verify: curl https://rpc.\$TF_VAR_domain_name/health"
