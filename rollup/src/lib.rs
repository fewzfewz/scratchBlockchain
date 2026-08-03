use anyhow::Result;
use common::types::{Transaction, Address};
use execution::EvmExecutor;
use serde::{Deserialize, Serialize};
use zk::ZkProver;
use da::DataAvailability;
use std::sync::{Arc, Mutex};
use tracing::info;
use std::collections::{HashMap, VecDeque};

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
                use execution::evm::{SignedTransaction, EvmExecutor};
                use revm::primitives::{Address, U256};

                let mut executor = EvmExecutor::new();
                for tx in &batch.transactions {
                    let stx = SignedTransaction::new(
                        Address::from_slice(&tx.sender),
                        tx.to.map(|a| Address::from_slice(&a)),
                        U256::from(tx.value),
                        tx.payload.clone(),
                        tx.nonce,
                        tx.gas_limit,
                        U256::from(tx.max_fee_per_gas),
                        tx.chain_id.unwrap_or(1),
                    );
                    executor.execute_transaction(stx)?;
                }
                Ok(true)
            }
        }
    }

    pub fn generate_fraud_proof(&self, batch_index: u64, tx_index: usize) -> FraudProof {
        let batch = &self.l1_batches[batch_index as usize];
        FraudProof {
            batch_index,
            tx_index,
            invalid_state_root: [0u8; 32],
            correct_state_root: [0u8; 32],
            evidence: vec![],
            challenger: [0u8; 20],
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

// ============================================================================
// Fraud Proof System
// ============================================================================

/// Fraud proof with evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudProof {
    pub batch_index: u64,
    pub tx_index: usize,
    pub invalid_state_root: [u8; 32],
    pub correct_state_root: [u8; 32],
    pub evidence: Vec<u8>,
    pub challenger: Address,
}

/// Fraud proof verifier
pub struct FraudVerifier {
    verified_proofs: HashMap<u64, bool>,
}

impl FraudVerifier {
    pub fn new() -> Self {
        Self {
            verified_proofs: HashMap::new(),
        }
    }
    
    /// Verify fraud proof by re-executing
    pub fn verify_fraud_proof(&mut self, rollup: &RollupNode, proof: &FraudProof) -> Result<bool, String> {
        let batch = rollup.l1_batches.get(proof.batch_index as usize)
            .ok_or("Batch not found")?;
        
        let tx = batch.transactions.get(proof.tx_index)
            .ok_or("Transaction not found")?;
        
        // Re-execute from previous state
        let mut state = batch.prev_state_root.clone();
        
        // Execute all transactions up to the invalid one
        for i in 0..=proof.tx_index {
            let current_tx = &batch.transactions[i];
            state = self.execute_transaction(rollup, &state, current_tx)?;
        }
        
        // Check if state matches expected
        let expected_root = self.hash_state(&state);
        let is_fraud = expected_root != proof.invalid_state_root;
        
        if is_fraud {
            self.verified_proofs.insert(proof.batch_index, true);
            info!("Fraud proven for batch {}", proof.batch_index);
        }
        
        Ok(is_fraud)
    }
    
    fn execute_transaction(&self, rollup: &RollupNode, _state: &[u8], tx: &Transaction) -> Result<Vec<u8>, String> {
        use execution::evm::{SignedTransaction, EvmExecutor};
        use revm::primitives::{Address, Bytes, U256};

        let mut executor = EvmExecutor::new();
        let stx = SignedTransaction::new(
            Address::from_slice(&tx.sender),
            tx.to.map(|a| Address::from_slice(&a)),
            U256::from(tx.value),
            Bytes::from(tx.payload.clone()),
            tx.nonce,
            tx.gas_limit,
            U256::from(tx.max_fee_per_gas),
            tx.chain_id.unwrap_or(1),
        );
        executor
            .execute_transaction(stx)
            .map_err(|e| e.to_string())?;
        Ok(executor.state_root().to_vec())
    }

    fn hash_state(&self, state: &[u8]) -> [u8; 32] {
        if state.len() == 32 {
            let mut root = [0u8; 32];
            root.copy_from_slice(state);
            return root;
        }
        let mut hasher = Sha256::new();
        hasher.update(state);
        hasher.finalize().into()
    }
}

// ============================================================================
// Cross-Rollup Communication
// ============================================================================

/// Message between rollups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossRollupMessage {
    pub from_rollup: String,
    pub to_rollup: String,
    pub sender: Address,
    pub recipient: Address,
    pub data: Vec<u8>,
    pub value: u128,
    pub nonce: u64,
    pub proof: Option<Vec<u8>>,
}

/// Cross-rollup messaging bridge
pub struct RollupBridge {
    outgoing_messages: VecDeque<CrossRollupMessage>,
    incoming_messages: VecDeque<CrossRollupMessage>,
    processed_nonces: HashMap<String, u64>,
}

impl RollupBridge {
    pub fn new() -> Self {
        Self {
            outgoing_messages: VecDeque::new(),
            incoming_messages: VecDeque::new(),
            processed_nonces: HashMap::new(),
        }
    }
    
    /// Send a message to another rollup
    pub fn send_message(&mut self, message: CrossRollupMessage) -> Result<(), String> {
        // Verify nonce is sequential
        let last_nonce = self.processed_nonces.get(&message.from_rollup).unwrap_or(&0);
        if message.nonce != *last_nonce + 1 {
            return Err("Invalid nonce".into());
        }
        
        self.outgoing_messages.push_back(message);
        Ok(())
    }
    
    /// Receive a message from another rollup
    pub fn receive_message(&mut self, message: CrossRollupMessage) -> Result<(), String> {
        // Verify proof if ZK rollup
        if let Some(proof) = &message.proof {
            // Verify proof using ZK prover
            if !self.verify_message_proof(&message, proof) {
                return Err("Invalid proof".into());
            }
        }
        
        self.incoming_messages.push_back(message);
        Ok(())
    }
    
    /// Execute pending incoming messages
    pub fn execute_messages(&mut self, executor: &mut EvmExecutor) -> Result<usize, String> {
        let mut executed = 0;
        
        while let Some(message) = self.incoming_messages.pop_front() {
            // Execute message on EVM
            let _result = executor.execute_message(&message)?;
            self.processed_nonces.insert(message.to_rollup, message.nonce);
            executed += 1;
        }
        
        Ok(executed)
    }
    
    fn verify_message_proof(&self, _message: &CrossRollupMessage, _proof: &[u8]) -> bool {
        // In production, verify ZK proof
        true
    }
}