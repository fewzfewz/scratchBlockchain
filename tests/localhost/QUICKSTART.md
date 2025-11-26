# Localhost Testing - Quick Start Guide

## 🚀 Quick Start (5 minutes)

### Step 1: Start Testnet
```bash
cd /home/fewzan/.gemini/antigravity/scratch/deployment/local
./setup.sh
```

This will take 10-15 minutes to build Docker images.

### Step 2: Verify It's Running
```bash
# Check all services are up
docker-compose ps

# Should see 9 services: validator1, validator2, validator3, rpc1, rpc2, prometheus, grafana, faucet, nginx
```

### Step 3: Run Quick Tests
```bash
cd ../../tests/localhost/scripts

# Test 1: Check blocks are being produced
./check-blocks.sh

# Test 2: Check metrics are working
./check-metrics.sh

# Test 3: Test SDK connection
node 30-sdk-connect.js

# Test 4: Test wallet functionality
node 31-sdk-wallet.js
```

## 📊 What You Can Test

### ✅ Ready Now (No setup needed)
1. **Core Blockchain** - Blocks, transactions, state
2. **Consensus** - BFT, finality, voting
3. **Monitoring** - Prometheus, Grafana
4. **SDK** - Connection, wallet, queries

### 🔧 Needs Setup
5. **Governance** - Needs proposal creation
6. **Staking** - Needs validator registration
7. **Bridge** - Needs local Ethereum node
8. **Runtime Upgrades** - Needs upgrade proposal

## 🎯 Recommended Testing Order

### Day 1: Core Features (2 hours)
```bash
# 1. Start testnet
cd deployment/local && ./setup.sh

# 2. Wait for build to complete (10-15 min)

# 3. Run core tests
cd ../../tests/localhost/scripts
./check-blocks.sh
node 02-state-queries.js
node 30-sdk-connect.js
node 31-sdk-wallet.js
```

### Day 2: Transactions (1 hour)
```bash
# Build SDK first
cd sdk/javascript
npm install && npm run build && npm link

# Run transaction tests
cd ../../tests/localhost/scripts
node 01-send-transaction.js
```

### Day 3: Monitoring (30 min)
```bash
# Check Prometheus
./check-metrics.sh

# Open Grafana
open http://localhost/grafana
# Login: admin/admin
# Import dashboard from: monitoring/grafana/dashboards/network-overview.json
```

### Day 4: Advanced (as needed)
- Governance proposals
- Staking operations
- Bridge testing (with local Ethereum)
- Runtime upgrades

## 📝 Test Scripts Available

### Core Tests
- ✅ `check-blocks.sh` - Verify block production
- ✅ `02-state-queries.js` - Test state queries
- ✅ `01-send-transaction.js` - Send transactions

### SDK Tests
- ✅ `30-sdk-connect.js` - Test SDK connection
- ✅ `31-sdk-wallet.js` - Test wallet functionality

### Monitoring Tests
- ✅ `check-metrics.sh` - Verify Prometheus metrics

### Coming Soon
- `11-create-proposal.js` - Governance proposals
- `16-stake-tokens.js` - Staking
- `21-bridge-lock.js` - Bridge operations
- `25-propose-upgrade.js` - Runtime upgrades

## 🐛 Troubleshooting

### Testnet Won't Start
```bash
# Check Docker
docker ps

# Check logs
cd deployment/local
docker-compose logs -f

# Rebuild if needed
docker-compose down -v
docker-compose build --no-cache
docker-compose up -d
```

### Tests Fail: "Cannot connect"
```bash
# Check RPC is accessible
curl http://localhost/rpc/status

# Check nginx is running
docker-compose ps nginx

# Restart services
docker-compose restart
```

### SDK Tests Fail
```bash
# Build and link SDK
cd sdk/javascript
npm install
npm run build
npm link

# Verify link
npm list -g --depth=0 | grep modular
```

## 📊 Success Criteria

After testing, you should have:

- ✅ Testnet running (9 services up)
- ✅ Blocks being produced every 3 seconds
- ✅ Transactions processing successfully
- ✅ SDK connecting and working
- ✅ Monitoring showing metrics
- ✅ No critical errors in logs

## 🎉 Next Steps

Once localhost testing is complete:

1. Document any issues found
2. Fix critical bugs
3. Run stress tests (optional)
4. Deploy to cloud (see `deployment/cloud/README.md`)

## 📚 Full Testing Plan

For complete testing of all 45+ features, see:
- [Complete Testing Plan](../../../brain/.../localhost_testing_plan.md)
- [All Test Scripts](./scripts/)

## ⏱️ Time Estimates

- **Quick validation**: 30 minutes
- **Core features**: 2 hours
- **Important features**: 6 hours
- **Complete testing**: 12 hours

Start with quick validation, then expand as needed!
