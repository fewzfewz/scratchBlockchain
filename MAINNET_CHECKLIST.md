# Mainnet Launch Checklist

Use this after public testnet is stable and audit findings are resolved.

## Phase 1 — Testnet hardening (current)

- [x] Local 3-validator Docker testnet
- [x] 29 RPC endpoints + WebSocket
- [x] Frontend wallet, faucet, explorer, governance, deploy
- [x] State pruning (full/minimal/archive)
- [x] Validator onboarding UI + Grafana dashboard
- [x] Alertmanager wired (local)
- [x] Integration test suite (governance execute, stake, load, chaos, bridge unlock queue)
- [ ] Public testnet deployed and stable 30+ days
- [ ] Full integration script suite green in CI (23+ scripts; advanced stress tests planned)

## Phase 2 — Security

- [ ] Professional security audit completed
- [ ] All critical/high findings remediated
- [ ] Bug bounty program live
- [ ] Incident response runbook documented
- [ ] Multi-sig treasury keys (hardware wallets)

## Phase 3 — Infrastructure

- [ ] Multi-region validator deployment
- [ ] RPC load balancer + DDoS protection (Cloudflare/AWS Shield)
- [ ] TLS on all public endpoints
- [ ] Automated backups + disaster recovery tested
- [ ] Production Alertmanager (Slack + PagerDuty)
- [ ] State pruning tuned for archive nodes vs validators

## Phase 4 — Governance & economics

- [ ] Genesis mainnet allocation finalized
- [ ] Validator minimum stake enforced on-chain
- [ ] Slashing conditions tested under adversarial scenarios
- [ ] Fee burn + treasury parameters ratified via governance

## Phase 5 — Launch

- [ ] Mainnet genesis ceremony (multiparty)
- [ ] Public RPC endpoints announced
- [ ] Block explorer indexed
- [ ] Wallet / SDK published to npm
- [ ] 72-hour war room monitoring post-launch

## Go / no-go criteria

**Go** when all Phase 2 items are done and Phase 3 RPC/TLS/alerting are live.

**No-go** if any open critical audit finding or consensus divergence in public testnet.
