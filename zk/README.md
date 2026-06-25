# ZK — Zero-Knowledge Proofs

Zero-knowledge proof generation and verification for state transitions.

## Components

| Component | Description |
|---|---|
| `ZkProver` | `prove()` / `verify()` — SHA-256 based simulated proofs; `prove_state_transition()` hashes `prev_root \|\| new_root \|\| tx_hash` |
| `ProofAggregator` | Aggregates multiple proofs into one with Merkle-like root commitment |
| `BatchVerifier` | Rayon-based parallel batch verification |
| `halo2_backend` | Placeholder module for future Halo2 zk-SNARK circuit implementation (gated behind `halo2` feature) |

## Feature Flags

- `halo2` — enables `halo2_proofs` 0.3 and `halo2curves` 0.1 (backend currently empty)

## Notes

In default mode, proofs are SHA-256 hash-based simulations, not cryptographically secure for production. Proof caching uses a `HashMap<u64, Vec<u8>>`.
