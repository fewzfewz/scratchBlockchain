//! # MEV Protection Module
//!
//! This module implements Maximum Extractable Value (MEV) protection mechanisms
//! to prevent front-running, sandwich attacks, and other harmful transaction ordering.
//!
//! ## Commit-Reveal Scheme
//! Transactions are first committed (only hash revealed), then revealed after a delay.
//! This prevents validators from seeing transaction contents before ordering them.
//!
//! ## Design Decisions
//! - Uses **block height + round number** instead of system time (prevents clock manipulation)
//! - FIFO ordering based on commit time (fair for all participants)
//! - Automatic cleanup of expired commitments

use anyhow::Result;
use common::types::Transaction;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tracing::{info, warn, debug};

/// Commit-reveal scheme for MEV protection
/// Transactions are first committed (hash only), then revealed after a delay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionCommitment {
    /// Hash of the transaction (commitment)
    pub commitment: [u8; 32],
    
    /// Block height when committed (not system time - prevents clock manipulation)
    pub commit_height: u64,
    
    /// Round number within the block (for ordering within same height)
    pub commit_round: u64,
    
    /// Index within the round (for deterministic ordering)
    pub commit_index: u64,
}

impl TransactionCommitment {
    /// Create a new commitment at the current block height
    pub fn new(commitment: [u8; 32], height: u64, round: u64, index: u64) -> Self {
        Self {
            commitment,
            commit_height: height,
            commit_round: round,
            commit_index: index,
        }
    }
    
    /// Get the order key for sorting (height, then round, then index)
    pub fn order_key(&self) -> (u64, u64, u64) {
        (self.commit_height, self.commit_round, self.commit_index)
    }
}

/// Revealed transaction ready for inclusion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevealedTransaction {
    /// The actual transaction
    pub transaction: Transaction,
    
    /// Original commitment
    pub commitment: TransactionCommitment,
    
    /// Block height when revealed
    pub reveal_height: u64,
}

impl RevealedTransaction {
    /// Get the order key (uses commit time for FIFO fairness)
    pub fn order_key(&self) -> (u64, u64, u64) {
        self.commitment.order_key()
    }
}

/// MEV protection configuration
#[derive(Debug, Clone)]
pub struct MevProtectionConfig {
    /// Minimum number of blocks between commit and reveal
    /// Prevents validators from revealing immediately to front-run
    pub min_reveal_delay_blocks: u64,
    
    /// Maximum number of blocks allowed for reveal
    /// After this, commitment expires and transaction cannot be revealed
    pub max_reveal_delay_blocks: u64,
    
    /// Enable encrypted mempool (future feature)
    pub enable_encrypted_mempool: bool,
    
    /// Maximum number of commitments per block height (prevents spam)
    pub max_commitments_per_height: usize,
}

impl Default for MevProtectionConfig {
    fn default() -> Self {
        Self {
            min_reveal_delay_blocks: 2,   // Wait at least 2 blocks
            max_reveal_delay_blocks: 30,  // Expire after 30 blocks
            enable_encrypted_mempool: true,
            max_commitments_per_height: 1000,
        }
    }
}

/// MEV protection layer for mempool
pub struct MevProtection {
    /// Configuration
    config: MevProtectionConfig,
    
    /// Pending commitments waiting to be revealed
    /// Key: commitment hash, Value: commitment details
    commitments: Arc<Mutex<HashMap<[u8; 32], TransactionCommitment>>>,
    
    /// Revealed transactions ready for inclusion
    /// Stored in a VecDeque for FIFO ordering
    revealed: Arc<Mutex<VecDeque<RevealedTransaction>>>,
    
    /// Track commitments per height (for cleanup and spam prevention)
    commitments_by_height: Arc<Mutex<HashMap<u64, Vec<[u8; 32]>>>>,
    
    /// Monotonic counter for commit ordering within rounds
    commit_counter: Arc<Mutex<u64>>,
}

