# What's Left - Development Status

## Current Status: **PRE-MAINNET** (~96%)

### Last Updated: August 3, 2026

---

## ✅ DONE (latest batch)

### BFT Validator Hot-Reload
- ✅ `BftEngine::update_validator_set()` + `FinalityGadget::update_validator_set()`
- ✅ `governance_store::load_consensus_validators()` — stake + delegated weight
- ✅ Sync after block finalize + every 5s metrics tick
- ✅ Node restart loads validators from state trie (not stale genesis)

### Patricia Trie GC
- ✅ `PatriciaTrie::gc_orphan_nodes()` — deletes unreachable 32-byte state nodes
- ✅ Runs after block/receipt prune pass

### Integration Tests
- ✅ `12-vote-proposal.js`, `13-execute-proposal.js`
- ✅ `17-unstake-tokens.js`, `18-register-validator.js`

### Crypto Hardening
- ✅ DA: Reed–Solomon erasure coding (`reed-solomon-erasure`)
- ✅ MEV: AES-256-GCM encryption (replaces XOR at rest)
- ✅ ZK: block hash bound into state-transition proofs; batch verify checks 32-byte proofs

### Public Testnet & Audit Prep
- ✅ `PUBLIC_TESTNET.md` + `deploy-public-testnet.sh`
- ✅ `AUDIT_READINESS.md` + `MAINNET_CHECKLIST.md`

---

## 🎯 REMAINING (mainnet blockers)

| Item | Status |
|------|--------|
| Deploy & stabilize public testnet 30+ days | ❌ Terraform/Ansible ready; not deployed |
| Professional security audit | ❌ Prep docs done; audit not started |
| Bug bounty program | ❌ |
| Real KZG / Halo2 ZK circuits | ❌ SHA-256 commitments remain |
| Production Slack/PagerDuty alerting | ⚠️ Local Alertmanager only |
| EVM persistent state (full MPT) | ❌ In-memory between restarts |
| MEV Shamir over GF(2^8) library | ⚠️ AES-GCM done; Lagrange still hand-rolled |

---

## Quick Commands

```bash
cargo build -p node --release
cargo test -p storage pruner

cd tests/localhost/scripts
node 12-vote-proposal.js
node 18-register-validator.js

bash deployment/cloud/scripts/deploy-public-testnet.sh
```

See `MAINNET_CHECKLIST.md` for launch criteria.
