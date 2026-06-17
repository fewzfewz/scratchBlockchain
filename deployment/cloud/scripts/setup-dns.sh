#!/usr/bin/env bash
set -euo pipefail

echo "============================================"
echo "  Public DNS Setup"
echo "============================================"
echo ""

DOMAIN="${1:-}"
LB_IP="${2:-}"

if [ -z "$DOMAIN" ] || [ -z "$LB_IP" ]; then
  echo "Usage: $0 <domain> <load-balancer-ip>"
  echo ""
  echo "Example: $0 testnet.modular-blockchain.io 123.45.67.89"
  exit 1
fi

echo "Domain: $DOMAIN"
echo "LB IP:  $LB_IP"
echo ""

# ---- AWS Route53 ----
setup_route53() {
  echo " Route53 DNS records"

  ZONE_ID=$(aws route53 list-hosted-zones-by-name \
    --dns-name "$DOMAIN" \
    --query "HostedZones[0].Id" \
    --output text 2>/dev/null | sed 's|/hostedzone/||')

  if [ -z "$ZONE_ID" ] || [ "$ZONE_ID" == "None" ]; then
    echo " Creating hosted zone for $DOMAIN..."
    ZONE_ID=$(aws route53 create-hosted-zone \
      --name "$DOMAIN" \
      --caller-reference "$(date +%s)" \
      --query "HostedZone.Id" \
      --output text | sed 's|/hostedzone/||')
    echo " Hosted zone created: $ZONE_ID"
    echo " Update your domain registrar's NS records to:"
    aws route53 get-hosted-zone --id "$ZONE_ID" \
      --query "DelegationSet.NameServers" \
      --output text
  fi

  # RPC record
  cat > /tmp/rpc-record.json <<EOF
{
  "Changes": [{
    "Action": "UPSERT",
    "ResourceRecordSet": {
      "Name": "rpc.$DOMAIN",
      "Type": "A",
      "TTL": 60,
      "ResourceRecords": [{ "Value": "$LB_IP" }]
    }
  }]
}
EOF
  aws route53 change-resource-record-sets \
    --hosted-zone-id "$ZONE_ID" \
    --change-batch file:///tmp/rpc-record.json
  echo "  rpc.$DOMAIN → $LB_IP"

  # Faucet record
  cat > /tmp/faucet-record.json <<EOF
{
  "Changes": [{
    "Action": "UPSERT",
    "ResourceRecordSet": {
      "Name": "faucet.$DOMAIN",
      "Type": "A",
      "TTL": 60,
      "ResourceRecords": [{ "Value": "$LB_IP" }]
    }
  }]
}
EOF
  aws route53 change-resource-record-sets \
    --hosted-zone-id "$ZONE_ID" \
    --change-batch file:///tmp/faucet-record.json
  echo "  faucet.$DOMAIN → $LB_IP"

  # Bootstrap record
  BOOTSTRAP_IP="${3:-}"
  if [ -n "$BOOTSTRAP_IP" ]; then
    cat > /tmp/bootstrap-record.json <<EOF
{
  "Changes": [{
    "Action": "UPSERT",
    "ResourceRecordSet": {
      "Name": "bootstrap.$DOMAIN",
      "Type": "A",
      "TTL": 60,
      "ResourceRecords": [{ "Value": "$BOOTSTRAP_IP" }]
    }
  }]
}
EOF
    aws route53 change-resource-record-sets \
      --hosted-zone-id "$ZONE_ID" \
      --change-batch file:///tmp/bootstrap-record.json
    echo "  bootstrap.$DOMAIN → $BOOTSTRAP_IP"
  fi

  rm -f /tmp/rpc-record.json /tmp/faucet-record.json /tmp/bootstrap-record.json
  echo " Route53 records created."
}

# ---- Google Cloud DNS ----
setup_cloud_dns() {
  echo " Google Cloud DNS records"

  ZONE_NAME="${DOMAIN//./-}"

  if ! gcloud dns managed-zones describe "$ZONE_NAME" &>/dev/null; then
    echo " Creating managed zone..."
    gcloud dns managed-zones create "$ZONE_NAME" \
      --dns-name="$DOMAIN" \
      --description="Testnet DNS zone"
  fi

  # RPC record
  gcloud dns record-sets create "rpc.$DOMAIN" \
    --zone="$ZONE_NAME" \
    --type="A" \
    --ttl="60" \
    --rrdatas="$LB_IP"
  echo "  rpc.$DOMAIN → $LB_IP"

  # Faucet record
  gcloud dns record-sets create "faucet.$DOMAIN" \
    --zone="$ZONE_NAME" \
    --type="A" \
    --ttl="60" \
    --rrdatas="$LB_IP"
  echo "  faucet.$DOMAIN → $LB_IP"

  echo " Cloud DNS records created."
}

