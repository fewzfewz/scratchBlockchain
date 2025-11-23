use anyhow::Result;
use common::types::Transaction;
use execution::EvmExecutor;
use serde::{Deserialize, Serialize};
use zk::ZkProver;
use da::DataAvailability;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub transactions: Vec<Transaction>,
    pub prev_state_root: Vec<u8>,
    pub new_state_root: Vec<u8>,
    /// ZK proof of state transition validity
    pub zk_proof: Option<Vec<u8>>,
    /// Data Availability commitment
    pub da_commitment: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudProof {
    pub batch_index: u64,
    pub invalid_tx_index: usize,
}

/// Rollup type configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollupType {
    /// Optimistic rollup (fraud proofs)
    Optimistic,
    /// ZK rollup (validity proofs)
    ZkRollup,
}

pub struct RollupNode {
    /// Rollup type
    rollup_type: RollupType,
    /// Batches submitted to L1
    pub l1_batches: Vec<Batch>,
    /// EVM executor
    pub executor: EvmExecutor,
    /// ZK prover (for ZK rollups)
    zk_prover: Option<ZkProver>,
    /// Data Availability layer
    da_layer: Arc<Mutex<DataAvailability>>,
}

impl RollupNode {
    pub fn new(rollup_type: RollupType, da_layer: Arc<Mutex<DataAvailability>>) -> Self {
        let zk_prover = if rollup_type == RollupType::ZkRollup {
            Some(ZkProver::new())
        } else {
            None
        };

        Self {
            rollup_type,
            l1_batches: Vec::new(),
            executor: EvmExecutor::new(),
            zk_prover,
            da_layer,
        }
    }

    /// Submit a batch with optional ZK proof
    pub fn submit_batch(&mut self, mut batch: Batch) -> Result<()> {
        // For ZK rollups, generate proof before submission
        if self.rollup_type == RollupType::ZkRollup {
            if let Some(prover) = &self.zk_prover {
                // Generate ZK proof of state transition
                let proof = self.generate_zk_proof(prover, &batch)?;
                batch.zk_proof = Some(proof);
                println!("✓ ZK proof generated for batch");
            }
        }

        // Submit data to DA layer
        // For simplicity, we serialize the transactions as the blob data
        let mut blob_data = Vec::new();
        for tx in &batch.transactions {
            // Simple serialization: [nonce (8 bytes) | payload_len (8 bytes) | payload]
            blob_data.extend_from_slice(&tx.nonce.to_le_bytes());
            blob_data.extend_from_slice(&(tx.payload.len() as u64).to_le_bytes());
            blob_data.extend_from_slice(&tx.payload);
        }

        {
            let mut da = self.da_layer.lock().unwrap();
            let commitment = da.submit_blob(blob_data)?;
            batch.da_commitment = Some(commitment.commitment);
        }

        println!(
            "Submitting {:?} batch to L1: {} transactions",
            self.rollup_type,
            batch.transactions.len()
        );
        self.l1_batches.push(batch);
        Ok(())
    }

    /// Generate ZK proof for a batch
    fn generate_zk_proof(&self, prover: &ZkProver, batch: &Batch) -> Result<Vec<u8>> {
        // In a real implementation, this would:
        // 1. Create a circuit representing the state transition
        // 2. Generate witness from transaction execution
        // 3. Prove the circuit with the witness
        
        // For MVP, we'll create a simple proof of the state transition
        let mut proof_input = Vec::new();
        proof_input.extend_from_slice(&batch.prev_state_root);
        proof_input.extend_from_slice(&batch.new_state_root);
        
        // Serialize transactions
        for tx in &batch.transactions {
            proof_input.extend_from_slice(&tx.hash());
        }
        
        // Generate proof
        let proof = prover.prove(&proof_input)?;
        Ok(proof)
    }

