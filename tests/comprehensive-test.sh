#!/bin/bash

# Comprehensive Blockchain Feature Test Suite
# Tests every working feature of the Modular Blockchain

set -e

echo "=========================================="
echo "MODULAR BLOCKCHAIN - COMPREHENSIVE TEST SUITE"
echo "=========================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Helper function to print test results
test_result() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓ PASS${NC}: $2"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ FAIL${NC}: $2"
        ((TESTS_FAILED++))
    fi
}

echo "=========================================="
echo "TEST 1: BLOCK PRODUCTION"
echo "=========================================="

echo "Testing: Get blockchain status..."
STATUS=$(curl -s http://localhost:26657/status)
echo "Response: $STATUS"

HEIGHT=$(echo $STATUS | grep -o '"height":[0-9]*' | grep -o '[0-9]*')
if [ "$HEIGHT" -gt 0 ]; then
    test_result 0 "Block production is working (height: $HEIGHT)"
else
    test_result 1 "Block production failed (height: $HEIGHT)"
fi

echo ""
echo "Testing: Get specific block..."
BLOCK=$(curl -s http://localhost:26657/block/1)
if echo "$BLOCK" | grep -q '"block"'; then
    test_result 0 "Block retrieval by height works"
else
    test_result 1 "Block retrieval by height failed"
fi

echo ""
echo "=========================================="
echo "TEST 2: HEALTH CHECK"
echo "=========================================="

HEALTH=$(curl -s http://localhost:26657/health)
echo "Response: $HEALTH"
if echo "$HEALTH" | grep -q '"status":"healthy"'; then
    test_result 0 "Health check endpoint works"
else
    test_result 1 "Health check endpoint failed"
fi

echo ""
echo "=========================================="
echo "TEST 3: ACCOUNT MANAGEMENT"
echo "=========================================="

echo "Testing: Query account balance..."
BALANCE=$(curl -s http://localhost:26657/balance/1111111111111111111111111111111111111111)
echo "Response: $BALANCE"
if echo "$BALANCE" | grep -q '"balance"'; then
    test_result 0 "Account balance query works"
else
    test_result 1 "Account balance query failed"
fi

echo ""
echo "=========================================="
echo "TEST 4: MEMPOOL"
echo "=========================================="

echo "Testing: Query mempool..."
MEMPOOL=$(curl -s http://localhost:26657/mempool)
echo "Response: $MEMPOOL"
if echo "$MEMPOOL" | grep -q '"size"'; then
    test_result 0 "Mempool query works"
    MEMPOOL_SIZE=$(echo $MEMPOOL | grep -o '"size":[0-9]*' | grep -o '[0-9]*')
    echo "  Current mempool size: $MEMPOOL_SIZE"
else
    test_result 1 "Mempool query failed"
fi

echo ""
echo "=========================================="
echo "TEST 5: FAUCET SERVICE"
echo "=========================================="

echo "Testing: Request test tokens from faucet..."
FAUCET_RESPONSE=$(curl -s -X POST http://localhost:3001/faucet \
  -H "Content-Type: application/json" \
  -d '{"address":"0x1111111111111111111111111111111111111111"}')
echo "Response: $FAUCET_RESPONSE"
if echo "$FAUCET_RESPONSE" | grep -q '"status":"success"'; then
    test_result 0 "Faucet service works"
    AMOUNT=$(echo $FAUCET_RESPONSE | grep -o '"amount":"[^"]*"' | cut -d'"' -f4)
    echo "  Tokens received: $AMOUNT"
else
    test_result 1 "Faucet service failed"
fi

echo ""
echo "=========================================="
echo "TEST 6: PROMETHEUS METRICS"
echo "=========================================="

echo "Testing: Fetch Prometheus metrics..."
METRICS=$(curl -s http://localhost:26657/metrics)
if echo "$METRICS" | grep -q 'block_height'; then
    test_result 0 "Prometheus metrics endpoint works"
    echo "  Sample metrics found: block_height, transaction_count"
else
    test_result 1 "Prometheus metrics endpoint failed"
fi

echo ""
echo "=========================================="
echo "TEST 7: PEER CONNECTIVITY"
echo "=========================================="

echo "Testing: Get validator peer IDs..."
PEER1=$(docker logs validator1 2>&1 | grep "Local peer id" | tail -1)
PEER2=$(docker logs validator2 2>&1 | grep "Local peer id" | tail -1)
PEER3=$(docker logs validator3 2>&1 | grep "Local peer id" | tail -1)

if [ -n "$PEER1" ] && [ -n "$PEER2" ] && [ -n "$PEER3" ]; then
    test_result 0 "All validators have peer IDs"
    echo "  Validator 1: $PEER1"
    echo "  Validator 2: $PEER2"
    echo "  Validator 3: $PEER3"
else
    test_result 1 "Failed to get validator peer IDs"
fi

echo ""
echo "Testing: Check for peer connections..."
CONNECTIONS=$(docker logs validator1 2>&1 | grep "Connection established" | wc -l)
if [ "$CONNECTIONS" -gt 0 ]; then
    test_result 0 "Validators have established connections ($CONNECTIONS connections)"
else
    test_result 1 "No peer connections found"
fi

echo ""
echo "=========================================="
echo "TEST 8: CONSENSUS"
echo "=========================================="

echo "Testing: Check for consensus activity..."
CONSENSUS_LOGS=$(docker logs validator1 2>&1 | grep -E "(Producing new block|Block produced)" | tail -5)
if [ -n "$CONSENSUS_LOGS" ]; then
    test_result 0 "Consensus is active (blocks being produced)"
    echo "  Recent activity:"
    echo "$CONSENSUS_LOGS" | head -3
else
    test_result 1 "No consensus activity detected"
fi

echo ""
echo "=========================================="
echo "TEST 9: STORAGE PERSISTENCE"
echo "=========================================="

echo "Testing: Check if data directories exist..."
if docker exec validator1 ls /data/block_db > /dev/null 2>&1; then
    test_result 0 "Block storage directory exists"
else
    test_result 1 "Block storage directory missing"
fi

if docker exec validator1 ls /data/state_db > /dev/null 2>&1; then
    test_result 0 "State storage directory exists"
else
    test_result 1 "State storage directory missing"
fi

if docker exec validator1 ls /data/receipts_db > /dev/null 2>&1; then
    test_result 0 "Receipt storage directory exists"
else
    test_result 1 "Receipt storage directory missing"
fi

echo ""
echo "=========================================="
echo "TEST 10: TRANSACTION SUBMISSION"
echo "=========================================="

echo "Testing: Submit a transaction..."
TX_RESPONSE=$(curl -s -X POST http://localhost:26657/submit_tx \
  -H "Content-Type: application/json" \
  -d '{
    "sender":[170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170],
    "nonce":0,
    "payload":[1,2,3,4,5],
    "signature":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,64],
    "gas_limit":21000,
    "max_fee_per_gas":2000000000,
    "max_priority_fee_per_gas":2000000000,
    "value":0
  }')
echo "Response: $TX_RESPONSE"
if echo "$TX_RESPONSE" | grep -q '"hash"'; then
    test_result 0 "Transaction submission endpoint works"
else
    test_result 1 "Transaction submission failed"
fi

echo ""
echo "=========================================="
echo "TEST 11: MONITORING SERVICES"
echo "=========================================="

echo "Testing: Prometheus service..."
PROM_STATUS=$(curl -s http://localhost:9095/-/healthy)
if [ -n "$PROM_STATUS" ]; then
    test_result 0 "Prometheus is accessible"
else
    test_result 1 "Prometheus is not accessible"
fi

echo ""
echo "Testing: Grafana service..."
GRAFANA_STATUS=$(curl -s http://localhost:3000/api/health)
if echo "$GRAFANA_STATUS" | grep -q 'ok'; then
    test_result 0 "Grafana is accessible"
else
    test_result 1 "Grafana is not accessible"
fi

echo ""
echo "=========================================="
echo "TEST 12: VALIDATOR OPERATIONS"
echo "=========================================="

echo "Testing: Check validator logs for block production..."
VALIDATOR_BLOCKS=$(docker logs validator1 2>&1 | grep "Block produced successfully" | wc -l)
if [ "$VALIDATOR_BLOCKS" -gt 0 ]; then
    test_result 0 "Validator 1 has produced blocks ($VALIDATOR_BLOCKS blocks)"
else
    test_result 1 "Validator 1 has not produced any blocks"
fi

echo ""
echo "Testing: Check for validator rewards..."
REWARDS=$(docker logs validator1 2>&1 | grep "Awarded.*tokens to validator" | tail -1)
if [ -n "$REWARDS" ]; then
    test_result 0 "Validator rewards are being distributed"
    echo "  $REWARDS"
else
    test_result 1 "No validator rewards found"
fi

echo ""
echo "=========================================="
echo "TEST 13: GAS METERING"
echo "=========================================="

echo "Testing: Check for gas usage in logs..."
GAS_LOGS=$(docker logs validator1 2>&1 | grep "gas_used" | tail -1)
if [ -n "$GAS_LOGS" ]; then
    test_result 0 "Gas metering is active"
    echo "  $GAS_LOGS"
else
    test_result 1 "No gas metering logs found"
fi

echo ""
echo "=========================================="
echo "TEST 14: NETWORK TOPOLOGY"
echo "=========================================="

echo "Testing: Verify all services are running..."
RUNNING_SERVICES=$(docker-compose ps | grep "Up" | wc -l)
if [ "$RUNNING_SERVICES" -ge 9 ]; then
    test_result 0 "All services are running ($RUNNING_SERVICES/9)"
else
    test_result 1 "Some services are not running ($RUNNING_SERVICES/9)"
fi

echo ""
echo "=========================================="
echo "TEST 15: RPC ENDPOINTS"
echo "=========================================="

echo "Testing: Multiple RPC endpoints..."

# Test validator1 RPC
V1_RPC=$(curl -s http://localhost:26657/health)
if echo "$V1_RPC" | grep -q 'healthy'; then
    test_result 0 "Validator1 RPC (port 26657) works"
else
    test_result 1 "Validator1 RPC (port 26657) failed"
fi

# Test validator2 RPC
V2_RPC=$(curl -s http://localhost:26659/health)
if echo "$V2_RPC" | grep -q 'healthy'; then
    test_result 0 "Validator2 RPC (port 26659) works"
else
    test_result 1 "Validator2 RPC (port 26659) failed"
fi

# Test validator3 RPC
V3_RPC=$(curl -s http://localhost:26661/health)
if echo "$V3_RPC" | grep -q 'healthy'; then
    test_result 0 "Validator3 RPC (port 26661) works"
else
    test_result 1 "Validator3 RPC (port 26661) failed"
fi

echo ""
echo "=========================================="
echo "FINAL RESULTS"
echo "=========================================="
echo ""
echo -e "${GREEN}Tests Passed: $TESTS_PASSED${NC}"
echo -e "${RED}Tests Failed: $TESTS_FAILED${NC}"
echo ""

TOTAL_TESTS=$((TESTS_PASSED + TESTS_FAILED))
SUCCESS_RATE=$((TESTS_PASSED * 100 / TOTAL_TESTS))

echo "Success Rate: $SUCCESS_RATE%"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}=========================================="
    echo "ALL TESTS PASSED! ✓"
    echo -e "==========================================${NC}"
    exit 0
else
    echo -e "${YELLOW}=========================================="
    echo "SOME TESTS FAILED"
    echo -e "==========================================${NC}"
    exit 1
fi
