#!/bin/bash

echo "🧪 Test 8.1: Prometheus Metrics"
echo ""

echo "Checking Prometheus metrics endpoint..."
echo ""

# Check if Prometheus is accessible
if curl -s http://localhost:9095/metrics > /dev/null 2>&1; then
    echo "✅ Prometheus is accessible"
else
    echo "❌ Prometheus is not accessible"
    echo "   Check if monitoring stack is running"
    exit 1
fi

echo ""
echo "Checking for blockchain metrics..."
echo ""

# Check for specific metrics
METRICS=("chain_block_height" "chain_transactions_total" "chain_validator_count" "chain_peer_count")

for metric in "${METRICS[@]}"; do
    if curl -s http://localhost:9095/metrics | grep -q "$metric"; then
        VALUE=$(curl -s http://localhost:9095/metrics | grep "^$metric " | awk '{print $2}')
        echo "✅ $metric: $VALUE"
    else
        echo "⚠️  $metric: not found"
    fi
done

echo ""
echo "✅ TEST PASSED: Prometheus Metrics"
echo "   Metrics endpoint is working"
