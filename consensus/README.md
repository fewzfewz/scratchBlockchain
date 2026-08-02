# Consensus

Hybrid BFT + GRANDPA-style finality gadget with slashing, view-change protocol, and validator management.

## Components

| File | Description |
|---|---|
| `src/bft.rs` | `BftEngine` — PBFT-style consensus with prevote/precommit/commit phases, 2/3 stake quorum, deterministic proposer rotation |
| `src/lib.rs` | `EnhancedConsensus`, `SimpleConsensus`, `FinalityGadget` (GRANDPA-style), `ViewChange` protocol |
| `src/slashing.rs` | `SlashingTracker`, `EvidenceCollector` — missed blocks, double-signing, liveness tracking, tombstones |

## Node integration

- `SlashingTracker` wired into node finalize loop (height tracking)
- `GET /slashing/events` RPC returns slashed validator public keys
- Equivocation detection in `FinalityGadget` (consensus layer)
| `fuzz/` | Property-based fuzz targets for finality votes and BFT rounds (1000 cases each via proptest) |