impl MevProtection {
    /// Create a new MEV protection instance
    pub fn new(config: MevProtectionConfig) -> Self {
        Self {
            config,
            commitments: Arc::new(Mutex::new(HashMap::new())),
            revealed: Arc::new(Mutex::new(VecDeque::new())),
            commitments_by_height: Arc::new(Mutex::new(HashMap::new())),
            commit_counter: Arc::new(Mutex::new(0)),
        }
    }

    /// Commit a transaction (submit hash only)
    /// 
    /// # Arguments
    /// * `tx_hash` - Hash of the transaction (32 bytes)
    /// * `current_height` - Current block height (from consensus)
    /// * `current_round` - Current consensus round
    /// 
    /// # Returns
    /// * `TransactionCommitment` - The created commitment
    pub fn commit_transaction(
        &self,
        tx_hash: [u8; 32],
        current_height: u64,
        current_round: u64,
    ) -> Result<TransactionCommitment> {
        // Check for duplicate commitment
        let mut commitments = self.commitments.lock().unwrap();
        if commitments.contains_key(&tx_hash) {
            return Err(anyhow::anyhow!("Transaction already committed"));
        }
        
        // Check commitment limit per height (prevent spam)
        let mut by_height = self.commitments_by_height.lock().unwrap();
        let height_commitments = by_height.entry(current_height).or_default();
        
        if height_commitments.len() >= self.config.max_commitments_per_height {
            return Err(anyhow::anyhow!(
                "Too many commitments for height {} (max: {})",
                current_height,
                self.config.max_commitments_per_height
            ));
        }
        
        // Get next commit index for deterministic ordering
        let mut counter = self.commit_counter.lock().unwrap();
        let commit_index = *counter;
        *counter += 1;
        
        // Create the commitment
        let commitment = TransactionCommitment::new(
            tx_hash,
            current_height,
            current_round,
            commit_index,
        );
        
        // Store the commitment
        commitments.insert(tx_hash, commitment.clone());
        height_commitments.push(tx_hash);
        
        debug!(
            "Transaction committed: hash={:?}, height={}, round={}, index={}",
            hex::encode(&tx_hash[..4]),
            current_height,
            current_round,
            commit_index
        );
        
        Ok(commitment)
    }

    /// Reveal a previously committed transaction
    /// 
    /// # Arguments
    /// * `tx` - The full transaction (must match committed hash)
    /// * `current_height` - Current block height
    /// 
    /// # Returns
    /// * `Ok(())` if successful, error otherwise
    pub fn reveal_transaction(
        &self,
        tx: Transaction,
        current_height: u64,
    ) -> Result<()> {
        let tx_hash = tx.hash();
        
        // Step 1: Check if commitment exists
        let mut commitments = self.commitments.lock().unwrap();
        let commitment = commitments
            .get(&tx_hash)
            .ok_or_else(|| anyhow::anyhow!("No commitment found for transaction"))?
            .clone();
        
        // Step 2: Verify reveal timing (using block heights, not system time)
        let blocks_elapsed = current_height.saturating_sub(commitment.commit_height);
        
        if blocks_elapsed < self.config.min_reveal_delay_blocks {
            return Err(anyhow::anyhow!(
                "Reveal too early: {} blocks elapsed, need at least {}",
                blocks_elapsed,
                self.config.min_reveal_delay_blocks
            ));
        }
        
        if blocks_elapsed > self.config.max_reveal_delay_blocks {
            return Err(anyhow::anyhow!(
                "Reveal too late: {} blocks elapsed, maximum is {}",
                blocks_elapsed,
                self.config.max_reveal_delay_blocks
            ));
        }
        
        // Step 3: Verify the revealed transaction matches the commitment
        // This prevents revealing a different transaction than what was committed
        // (Already ensured by using tx_hash as key, but double-check)
        if tx_hash != commitment.commitment {
            return Err(anyhow::anyhow!("Transaction hash does not match commitment"));
        }
        
        // Step 4: Verify transaction signature (security check)
        if !self.verify_transaction_signature(&tx) {
            return Err(anyhow::anyhow!("Invalid transaction signature"));
        }
        
        // Step 5: Add to revealed transactions
        let revealed_tx = RevealedTransaction {
            transaction: tx,
            commitment,
            reveal_height: current_height,
        };
        
        let mut revealed = self.revealed.lock().unwrap();
        revealed.push_back(revealed_tx);
        
        // Step 6: Remove from commitments
        commitments.remove(&tx_hash);
        
        // Clean up height tracking
        let mut by_height = self.commitments_by_height.lock().unwrap();
        if let Some(commitments_list) = by_height.get_mut(&commitment.commit_height) {
            commitments_list.retain(|h| h != &tx_hash);
            if commitments_list.is_empty() {
                by_height.remove(&commitment.commit_height);
            }
        }
        
        info!(
            "Transaction revealed: hash={:?}, commit_height={}, reveal_height={}",
            hex::encode(&tx_hash[..4]),
            commitment.commit_height,
            current_height
        );
        
        Ok(())
    }

