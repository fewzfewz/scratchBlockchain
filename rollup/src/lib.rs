use anyhow::Result;
use common::types::Transaction;
use execution::EvmExecutor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub transactions: Vec<Transaction>,
    pub prev_state_root: Vec<u8>,
    pub new_state_root: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudProof {
    pub batch_index: u64,
    pub invalid_tx_index: usize,
}

pub struct RollupNode {
    // In a real implementation, this would connect to L1
    pub l1_batches: Vec<Batch>,
    pub executor: EvmExecutor,
}

impl RollupNode {
    pub fn new() -> Self {
        Self {
            l1_batches: Vec::new(),
            executor: EvmExecutor::new(),
        }
    }

    pub fn submit_batch(&mut self, batch: Batch) {
        // Mock L1 submission
        println!(
            "Submitting batch to L1: {} transactions",
            batch.transactions.len()
        );
        self.l1_batches.push(batch);
    }

    pub fn verify_batch(&mut self, batch_index: usize) -> Result<bool> {
        if batch_index >= self.l1_batches.len() {
            return Err(anyhow::anyhow!("Batch index out of bounds"));
        }

        let batch = &self.l1_batches[batch_index];

        // Re-execute transactions
        // Note: This is a simplified verification. Real rollup verification is more complex.
        for tx in &batch.transactions {
            // For MVP, we assume payload is just raw bytes for EVM
            // In reality, we'd parse it
            let _res = self.executor.execute_transaction(
                "0x0000000000000000000000000000000000000000", // Mock caller
                None,
                0,
                &tx.payload,
            );

            // If execution fails or state root mismatches, we'd generate a fraud proof
        }

        // Mock check: assume valid if we got here
        Ok(true)
    }

    pub fn generate_fraud_proof(&self, batch_index: u64, tx_index: usize) -> FraudProof {
        FraudProof {
            batch_index,
            invalid_tx_index: tx_index,
        }
    }
}

pub fn init() {
    println!("Rollup initialized (use RollupNode::new)");
}
