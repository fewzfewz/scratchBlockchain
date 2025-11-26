# Localhost Testing Guide

Complete guide for testing ALL blockchain features on localhost before cloud deployment.

## 🎯 Overview

This guide covers **45+ tests** across **11 phases** to validate every feature of your blockchain on localhost.

## 📋 Quick Start

### 1. Start Testnet
```bash
cd deployment/local
./setup.sh
```

### 2. Run All Tests
```bash
cd ../../tests/localhost
./run-all-tests.sh
```

### 3. View Results
Tests will run automatically and show pass/fail for each phase.

## 📚 Test Phases

### Phase 1: Core Blockchain ✅
**Time**: 30 minutes  
**Tests**: 5  
**Critical**: Yes

- Node startup and sync
- Block production (every 3 seconds)
- Transaction processing
- State management
- P2P networking

**Run**:
```bash
cd scripts
node 01-send-transaction.js
node 02-state-queries.js
```

### Phase 2: Consensus & Finality ✅
**Time**: 20 minutes  
**Tests**: 5  
**Critical**: Yes

- BFT consensus rounds
- Block finalization
- Validator voting
- Fork handling
- Slashing conditions

**Run**:
```bash
node 03-consensus-test.js
node 04-fork-test.js
node 05-slashing-test.js
```

### Phase 3: Transactions & Mempool ✅
**Time**: 15 minutes  
**Tests**: 5  
**Critical**: Yes

- Send transactions
- Transaction validation
- Nonce management
- Gas estimation
- MEV protection

**Run**:
```bash
node 06-transaction-types.js
node 07-invalid-transactions.js
node 08-nonce-test.js
node 09-gas-estimation.js
node 10-mev-test.js
```

### Phase 4: Governance 🗳️
**Time**: 25 minutes  
**Tests**: 5  
**Critical**: Important

- Create proposals
- Vote on proposals
- Execute proposals
- Delegation
- Parameter changes

**Run**:
```bash
node 11-create-proposal.js
node 12-vote-proposal.js
node 13-execute-proposal.js
node 14-delegation-test.js
node 15-param-change.js
```

### Phase 5: Staking & Validators 💰
**Time**: 20 minutes  
**Tests**: 5  
**Critical**: Important

- Stake tokens
- Unstake tokens
- Validator registration
- Reward distribution
- Commission rates

**Run**:
```bash
node 16-stake-tokens.js
node 17-unstake-tokens.js
node 18-register-validator.js
node 19-rewards-test.js
node 20-commission-test.js
```

### Phase 6: Bridge 🌉
**Time**: 30 minutes  
**Tests**: 6  
**Critical**: Optional (needs local Ethereum)

- Deploy local Ethereum node
- Deploy bridge contracts
- Lock tokens (Modular → Ethereum)
- Unlock tokens (Ethereum → Modular)
- Relayer operations
- Multi-token support

**Setup**:
```bash
# Start local Ethereum node
cd scripts
./deploy-local-ethereum.sh

# Deploy bridge contracts
cd ../../interop
npx hardhat run scripts/deploy.js --network localhost
```

**Run**:
```bash
cd ../tests/localhost/scripts
node 21-bridge-lock.js
node 22-bridge-unlock.js
node 23-start-relayer.js
node 24-multi-token-bridge.js
```

### Phase 7: Runtime Upgrades 🔄
**Time**: 20 minutes  
**Tests**: 5  
**Critical**: Important

- Propose upgrade
- Vote on upgrade
- Execute upgrade
- State migration
- Rollback test

**Run**:
```bash
node 25-propose-upgrade.js
node 26-vote-upgrade.js
node 27-execute-upgrade.js
node 28-rollback-upgrade.js
```

### Phase 8: Monitoring 📊
**Time**: 15 minutes  
**Tests**: 5  
**Critical**: Important

- Prometheus metrics
- Grafana dashboards
- Alert rules
- Performance monitoring
- Log aggregation

**Check**:
```bash
# Prometheus
curl http://localhost:9095/metrics | grep chain_

# Grafana
open http://localhost/grafana

# Alerts
curl http://localhost:9095/alerts
```

### Phase 9: SDK Integration 🔌
**Time**: 20 minutes  
**Tests**: 6  
**Critical**: Yes

- Connect to node
- Create wallet
- Send transactions
- Query state
- Event listening
- Contract interaction

