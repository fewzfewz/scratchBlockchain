// Example demonstrating the modular blockchain architecture

use common::types::{Block, Header, Transaction};
use consensus::SimpleConsensus;
use storage::MemStore;
use execution::{WasmExecutor, ParallelExecutor, EvmExecutor};
use rollup::{Batch, RollupNode};
use interop::{Router, CrossChainMessage};
use governance::Governance;

fn main() {
    println!("=== Modular Blockchain Architecture Demo ===\n");

    // 1. Storage Layer
    println!("1. Storage Layer");
    let store = MemStore::new();
    println!("   ✓ In-memory storage initialized\n");

    // 2. Consensus Layer
    println!("2. Consensus Layer");
    let validators = vec![vec![1, 2, 3, 4]]; // Mock validator
    let consensus = SimpleConsensus::new(validators);
    println!("   ✓ PoA consensus initialized\n");

    // 3. Execution Layer
    println!("3. Execution Layer");
    let wasm_executor = WasmExecutor::new().expect("Failed to create WASM executor");
    let parallel_executor = ParallelExecutor::new();
    let mut evm_executor = EvmExecutor::new();
    println!("   ✓ WASM runtime ready");
    println!("   ✓ Parallel executor ready");
    println!("   ✓ EVM executor ready\n");

    // 4. EVM Example
    println!("4. EVM Execution Example");
    match evm_executor.execute_transaction(
        "0x0000000000000000000000000000000000000001",
        Some("0x0000000000000000000000000000000000000002"),
        100,
        &[],
    ) {
        Ok(_) => println!("   ✓ EVM transaction executed successfully\n"),
        Err(e) => println!("   ⚠ EVM transaction failed: {}\n", e),
    }

    // 5. Rollup Layer
    println!("5. Rollup Layer");
    let mut rollup = RollupNode::new();
    let batch = Batch {
        transactions: vec![],
        prev_state_root: vec![0; 32],
        new_state_root: vec![1; 32],
    };
    rollup.submit_batch(batch);
    println!("   ✓ Batch submitted to L2\n");

    // 6. Cross-Chain Messaging
    println!("6. Cross-Chain Messaging");
    let mut router_a = Router::new(1);
    let mut router_b = Router::new(2);
    
    router_a.send_message(2, b"Hello from Chain A!".to_vec())
        .expect("Failed to send message");
    println!("   ✓ Message sent from Chain 1 to Chain 2\n");

    // 7. Governance
    println!("7. Governance");
    let mut gov = Governance::new();
    let proposal_id = gov.create_proposal("Upgrade consensus to GRANDPA".to_string());
    gov.vote(proposal_id, true);
    gov.vote(proposal_id, true);
    gov.vote(proposal_id, false);
    
    if gov.tally_votes(proposal_id) {
        gov.execute_proposal(proposal_id);
    }
    println!("   ✓ Governance proposal lifecycle complete\n");

    println!("=== Demo Complete ===");
    println!("\nAll layers are functional and integrated!");
}