# ---- Azure DNS ----
setup_azure_dns() {
  echo " Azure DNS records"

  RESOURCE_GROUP="${AZURE_RESOURCE_GROUP:-modular-testnet-rg}"

  if ! az dns zone show --name "$DOMAIN" --resource-group "$RESOURCE_GROUP" &>/dev/null; then
    echo " Creating DNS zone..."
    az dns zone create --name "$DOMAIN" --resource-group "$RESOURCE_GROUP"
  fi

  # RPC record
  az dns record-set a create \
    --name "rpc" \
    --zone-name "$DOMAIN" \
    --resource-group "$RESOURCE_GROUP" \
    --ttl 60 \
    --target "$LB_IP" 2>/dev/null || \
  az dns record-set a update \
    --name "rpc" \
    --zone-name "$DOMAIN" \
    --resource-group "$RESOURCE_GROUP" \
    --target "$LB_IP"
  echo "  rpc.$DOMAIN → $LB_IP"

  # Faucet record
  az dns record-set a create \
    --name "faucet" \
    --zone-name "$DOMAIN" \
    --resource-group "$RESOURCE_GROUP" \
    --ttl 60 \
    --target "$LB_IP" 2>/dev/null || \
  az dns record-set a update \
    --name "faucet" \
    --zone-name "$DOMAIN" \
    --resource-group "$RESOURCE_GROUP" \
    --target "$LB_IP"
  echo "  faucet.$DOMAIN → $LB_IP"

  # Bootstrap record
  BOOTSTRAP_IP="${3:-}"
  if [ -n "$BOOTSTRAP_IP" ]; then
    az dns record-set a create \
      --name "bootstrap" \
      --zone-name "$DOMAIN" \
      --resource-group "$RESOURCE_GROUP" \
      --ttl 60 \
      --target "$BOOTSTRAP_IP" 2>/dev/null || \
    az dns record-set a update \
      --name "bootstrap" \
      --zone-name "$DOMAIN" \
      --resource-group "$RESOURCE_GROUP" \
      --target "$BOOTSTRAP_IP"
    echo "  bootstrap.$DOMAIN → $BOOTSTRAP_IP"
  fi

  echo " Azure DNS records created."
}

# ---- Cloudflare ----
setup_cloudflare() {
  echo " Cloudflare DNS records"

  API_TOKEN="${CF_API_TOKEN:-}"
  if [ -z "$API_TOKEN" ]; then
    echo "Error: CF_API_TOKEN environment variable required."
    exit 1
  fi

  ZONE_ID=$(curl -sf "https://api.cloudflare.com/client/v4/zones?name=$DOMAIN" \
    -H "Authorization: Bearer $API_TOKEN" \
    -H "Content-Type: application/json" \
    | jq -r '.result[0].id')

  if [ -z "$ZONE_ID" ] || [ "$ZONE_ID" == "null" ]; then
    echo " Error: Zone not found for $DOMAIN"
    echo " Add domain to Cloudflare first."
    exit 1
  fi

  create_cf_record() {
    local name="$1" value="$2"
    curl -sf "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/dns_records" \
      -X POST \
      -H "Authorization: Bearer $API_TOKEN" \
      -H "Content-Type: application/json" \
      -d "{\"type\":\"A\",\"name\":\"$name\",\"content\":\"$value\",\"ttl\":60,\"proxied\":true}" > /dev/null
    echo "  $name.$DOMAIN → $value (proxied)"
  }

  create_cf_record "rpc" "$LB_IP"
  create_cf_record "faucet" "$LB_IP"

  echo " Cloudflare records created."
}

# ---- Choose ----
echo "Select DNS provider:"
echo "  1) AWS Route53"
echo "  2) Google Cloud DNS"
echo "  3) Cloudflare"
echo "  4) Azure DNS"
echo ""
read -rp "Choice [1]: " choice

case "${choice:-1}" in
  1) setup_route53 ;;
  2) setup_cloud_dns ;;
  3) setup_cloudflare ;;
  4) setup_azure_dns ;;
  *) echo "Invalid choice."; exit 1 ;;
esac

echo ""
echo " DNS setup complete for $DOMAIN."
echo " Records may take a few minutes to propagate."
