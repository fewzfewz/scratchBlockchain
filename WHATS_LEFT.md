# What's Left - Development Status

## Current Status: **PRE-MAINNET 100%** (code-complete)

### Last Updated: August 3, 2026

All **pre-mainnet software work is complete**. Remaining items are **mainnet launch** operations (deploy, audit, economics ceremony) — not missing features.

---

## ✅ COMPLETE — Pre-Mainnet Checklist

### Node & consensus
- BFT consensus with 2/3 finality
- Dynamic validator register + **hot-reload** from state trie
- Slashing tracker + adversarial tests
- State pruning (archive/full/minimal) + Patricia trie GC

### Execution
- EVM (revm) with **persistent** `ChainStoreEvmStore`
- WASM (wasmtime fuel metering, `/deploy_wasm`, `/call_wasm`)
- EIP-1559 gas, parallel rayon executor
- Account abstraction (`/submit_user_operation`)

### MEV / ZK / DA
- MEV commit-reveal + threshold encryption (sharks + AES-GCM)
- DA Reed–Solomon + Merkle commitments
- ZK proof binding (SHA-256; Halo2 stub for future)

### Frontend (10 routes)
- Wallet + **tx history**, faucet, explorer, governance, deploy, validators/onboard, docs, API docs, SDK portal

### RPC (32 HTTP + WebSocket)
- Core, staking, governance, MEV, AA, WASM, faucet, slashing
- OpenAPI synced (`docs/openapi.yaml`)
- `/status` returns `chain_id`

### SDK (`@modular-blockchain/sdk`)
- HttpProvider aligned to all node routes
- 40+ client methods (tx history, WASM, faucet, validators, MEV, AA)
- CI build/test + npm publish workflow

### Testing & ops
- 15+ integration scripts + `run-all-tests.sh`
- CI: Rust + SDK + Docker integration job
- Grafana/Prometheus/Alertmanager (local + prod template)
- Public testnet deploy script + audit prep docs

---

## 🎯 MAINNET LAUNCH ONLY (not pre-mainnet code)

| Item | Owner | Notes |
|------|-------|-------|
| Deploy public testnet 30+ days | DevOps | `deployment/cloud/scripts/deploy-public-testnet.sh` |
| Professional security audit | Security | `AUDIT_READINESS.md` |
| Bug bounty | Security | Post-audit (Immunefi) |
| Full KZG/Halo2 ZK circuits | Crypto | Merkle + SHA-256 sufficient for testnet |
| Prod Slack/PagerDuty | DevOps | `alertmanager.prod.yml.example` |
| Mainnet genesis ceremony | Governance | `tools/genesis-builder/examples/mainnet.toml` |
| Multi-region RPC + TLS + DDoS | DevOps | `deployment/cloud/` |

See `MAINNET_CHECKLIST.md` for go/no-go criteria.

---

## Quick Commands

```bash
cargo build -p node --release
cd frontend && npm run dev
cd sdk/javascript && npm run build && npm test
cd tests/localhost && bash run-all-tests.sh
bash deployment/cloud/scripts/deploy-public-testnet.sh
```