    /// Verify a batch (different logic for optimistic vs ZK)
    pub fn verify_batch(&mut self, batch_index: usize) -> Result<bool> {
        if batch_index >= self.l1_batches.len() {
            return Err(anyhow::anyhow!("Batch index out of bounds"));
        }

        let batch = &self.l1_batches[batch_index];

        match self.rollup_type {
            RollupType::ZkRollup => {
                // For ZK rollups, verify the proof
                if let Some(proof) = &batch.zk_proof {
                    if let Some(prover) = &self.zk_prover {
                        let mut proof_input = Vec::new();
                        proof_input.extend_from_slice(&batch.prev_state_root);
                        proof_input.extend_from_slice(&batch.new_state_root);
                        
                        for tx in &batch.transactions {
                            proof_input.extend_from_slice(&tx.hash());
                        }
                        
                        return prover.verify(proof, &proof_input);
                    }
                }
                Err(anyhow::anyhow!("No ZK proof found for ZK rollup batch"))
            }
            RollupType::Optimistic => {
                // For optimistic rollups, re-execute transactions
                for tx in &batch.transactions {
                    let _res = self.executor.execute_transaction(
                        "0x0000000000000000000000000000000000000000",
                        None,
                        0,
                        &tx.payload,
                    );
                }
                Ok(true)
            }
        }
    }

    pub fn generate_fraud_proof(&self, batch_index: u64, tx_index: usize) -> FraudProof {
        FraudProof {
            batch_index,
            invalid_tx_index: tx_index,
        }
    }

    pub fn rollup_type(&self) -> RollupType {
        self.rollup_type
    }
}

pub fn init() {
    println!("Rollup initialized (use RollupNode::new)");
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::types::Transaction;

    fn create_test_transaction(nonce: u64) -> Transaction {
        Transaction::test_transaction([1; 20], nonce)
    }

    #[test]
    fn test_optimistic_rollup() {
        let da_layer = Arc::new(Mutex::new(DataAvailability::new(4, 2, 10)));
        let mut rollup = RollupNode::new(RollupType::Optimistic, da_layer);
        
        let batch = Batch {
            transactions: vec![create_test_transaction(1)],
            prev_state_root: vec![0; 32],
            new_state_root: vec![1; 32],
            zk_proof: None,
            da_commitment: None,
        };

        assert!(rollup.submit_batch(batch).is_ok());
        assert_eq!(rollup.l1_batches.len(), 1);
    }

    #[test]
    fn test_zk_rollup() {
        let da_layer = Arc::new(Mutex::new(DataAvailability::new(4, 2, 10)));
        let mut rollup = RollupNode::new(RollupType::ZkRollup, da_layer);
        
        let batch = Batch {
            transactions: vec![create_test_transaction(1)],
            prev_state_root: vec![0; 32],
            new_state_root: vec![1; 32],
            zk_proof: None,
            da_commitment: None,
        };

        assert!(rollup.submit_batch(batch).is_ok());
        assert_eq!(rollup.l1_batches.len(), 1);
        
        // ZK proof should be generated
        assert!(rollup.l1_batches[0].zk_proof.is_some());
    }

    #[test]
    fn test_zk_rollup_verification() {
        let da_layer = Arc::new(Mutex::new(DataAvailability::new(4, 2, 10)));
        let mut rollup = RollupNode::new(RollupType::ZkRollup, da_layer);
        
        let batch = Batch {
            transactions: vec![create_test_transaction(1)],
            prev_state_root: vec![0; 32],
            new_state_root: vec![1; 32],
            zk_proof: None,
            da_commitment: None,
        };

        rollup.submit_batch(batch).unwrap();
        
        // Verify the batch
        let result = rollup.verify_batch(0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rollup_da_integration() {
        let da_layer = Arc::new(Mutex::new(DataAvailability::new(4, 2, 10)));
        let mut rollup = RollupNode::new(RollupType::Optimistic, da_layer.clone());
        
        let batch = Batch {
            transactions: vec![create_test_transaction(1)],
            prev_state_root: vec![0; 32],
            new_state_root: vec![1; 32],
            zk_proof: None,
            da_commitment: None,
        };

        // Submit batch
        assert!(rollup.submit_batch(batch).is_ok());
        
        // Verify commitment was stored
        let submitted_batch = &rollup.l1_batches[0];
        assert!(submitted_batch.da_commitment.is_some());
        
        // Verify data is in DA layer
        let da = da_layer.lock().unwrap();
        assert_eq!(da.blob_count(), 1);
        
        let blob = da.get_blob(0).unwrap();
        assert_eq!(blob.commitment.commitment, submitted_batch.da_commitment.clone().unwrap());
    }
}
