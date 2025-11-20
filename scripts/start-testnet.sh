#!/bin/bash
# Start local 3-node testnet

set -e

echo "🚀 Starting Modular Blockchain Testnet..."

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Error: Docker is not running"
    exit 1
fi

# Build images
echo "📦 Building Docker images..."
docker-compose build

# Start nodes
echo "🔧 Starting nodes..."
docker-compose up -d

# Wait for nodes to start
echo "⏳ Waiting for nodes to initialize..."
sleep 5

# Show status
echo "✅ Testnet started successfully!"
echo ""
echo "Node 1: http://localhost:9933"
echo "Node 2: http://localhost:9935"
echo "Node 3: http://localhost:9937"
echo ""
echo "To view logs: docker-compose logs -f"
echo "To stop testnet: ./scripts/stop-testnet.sh"
