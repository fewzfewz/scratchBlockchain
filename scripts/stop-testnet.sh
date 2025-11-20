#!/bin/bash
# Stop local testnet

set -e

echo "🛑 Stopping Modular Blockchain Testnet..."

# Stop and remove containers
docker-compose down

# Optional: Remove volumes (uncomment to clean data)
# docker-compose down -v

echo "✅ Testnet stopped successfully!"
