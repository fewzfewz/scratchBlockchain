#!/usr/bin/env bash
set -euo pipefail

echo "============================================"
echo "  SSL/TLS Certificate Setup"
echo "============================================"
echo ""

DOMAIN="${1:-}"
EMAIL="${2:-admin@${DOMAIN}}"

if [ -z "$DOMAIN" ]; then
  echo "Usage: $0 <domain> [email]"
  echo ""
  echo "Example: $0 testnet.modular-blockchain.io"
  exit 1
fi

echo "Domain: $DOMAIN"
echo "Email:  $EMAIL"
echo ""

# ---- Option 1: Let's Encrypt (Certbot) ----
setup_certbot() {
  echo " Option 1: Let's Encrypt via Certbot"
  echo ""

  if ! command -v certbot &>/dev/null; then
    echo " Installing certbot..."
    sudo apt-get update
    sudo apt-get install -y certbot python3-certbot-nginx
  fi

  echo " Obtaining certificate for $DOMAIN..."
  sudo certbot --nginx \
    -d "$DOMAIN" \
    -d "rpc.$DOMAIN" \
    -d "bootstrap.$DOMAIN" \
    --non-interactive \
    --agree-tos \
    --email "$EMAIL"

  echo ""
  echo " Certificate obtained:"
  echo "   Fullchain: /etc/letsencrypt/live/$DOMAIN/fullchain.pem"
  echo "   Private key: /etc/letsencrypt/live/$DOMAIN/privkey.pem"

  # Set up auto-renewal
  echo " Setting up auto-renewal..."
  sudo systemctl enable certbot.timer
  sudo systemctl start certbot.timer

  echo " Auto-renewal configured."
}

# ---- Option 2: Self-signed (for testing) ----
setup_self_signed() {
  echo " Option 2: Self-signed certificate (testing only)"
  echo ""

  sudo mkdir -p /etc/nginx/ssl

  sudo openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
    -keyout "/etc/nginx/ssl/$DOMAIN.key" \
    -out "/etc/nginx/ssl/$DOMAIN.crt" \
    -subj "/CN=$DOMAIN/O=Modular Blockchain/C=US" \
    -addext "subjectAltName=DNS:$DOMAIN,DNS:rpc.$DOMAIN,DNS:bootstrap.$DOMAIN"

  echo ""
  echo " Self-signed certificate created:"
  echo "   Cert: /etc/nginx/ssl/$DOMAIN.crt"
  echo "   Key:  /etc/nginx/ssl/$DOMAIN.key"
}

# ---- Option 3: AWS ACM ----
setup_acm() {
  echo " Option 3: AWS ACM Certificate"
  echo ""
  echo " For AWS, use the ACM console or CLI:"
  echo ""
  echo "   aws acm request-certificate \\"
  echo "     --domain-name \"$DOMAIN\" \\"
  echo "     --subject-alternames \"rpc.$DOMAIN\" \"bootstrap.$DOMAIN\" \\"
  echo "     --validation-method DNS \\"
  echo "     --region us-east-1"
  echo ""
  echo " Then add the CNAME validation records to Route53."
  echo ""
  echo " Note: ACM certificates are free and auto-renew."
}

# ---- Option 4: Azure App Service Certificate ----
setup_azure() {
  echo " Option 4: Azure App Service Certificate"
  echo ""
  echo " For Azure, use the Azure CLI or portal:"
  echo ""
  echo "   az appservice ase create-certificate \\"
  echo "     --name $DOMAIN \\"
  echo "     --hostname rpc.$DOMAIN \\"
  echo "     --hostname bootstrap.$DOMAIN \\"
  echo "     --hostname faucet.$DOMAIN"
  echo ""
  echo " Or use Key Vault for BYO certificate:"
  echo ""
  echo "   az keyvault certificate import \\"
  echo "     --vault-name modular-kv \\"
  echo "     --name $DOMAIN \\"
  echo "     --file /path/to/certificate.pfx \\"
  echo "     --password '...'"
  echo ""
  echo " Then reference the Key Vault cert in the App Service or LB config."
  echo ""
  echo " Alternatively, set up Let's Encrypt (Option 1) on the LB VM."
}

# ---- Choose ----
echo "Select SSL method:"
echo "  1) Let's Encrypt (Certbot) — recommended for production"
echo "  2) Self-signed — testing only"
echo "  3) AWS ACM — if deploying on AWS"
echo "  4) Azure App Service Certificate — if deploying on Azure"
echo ""
read -rp "Choice [1]: " choice

case "${choice:-1}" in
  1) setup_certbot ;;
  2) setup_self_signed ;;
  3) setup_acm ;;
  4) setup_azure ;;
  *) echo "Invalid choice."; exit 1 ;;
esac

echo ""
echo " SSL setup complete for $DOMAIN."
echo ""
echo " To configure Nginx, use:"
echo "   ssl_certificate     /etc/letsencrypt/live/$DOMAIN/fullchain.pem;"
echo "   ssl_certificate_key /etc/letsencrypt/live/$DOMAIN/privkey.pem;"