**Run**:
```bash
node 30-sdk-connect.js
node 31-sdk-wallet.js
node 32-sdk-transactions.js
node 33-sdk-queries.js
node 34-sdk-events.js
node 35-sdk-contracts.js
```

### Phase 10: Advanced Features 🚀
**Time**: 25 minutes  
**Tests**: 5  
**Critical**: Optional

- Account abstraction
- Data availability
- ZK proofs
- MEV auction
- Parallel execution

**Run**:
```bash
node 36-account-abstraction.js
node 37-da-layer.js
node 38-zk-proofs.js
node 39-mev-auction.js
node 40-parallel-execution.js
```

### Phase 11: Stress Testing 💪
**Time**: 30 minutes  
**Tests**: 5  
**Critical**: Important

- High transaction volume (10,000 tx)
- Large state size (100,000 accounts)
- Network partitions
- Validator failures
- Recovery scenarios

**Run**:
```bash
node 41-stress-test.js --txs=10000
node 42-large-state-test.js
./43-partition-test.sh
./44-validator-failure.sh
./45-recovery-test.sh
```

## 📊 Test Results

### Expected Results

After running all tests, you should see:

```
==========================================
Test Summary
==========================================

Passed:  40
Failed:  0
Skipped: 5

Total tests: 45

🎉 All tests passed!
```

### What Each Test Validates

| Phase | Feature | Validates |
|-------|---------|-----------|
| 1 | Core | Node works, blocks produced, txs processed |
| 2 | Consensus | BFT works, finality achieved, validators vote |
| 3 | Transactions | All tx types work, validation correct |
| 4 | Governance | Proposals work, voting works, execution works |
| 5 | Staking | Staking works, rewards distributed |
| 6 | Bridge | Cross-chain transfers work |
| 7 | Upgrades | Runtime can be upgraded safely |
| 8 | Monitoring | Metrics collected, dashboards work |
| 9 | SDK | Developers can build dApps |
| 10 | Advanced | Advanced features functional |
| 11 | Stress | System handles load |

## 🎯 Success Criteria

### Must Pass (Before Cloud)
- ✅ All Phase 1 tests (Core)
- ✅ All Phase 2 tests (Consensus)
- ✅ All Phase 3 tests (Transactions)
- ✅ All Phase 9 tests (SDK)

### Should Pass (Before Public)
- ✅ All Phase 4 tests (Governance)
- ✅ All Phase 5 tests (Staking)
- ✅ All Phase 7 tests (Upgrades)
- ✅ All Phase 8 tests (Monitoring)

### Nice to Have
- ✅ Phase 6 (Bridge)
- ✅ Phase 10 (Advanced)
- ✅ Phase 11 (Stress)

## 🐛 Troubleshooting

### Test Fails: "Cannot connect to RPC"
```bash
# Check testnet is running
docker-compose ps

# Restart if needed
docker-compose restart
```

### Test Fails: "Insufficient balance"
```bash
# Request tokens from faucet
open http://localhost/faucet
# Enter wallet address from test output
```

### Test Fails: "SDK not found"
```bash
# Build and link SDK
cd sdk/javascript
npm install
npm run build
npm link
```

## 📝 Test Report

After testing, generate report:

```bash
./generate-report.sh > test-report.md
```

Report includes:
- All test results
- Performance metrics
- Issues found
- Recommendations

## 🚀 Next Steps

After all tests pass:

1. ✅ Document any issues found
2. ✅ Fix critical issues
3. ✅ Retest failed tests
4. ✅ Generate final report
5. 🚀 Ready for cloud deployment!

## 📚 Additional Resources

- [Complete Testing Plan](../../../brain/.../localhost_testing_plan.md)
- [Deployment Guide](../../deployment/local/README.md)
- [Cloud Migration](../../deployment/cloud/README.md)
- [SDK Documentation](../../sdk/javascript/README.md)

## ⏱️ Time Estimate

- **Minimum** (Critical tests only): 2 hours
- **Recommended** (All important tests): 6 hours
- **Complete** (All tests): 12 hours

## 🎉 Ready to Test!

Start with:
```bash
cd deployment/local
./setup.sh
```

Then run tests:
```bash
cd ../../tests/localhost
./run-all-tests.sh
```

Good luck! 🚀
