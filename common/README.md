# Common

Core types, traits, cryptographic primitives, and shared utilities used by every crate in the blockchain.

## Modules

| File | Contents |
|---|---|
| `src/lib.rs` | Re-exports all public modules |
| `src/types.rs` | `Hash`, `Address`, `Signature`, `Header`, `Transaction`, `Block`, `Account`, `GenesisConfig`, `TransactionReceipt`, `ExecutionStatus` |
| `src/traits.rs` | Core traits: `Consensus`, `Storage`, `Executor`, `Mempool`, `State`, `BlockProducer`, `Validator` |
| `src/crypto.rs` | Ed25519 signing/verification, SHA-256 hashing, address derivation, hex helpers |
| `src/consensus_types.rs` | `Step`, `Vote`, `Proposal`, `ConsensusMessage`, `ValidatorInfo` |
| `src/merkle.rs` | `MerkleTree`, `MerkleProof`, `MultiProof` with generation, verification, batch verification |
| `src/validation.rs` | `TransactionValidator` with EIP-1559 gas checks, chain-ID replay protection, EIP-2718 access lists |
