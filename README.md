# Modular Blockchain Architecture

A comprehensive, modular blockchain implementation in Rust featuring multi-VM execution, ZK proofs, optimistic rollups, and cross-chain messaging.

## 🏗️ Architecture Overview

This project implements a layered blockchain architecture with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────┐
│                    Node (CLI)                           │
├─────────────────────────────────────────────────────────┤
│  Governance  │  Interop  │  Rollup  │  ZK Prover       │
├─────────────────────────────────────────────────────────┤
│           Execution (WASM + EVM + Parallel)             │
├─────────────────────────────────────────────────────────┤
│    Consensus (PoA)    │    Storage (KV)                 │
├─────────────────────────────────────────────────────────┤
│              Network (libp2p)                           │
├─────────────────────────────────────────────────────────┤
│              Common (Types & Traits)                    │
└─────────────────────────────────────────────────────────┘
```

## 📦 Crates

### Core Layer
- **`common`**: Shared types (`Block`, `Transaction`, `Header`) and traits (`Consensus`, `Storage`, `Executor`)
- **`network`**: P2P networking using libp2p with Gossipsub and Kademlia DHT
- **`consensus`**: Proof of Authority (PoA) consensus mechanism
- **`storage`**: In-memory key-value store implementing generic `Storage` trait
- **`node`**: Main binary with CLI for node management

### Execution Layer
- **`execution`**: Multi-VM execution environment
  - WASM runtime (wasmtime)
  - EVM compatibility (revm)
  - Parallel execution engine (rayon)

### L2 & ZK Layer
- **`zk`**: Zero-knowledge prover infrastructure using halo2
- **`rollup`**: Optimistic rollup with batch management and fraud proofs

### Interoperability & Governance
- **`interop`**: Cross-chain message router with ed25519 signatures
- **`governance`**: On-chain governance with proposals and voting

## 🚀 Quick Start

### Prerequisites
- Rust (latest stable)
- `protoc` (Protocol Buffers compiler for libp2p)

### Build
```bash
cargo build --release
```

### Run Node
```bash
# Start the node
cargo run --bin node -- start

# Generate a keypair
cargo run --bin node -- key-gen
```

### Run Tests
```bash
cargo test --workspace
```

## 🔧 Development

### Project Structure
```
.
├── common/          # Shared types and traits
├── network/         # P2P networking
├── consensus/       # Consensus mechanism
├── storage/         # Data storage
├── execution/       # Multi-VM execution
├── zk/             # ZK prover
├── rollup/         # L2 rollup
├── interop/        # Cross-chain messaging
├── governance/     # On-chain governance
└── node/           # Main binary
```

### Key Design Decisions

1. **Modularity**: Each component is a separate crate with well-defined interfaces
2. **Trait-based**: Core functionality defined through traits for easy swapping
3. **Multi-VM**: Support for both WASM and EVM execution environments
4. **L2-Ready**: Built-in support for optimistic rollups and ZK proofs
5. **Interoperable**: Cross-chain message routing with cryptographic verification

## 🎯 Features

### Implemented
- ✅ P2P networking with peer discovery
- ✅ Proof of Authority consensus
- ✅ In-memory storage (ready for persistent DB)
- ✅ WASM runtime integration
- ✅ EVM compatibility
- ✅ Parallel transaction execution
- ✅ ZK prover infrastructure
- ✅ Optimistic rollup support
- ✅ Cross-chain messaging
- ✅ On-chain governance

### Roadmap
- 🔲 Persistent storage (RocksDB integration)
- 🔲 Advanced consensus (GRANDPA/BABE)
- 🔲 Full ZK rollup implementation
- 🔲 Light client support
- 🔲 Multi-node testnet
- 🔲 Production hardening & audits

## 📚 Documentation

- [Walkthrough](../brain/05a0e82e-975f-40cf-8a31-b0ead6bdb8d9/walkthrough.md): Detailed feature overview
- [Task Roadmap](../brain/05a0e82e-975f-40cf-8a31-b0ead6bdb8d9/task.md): Development progress

## 🤝 Contributing

This is a reference implementation demonstrating modular blockchain architecture. Contributions are welcome!

## 📄 License

MIT License - see LICENSE file for details

## 🔗 Key Dependencies

- **libp2p**: P2P networking
- **wasmtime**: WASM runtime
- **revm**: Rust EVM implementation
- **halo2**: ZK proof system
- **ed25519-dalek**: Cryptographic signatures
- **serde**: Serialization
- **tokio**: Async runtime