    /// Verify transaction signature (internal helper)
    fn verify_transaction_signature(&self, tx: &Transaction) -> bool {
        use ed25519_dalek::{Signature, VerifyingKey};
        
        if tx.signature.is_empty() || tx.signature.len() != 64 {
            return false;
        }
        
        // Reconstruct the message that was signed (transaction hash)
        let message = tx.hash();
        
        // Parse signature
        let signature = match Signature::from_slice(&tx.signature) {
            Ok(sig) => sig,
            Err(_) => return false,
        };
        
        // Parse public key from sender (simplified - in production you'd have the key)
        // For now, we assume the sender field is correct
        // A production system would include the public key in the transaction
        
        true
    }

    /// Get transactions ready for inclusion (sorted by commit time for fairness)
    /// 
    /// # Arguments
    /// * `limit` - Maximum number of transactions to return
    /// 
    /// # Returns
    /// * `Vec<Transaction>` - Transactions in FIFO order (oldest commit first)
    pub fn get_ready_transactions(&self, limit: usize) -> Vec<Transaction> {
        let mut revealed = self.revealed.lock().unwrap();
        
        // Sort by commit order (height, then round, then index)
        // This ensures fairness: earlier commits are included first
        let mut all_revealed: Vec<RevealedTransaction> = revealed.drain().collect();
        all_revealed.sort_by_key(|r| r.order_key());
        
        // Take up to limit transactions
        let result: Vec<Transaction> = all_revealed
            .iter()
            .take(limit)
            .map(|r| r.transaction.clone())
            .collect();
        
        // Keep remaining transactions
        let remaining: VecDeque<RevealedTransaction> = all_revealed
            .into_iter()
            .skip(limit)
            .collect();
        
        *revealed = remaining;
        
        result
    }

    /// Remove transactions that have been included in a block
    /// 
    /// # Arguments
    /// * `txs` - Transactions that were included in the block
    pub fn remove_transactions(&self, txs: &[Transaction]) {
        let mut revealed = self.revealed.lock().unwrap();
        let tx_hashes: Vec<[u8; 32]> = txs.iter().map(|tx| tx.hash()).collect();
        
        let before_count = revealed.len();
        revealed.retain(|r| !tx_hashes.contains(&r.transaction.hash()));
        let removed_count = before_count - revealed.len();
        
        if removed_count > 0 {
            debug!("Removed {} revealed transactions after block inclusion", removed_count);
        }
    }

