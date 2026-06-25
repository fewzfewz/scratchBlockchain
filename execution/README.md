# Execution

Transaction and smart-contract execution engine. Supports EVM (revm), WASM, native transfers, and parallel execution.

## Components

| Module | Description |
|---|---|
| `src/lib.rs` | `NativeExecutor`, `WasmExecutor` (placeholder), `ParallelExecutor` (Rayon-based) |
| `src/evm.rs` | Full EVM via `revm` 3.5 with `EvmStore` persistence trait, `EvmDb`, `SignedTransaction`, `EvmExecutor` (block processing) |
| `src/account_abstraction.rs` | ERC-4337-style account abstraction: `UserOperation`, `Bundler`, `AccountAbstractionExecutor` |
| `src/gas.rs` | EIP-1559 base fee oracle, `GasMeter` (EIP-3529 refund capping), opcode gas costs |

## Features

- EVM backend with in-memory or persistent `EvmStore`
- WASM executor ready for future contract languages
- Parallel executor for non-conflicting transactions
- Account abstraction with user operation bundling
