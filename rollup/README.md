# Rollup

Rollup node logic — both Optimistic and ZK rollup variants.

## Components

| Component | Description |
|---|---|
| `RollupNode` | Core node: EVM executor + optional ZK prover + DA layer |
| `Batch` | Transaction batch with `prev_state_root`, `new_state_root`, optional ZK proof, optional DA commitment |
| `RollupType` | `Optimistic` (fraud proofs) or `ZkRollup` (validity proofs) |
| `FraudProof` / `FraudVerifier` | Fraud proof generation and verification via re-execution (**re-execution stubbed** in current code) |

> **Status:** Rollup logic exists as a library; not integrated into the main node binary. See [WHATS_LEFT.md](../WHATS_LEFT.md).
| `CrossRollupMessage` / `RollupBridge` | Inter-rollup message queuing and proof verification |