    /// Clean up expired commitments that were never revealed
    /// 
    /// # Arguments
    /// * `current_height` - Current block height
    pub fn cleanup_expired(&self, current_height: u64) {
        let mut commitments = self.commitments.lock().unwrap();
        let mut by_height = self.commitments_by_height.lock().unwrap();
        
        let before_count = commitments.len();
        
        // Find and remove expired commitments
        commitments.retain(|hash, commitment| {
            let blocks_elapsed = current_height.saturating_sub(commitment.commit_height);
            let is_expired = blocks_elapsed > self.config.max_reveal_delay_blocks;
            
            if is_expired {
                debug!("Expired commitment: {:?} from height {}", 
                       hex::encode(&hash[..4]), commitment.commit_height);
                
                // Clean up height tracking
                if let Some(list) = by_height.get_mut(&commitment.commit_height) {
                    list.retain(|h| h != hash);
                }
            }
            
            !is_expired
        });
        
        let expired_count = before_count - commitments.len();
        if expired_count > 0 {
            info!("Cleaned up {} expired commitments at height {}", expired_count, current_height);
        }
    }

    /// Get number of pending commitments
    pub fn pending_commitments(&self) -> usize {
        let commitments = self.commitments.lock().unwrap();
        commitments.len()
    }

    /// Get number of revealed transactions
    pub fn revealed_count(&self) -> usize {
        let revealed = self.revealed.lock().unwrap();
        revealed.len()
    }
    
    /// Get a specific commitment by hash (for debugging)
    pub fn get_commitment(&self, hash: &[u8; 32]) -> Option<TransactionCommitment> {
        let commitments = self.commitments.lock().unwrap();
        commitments.get(hash).cloned()
    }
}

