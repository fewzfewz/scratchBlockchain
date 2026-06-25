# Examples

End-to-end walkthrough demonstrating how the modular blockchain layers work together.

## Files

- `full_demo.rs` — Instantiates and exercises storage (`MemStore`), consensus (`SimpleConsensus`), execution (`WasmExecutor`, `ParallelExecutor`, `EvmExecutor`), rollup (`RollupNode`), cross-chain messaging (`Router`), and governance (`Governance`).

## Notes

Purely illustrative — not built by default. Not a Cargo workspace member (no `Cargo.toml`).
