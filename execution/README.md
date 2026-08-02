# Execution

Transaction and smart-contract execution engine. Supports EVM (revm), WASM scaffold, native transfers, and parallel execution.

## Components

| Module | Description |
|---|---|
| `src/lib.rs` | `NativeExecutor`, `WasmExecutor` (placeholder), `ParallelExecutor` (Rayon-based) |
| `src/evm.rs` | Full EVM via `revm` 3.5 with `EvmStore` persistence trait, `EvmDb`, `SignedTransaction`, `EvmExecutor` |
| `src/account_abstraction.rs` | ERC-4337-style AA: `UserOperation`, `Bundler`, `AccountAbstractionExecutor` — **wired via node `TxPool` + `POST /submit_user_operation`** |
| `src/gas.rs` | EIP-1559 base fee oracle, `GasMeter`, opcode gas costs |

## Node Integration

- **EVM**: `BlockExecutor` runs transactions through `EvmExecutor` on finalize
- **Account abstraction**: `AccountAbstractionExecutor` in `node/src/tx_pool.rs`; bundled ops become transactions at block production
- **Gas estimation**: `POST /estimate_gas` uses EIP-2028 data cost + 10% buffer

## Known Limitations

- `WasmExecutor` is a scaffold (wasmtime loads modules but not production-ready for contracts)
- EVM state is not fully synced to Patricia trie for state roots (node uses deterministic header hash)
