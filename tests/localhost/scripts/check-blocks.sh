#!/bin/bash

echo "🧪 Test 1.2: Block Production"
echo ""

echo "Checking if blocks are being produced..."
echo ""

# Get initial block height
BLOCK1=$(curl -s http://localhost/rpc/status 2>/dev/null | jq -r '.result.sync_info.latest_block_height' 2>/dev/null)

if [ -z "$BLOCK1" ] || [ "$BLOCK1" == "null" ]; then
    echo "❌ Could not get block height"
    echo "   Is testnet running?"
    exit 1
fi

echo "Initial block height: $BLOCK1"
echo "Waiting 6 seconds for new blocks..."
sleep 6

# Get new block height
BLOCK2=$(curl -s http://localhost/rpc/status 2>/dev/null | jq -r '.result.sync_info.latest_block_height' 2>/dev/null)

if [ -z "$BLOCK2" ] || [ "$BLOCK2" == "null" ]; then
    echo "❌ Could not get block height"
    exit 1
fi

echo "New block height: $BLOCK2"
echo ""

# Check if blocks increased
if [ "$BLOCK2" -gt "$BLOCK1" ]; then
    DIFF=$((BLOCK2 - BLOCK1))
    echo "✅ TEST PASSED: Block Production"
    echo "   Blocks produced: $DIFF in 6 seconds"
    echo "   Block time: ~$((6 / DIFF)) seconds"
else
    echo "❌ TEST FAILED: No new blocks produced"
    exit 1
fi
