# Security Audit Readiness

Checklist for engaging a third-party auditor before mainnet launch.

## Scope for audit

| Area | Path | Priority |
|------|------|----------|
| BFT consensus | `consensus/src/bft.rs` | Critical |
| Block execution | `node/src/block_producer.rs`, `execution/` | Critical |
| Cryptography | `common/src/crypto`, `mev/`, `da/`, `zk/` | Critical |
| RPC surface | `node/src/rpc.rs` | High |
| Governance | `governance/`, `node/src/governance_store.rs` | High |
| Bridge | `interop/` | High |
| Storage / pruning | `storage/` | Medium |

## Pre-audit checklist

- [x] Unified tx pool with MEV + AA integration
- [x] BFT validator hot-reload from state trie
- [x] State pruning + trie orphan GC
- [x] Integration tests (governance, staking, bridge readiness)
- [x] Load + chaos smoke tests
- [x] Prometheus alerts on live `blockchain_*` metrics
- [ ] Full fuzz coverage on RPC deserialization
- [ ] External penetration test on public RPC
- [ ] Formal verification of consensus (optional)

## Known limitations (disclose to auditors)

1. **ZK proofs** — SHA-256 commitments; Halo2 feature stub only
2. **KZG commitments** — SHA-256 in DA layer, not real KZG
3. **MEV Shamir** — Integer Lagrange over bytes; AES-GCM at rest (upgraded from XOR)
4. **EVM state** — In-memory between restarts; not full persistent MPT
5. **RPC registration** — `POST /validators/register` writes trie directly (dev convenience)

## Recommended audit firms

Engage firms with blockchain + Rust experience. Typical timeline: 4–8 weeks.

## Bug bounty (post-audit)

Launch on Immunefi or HackerOne after critical findings are resolved. Suggested tiers:

| Severity | Bounty |
|----------|--------|
| Critical (consensus break, fund loss) | $10k–$50k |
| High (RPC DoS, slashing bypass) | $2k–$10k |
| Medium | $500–$2k |

## Documentation pack for auditors

Provide:

- `README.md`, `WHATS_LEFT.md`, `STATUS.md`
- `docs/openapi.yaml` (RPC spec)
- `genesis.json` + local docker-compose repro steps
- This file + `MAINNET_CHECKLIST.md`