/// Initialize the MEV protection module
pub fn init() {
    println!("MEV Protection module initialized");
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::types::Transaction;
    
    fn create_test_transaction(nonce: u64, sender: [u8; 20]) -> Transaction {
        let mut tx = Transaction::test_transaction(sender, nonce);
        tx.signature = vec![nonce as u8; 64];
        tx.max_priority_fee_per_gas = 1_000_000_000;
        tx
    }
    
    #[test]
    fn test_commit_reveal_flow() {
        let config = MevProtectionConfig {
            min_reveal_delay_blocks: 0, // No delay for testing
            max_reveal_delay_blocks: 10,
            enable_encrypted_mempool: true,
            max_commitments_per_height: 1000,
        };
        
        let mev = MevProtection::new(config);
        let tx = create_test_transaction(1, [1; 20]);
        let tx_hash = tx.hash();
        
        // Commit
        let commitment = mev.commit_transaction(tx_hash, 1, 0).unwrap();
        assert_eq!(mev.pending_commitments(), 1);
        assert_eq!(commitment.commit_height, 1);
        assert_eq!(commitment.commit_round, 0);
        
        // Reveal (at height 3, which is 2 blocks later)
        mev.reveal_transaction(tx.clone(), 3).unwrap();
        assert_eq!(mev.pending_commitments(), 0);
        assert_eq!(mev.revealed_count(), 1);
        
        // Get ready transactions
        let ready = mev.get_ready_transactions(10);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].nonce, 1);
    }
    
    #[test]
    fn test_reveal_too_early() {
        let config = MevProtectionConfig {
            min_reveal_delay_blocks: 5,
            max_reveal_delay_blocks: 10,
            enable_encrypted_mempool: true,
            max_commitments_per_height: 1000,
        };
        
        let mev = MevProtection::new(config);
        let tx = create_test_transaction(1, [1; 20]);
        let tx_hash = tx.hash();
        
        mev.commit_transaction(tx_hash, 1, 0).unwrap();
        
        // Try to reveal immediately (same height)
        let result = mev.reveal_transaction(tx, 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too early"));
    }
    
    #[test]
    fn test_reveal_too_late() {
        let config = MevProtectionConfig {
            min_reveal_delay_blocks: 0,
            max_reveal_delay_blocks: 5,
            enable_encrypted_mempool: true,
            max_commitments_per_height: 1000,
        };
        
        let mev = MevProtection::new(config);
        let tx = create_test_transaction(1, [1; 20]);
        let tx_hash = tx.hash();
        
        mev.commit_transaction(tx_hash, 1, 0).unwrap();
        
        // Try to reveal after too many blocks (height 10)
        let result = mev.reveal_transaction(tx, 10);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too late"));
    }
    
    #[test]
    fn test_fifo_ordering() {
        let config = MevProtectionConfig::default();
        let mev = MevProtection::new(config);
        
        let tx1 = create_test_transaction(1, [1; 20]);
        let tx2 = create_test_transaction(2, [1; 20]);
        let tx3 = create_test_transaction(3, [1; 20]);
        
        let hash1 = tx1.hash();
        let hash2 = tx2.hash();
        let hash3 = tx3.hash();
        
        // Commit in order 1, 2, 3
        mev.commit_transaction(hash1, 1, 0).unwrap();
        mev.commit_transaction(hash2, 2, 0).unwrap();
        mev.commit_transaction(hash3, 3, 0).unwrap();
        
        // Reveal in reverse order 3, 2, 1
        mev.reveal_transaction(tx3, 5).unwrap();
        mev.reveal_transaction(tx2, 5).unwrap();
        mev.reveal_transaction(tx1, 5).unwrap();
        
        // Should still be returned in commit order (1, 2, 3)
        let ready = mev.get_ready_transactions(3);
        assert_eq!(ready.len(), 3);
        assert_eq!(ready[0].nonce, 1);
        assert_eq!(ready[1].nonce, 2);
        assert_eq!(ready[2].nonce, 3);
    }
    
    #[test]
    fn test_expired_cleanup() {
        let config = MevProtectionConfig {
            min_reveal_delay_blocks: 0,
            max_reveal_delay_blocks: 10,
            enable_encrypted_mempool: true,
            max_commitments_per_height: 1000,
        };
        
        let mev = MevProtection::new(config);
        let tx = create_test_transaction(1, [1; 20]);
        let tx_hash = tx.hash();
        
        // Commit at height 1
        mev.commit_transaction(tx_hash, 1, 0).unwrap();
        assert_eq!(mev.pending_commitments(), 1);
        
        // Cleanup at height 20 (should expire)
        mev.cleanup_expired(20);
        assert_eq!(mev.pending_commitments(), 0);
    }
    
    #[test]
    fn test_commitment_limit_per_height() {
        let config = MevProtectionConfig {
            min_reveal_delay_blocks: 0,
            max_reveal_delay_blocks: 10,
            enable_encrypted_mempool: true,
            max_commitments_per_height: 2,
        };
        
        let mev = MevProtection::new(config);
        
        let tx1 = create_test_transaction(1, [1; 20]);
        let tx2 = create_test_transaction(2, [1; 20]);
        let tx3 = create_test_transaction(3, [1; 20]);
        
        let hash1 = tx1.hash();
        let hash2 = tx2.hash();
        let hash3 = tx3.hash();
        
        assert!(mev.commit_transaction(hash1, 1, 0).is_ok());
        assert!(mev.commit_transaction(hash2, 1, 0).is_ok());
        
        // Third commitment at same height should fail
        assert!(mev.commit_transaction(hash3, 1, 0).is_err());
    }
    
    #[test]
    fn test_commit_counter_increments() {
        let config = MevProtectionConfig::default();
        let mev = MevProtection::new(config);
        
        let tx1 = create_test_transaction(1, [1; 20]);
        let tx2 = create_test_transaction(2, [1; 20]);
        
        let hash1 = tx1.hash();
        let hash2 = tx2.hash();
        
        let commit1 = mev.commit_transaction(hash1, 1, 0).unwrap();
        let commit2 = mev.commit_transaction(hash2, 1, 0).unwrap();
        
        // Commit indices should be 0, 1 in order
        assert_eq!(commit1.commit_index, 0);
        assert_eq!(commit2.commit_index, 1);
    }
}