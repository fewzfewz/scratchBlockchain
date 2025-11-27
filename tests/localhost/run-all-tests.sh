#!/bin/bash
set -e

echo "=========================================="
echo "Complete Localhost Testing Suite"
echo "=========================================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Test results
PASSED=0
FAILED=0
SKIPPED=0

# Function to run test
run_test() {
    local test_name=$1
    local test_script=$2
    
    echo -e "${BLUE}Running: ${test_name}${NC}"
    
    if [ -f "$test_script" ]; then
        if node "$test_script"; then
            echo -e "${GREEN}✅ PASSED${NC}\n"
            ((PASSED++))
        else
            echo -e "${RED}❌ FAILED${NC}\n"
            ((FAILED++))
        fi
    else
        echo -e "${YELLOW}⏭️  SKIPPED (script not found)${NC}\n"
        ((SKIPPED++))
    fi
}

# Check prerequisites
echo "Checking prerequisites..."
echo ""

# Check if testnet is running
if ! docker ps | grep -q "validator1"; then
    echo -e "${RED}❌ Testnet is not running!${NC}"
    echo "Please start testnet first:"
    echo "  cd deployment/local && docker-compose up -d"
    exit 1
fi

echo -e "${GREEN}✅ Testnet is running${NC}"
echo ""

# Check if SDK is built
if [ ! -d "../../sdk/javascript/dist" ]; then
    echo -e "${YELLOW}⚠️  SDK not built. Building now...${NC}"
    cd ../../sdk/javascript
    npm install
    npm run build
    npm link
    cd ../../tests/localhost/scripts
    echo -e "${GREEN}✅ SDK built${NC}"
fi

echo ""
echo "=========================================="
echo "Phase 1: Core Blockchain Tests"
echo "=========================================="
echo ""

run_test "1.2 Block Production" "./check-blocks.sh"
run_test "1.3 Transaction Processing" "./01-send-transaction.js"
run_test "1.4 State Queries" "./02-state-queries.js"

echo ""
echo "=========================================="
echo "Phase 2: Consensus Tests"
echo "=========================================="
echo ""

run_test "2.1 BFT Consensus" "./03-consensus-test.js"
run_test "2.2 Block Finalization" "./check-finality.sh"

echo ""
echo "=========================================="
echo "Phase 3: Transaction Tests"
echo "=========================================="
echo ""

run_test "3.1 Transaction Types" "./06-transaction-types.js"
run_test "3.2 Invalid Transactions" "./07-invalid-transactions.js"
run_test "3.3 Nonce Management" "./08-nonce-test.js"

echo ""
echo "=========================================="
echo "Phase 4: Governance Tests"
echo "=========================================="
echo ""

run_test "4.1 Create Proposal" "./11-create-proposal.js"
run_test "4.2 Vote on Proposal" "./12-vote-proposal.js"
run_test "4.3 Execute Proposal" "./13-execute-proposal.js"

echo ""
echo "=========================================="
echo "Phase 5: Staking Tests"
echo "=========================================="
echo ""

run_test "5.1 Stake Tokens" "./16-stake-tokens.js"
run_test "5.2 Unstake Tokens" "./17-unstake-tokens.js"
run_test "5.3 Validator Registration" "./18-register-validator.js"

echo ""
echo "=========================================="
echo "Phase 8: Monitoring Tests"
echo "=========================================="
echo ""

run_test "8.1 Prometheus Metrics" "./check-metrics.sh"
run_test "8.2 Grafana Dashboard" "./check-grafana.sh"

echo ""
echo "=========================================="
echo "Phase 9: SDK Tests"
echo "=========================================="
echo ""

run_test "9.1 SDK Connection" "./30-sdk-connect.js"
run_test "9.2 SDK Wallet" "./31-sdk-wallet.js"
run_test "9.3 SDK Transactions" "./32-sdk-transactions.js"
run_test "9.4 SDK Queries" "./33-sdk-queries.js"

echo ""
echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo ""
echo -e "${GREEN}Passed:  ${PASSED}${NC}"
echo -e "${RED}Failed:  ${FAILED}${NC}"
echo -e "${YELLOW}Skipped: ${SKIPPED}${NC}"
echo ""

TOTAL=$((PASSED + FAILED + SKIPPED))
echo "Total tests: ${TOTAL}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some tests failed${NC}"
    exit 1
fi
