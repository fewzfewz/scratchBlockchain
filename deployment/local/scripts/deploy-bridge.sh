#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
INTEROP="$ROOT/interop"
ETH_RPC="${ETH_RPC_URL:-http://127.0.0.1:9545}"

echo "==> Deploying Bridge.sol to Hardhat at $ETH_RPC"

cd "$INTEROP"
if [ ! -d node_modules ]; then
  npm install
fi

npx hardhat compile
ETH_RPC_URL="$ETH_RPC" npx hardhat run scripts/deploy.js --network localhost

echo "==> Bridge config written to frontend/public/bridge-config.json"
