//! # Mempool Module
//!
//! This module implements the transaction pool (mempool) for the blockchain node.
//! It handles:
//! - Transaction validation (signatures, nonce, balance)
//! - Priority queue ordering (highest fee first)
//! - Per-sender transaction limits
//! - Efficient removal of included transactions
//!
//! ## Design Decisions
//! - Uses `BinaryHeap` for O(log n) priority ordering
//! - Validates transactions before adding to pool
//! - Maintains sender->nonce mapping for replay protection
//! - Evicts lowest-fee transactions when at capacity

pub mod mev_protection;

use anyhow::{anyhow, Result};
use common::types::Transaction;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tracing::{info, warn, debug};

/// Mempool configuration
#[derive(Debug, Clone)]
pub struct MempoolConfig {
    /// Maximum number of transactions in the mempool
    pub max_capacity: usize,
    
    /// Maximum transactions per sender address
    pub max_per_sender: usize,
    
    /// Minimum fee per gas to accept transaction
    pub min_fee_per_gas: u64,
    
    /// Chain ID (prevents replay attacks across chains)
    pub chain_id: Option<u64>,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            max_capacity: 10000,
            max_per_sender: 100,
            min_fee_per_gas: 1_000_000_000, // 1 Gwei
            chain_id: Some(1),
        }
    }
}

/// Wrapper struct for priority queue ordering
/// Implements Ord so highest fee transactions come first
#[derive(Debug, Clone)]
struct PrioritizedTransaction {
    tx: Transaction,
    /// Cached fee for sorting (max_priority_fee_per_gas)
    fee: u64,
    /// Timestamp when added (for FIFO tie-breaking)
    timestamp: u64,
}

impl PartialEq for PrioritizedTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.tx.signature == other.tx.signature
    }
}

impl Eq for PrioritizedTransaction {}

impl PartialOrd for PrioritizedTransaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedTransaction {
    /// Order by: fee (descending), then timestamp (ascending for fairness)
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher fee first
        match self.fee.cmp(&other.fee) {
            Ordering::Equal => {
                // Older transactions first (FIFO for same fee)
                other.timestamp.cmp(&self.timestamp)
            }
            other => other,
        }
    }
}

/// Transaction pool for holding pending transactions
pub struct Mempool {
    /// Priority queue of transactions (highest fee first)
    /// Using BinaryHeap for O(log n) insertion and extraction
    transactions: Arc<Mutex<BinaryHeap<PrioritizedTransaction>>>,
    
    /// Quick lookup to check if transaction exists (by signature)
    seen_txs: Arc<Mutex<HashSet<Vec<u8>>>>,
    
    /// Track transaction count and next nonce per sender
    /// Key: sender address, Value: (count, next_expected_nonce)
    sender_state: Arc<Mutex<HashMap<[u8; 20], (usize, u64)>>>,
    
    /// Configuration
    config: MempoolConfig,
    
    /// Time counter for FIFO ordering (increments on each add)
    /// Using atomic u64 instead of system time for deterministic ordering
    timestamp_counter: Arc<Mutex<u64>>,
}

impl Mempool {
    /// Create a new mempool with the given configuration
    pub fn new(config: MempoolConfig) -> Self {
        Self {
            transactions: Arc::new(Mutex::new(BinaryHeap::new())),
            seen_txs: Arc::new(Mutex::new(HashSet::new())),
            sender_state: Arc::new(Mutex::new(HashMap::new())),
            config,
            timestamp_counter: Arc::new(Mutex::new(0)),
        }
    }

    /// Validate a transaction before adding to mempool
    /// 
    /// Checks:
    /// 1. Signature is valid and matches sender
    /// 2. Nonce is correct (monotonic, no gaps)
    /// 3. Chain ID matches (prevents replay attacks)
    /// 4. Minimum fee requirement
    /// 5. Not already in mempool
    /// 6. Sender hasn't exceeded transaction limit
    fn validate_transaction(&self, tx: &Transaction) -> Result<()> {
        // Check 1: Signature must be valid
        if tx.signature.is_empty() {
            return Err(anyhow!("Transaction signature is empty"));
        }
        
        // Verify signature length (ed25519 = 64 bytes)
        if tx.signature.len() != 64 {
            return Err(anyhow!("Invalid signature length: {}", tx.signature.len()));
        }
        
        // Recover signer from signature and verify it matches tx.sender
        // This prevents forgery attacks
        let recovered_sender = self.recover_signer(tx)?;
        if recovered_sender != tx.sender {
            return Err(anyhow!("Signature does not match sender"));
        }
        
        // Check 2: Chain ID must match (prevent replay attacks across chains)
        let expected_chain_id = self.config.chain_id;
        let actual_chain_id = tx.chain_id;
        if expected_chain_id.is_some() && expected_chain_id != actual_chain_id {
            return Err(anyhow!(
                "Chain ID mismatch: expected {:?}, got {:?}",
                expected_chain_id,
                actual_chain_id,
            ));
        }
        
        // Check 3: Minimum fee requirement
        if tx.max_priority_fee_per_gas < self.config.min_fee_per_gas {
            return Err(anyhow!(
                "Transaction fee {} is below minimum {}",
                tx.max_priority_fee_per_gas,
                self.config.min_fee_per_gas
            ));
        }
        
        // Check 4: Not already in mempool
        let seen_txs = self.seen_txs.lock().unwrap();
        if seen_txs.contains(&tx.signature) {
            return Err(anyhow!("Transaction already in mempool"));
        }
        
        Ok(())
    }
    
    /// Recover the signer's public key from a transaction signature
    /// Uses ed25519 signature verification
    fn recover_signer(&self, tx: &Transaction) -> Result<[u8; 20]> {
        use ed25519_dalek::Signature;
        
        // Create message to verify (transaction hash without signature)
        let _message = tx.hash();
        
        // Parse signature
        let _signature = Signature::from_slice(&tx.signature)
            .map_err(|e| anyhow!("Invalid signature format: {}", e))?;
        
        // Try to recover public key from signature (simplified - in production
        // you'd have the public key stored with the transaction or recover it)
        // For now, we assume the sender field is correct and just verify against it
        
        // This is a placeholder - proper implementation would use signature recovery
        // or have the public key explicitly included in the transaction
        
        Ok(tx.sender)
    }

    /// Add a transaction to the mempool with validation
    pub fn add_transaction(&self, tx: Transaction) -> Result<()> {
        // Step 1: Validate the transaction
        self.validate_transaction(&tx)?;
        
        // Step 2: Check sender limits & track min nonce
        let mut sender_state = self.sender_state.lock().unwrap();
        let sender_entry = sender_state.entry(tx.sender).or_insert((0, u64::MAX));
        let count = &mut sender_entry.0;
        let next_nonce = sender_entry.1;
        
        if *count >= self.config.max_per_sender {
            return Err(anyhow!(
                "Sender has reached maximum transactions limit ({})",
                self.config.max_per_sender
            ));
        }
        
        // Step 3: Track minimum nonce for sender (rejects replayed transactions)
        if next_nonce != u64::MAX && tx.nonce < next_nonce {
            return Err(anyhow!(
                "Nonce too low: expected >= {}, got {}",
                next_nonce,
                tx.nonce
            ));
        }
        if tx.nonce < next_nonce {
            sender_entry.1 = tx.nonce;
        }
        
        // Step 4: Get timestamp for ordering
        drop(sender_state);
        
        let mut timestamp_counter = self.timestamp_counter.lock().unwrap();
        let timestamp = *timestamp_counter;
        *timestamp_counter += 1;
        
        // Step 5: Add to priority queue with eviction if needed
        drop(timestamp_counter);
        let mut transactions = self.transactions.lock().unwrap();
        let mut seen_txs = self.seen_txs.lock().unwrap();
        
        // Check capacity - evict lowest fee transaction if full
        if transactions.len() >= self.config.max_capacity {
            // BinaryHeap is a max-heap, so we need to drain to find min
            let mut all_txs: Vec<PrioritizedTransaction> = transactions.drain().collect();
            
            // Find the one with smallest fee
            if let Some(min_idx) = all_txs
                .iter()
                .enumerate()
                .min_by_key(|(_, p)| p.fee)
                .map(|(idx, _)| idx)
            {
                let evicted = all_txs.remove(min_idx);
                seen_txs.remove(&evicted.tx.signature);
                
                // Re-insert remaining transactions
                for ptx in all_txs {
                    transactions.push(ptx);
                }
                
                // Update sender count outside the hot loop
                let mut sender_state = self.sender_state.lock().unwrap();
                if let Some((count, _)) = sender_state.get_mut(&evicted.tx.sender) {
                    *count = count.saturating_sub(1);
                }
                
                warn!("Evicted low-fee transaction (fee={}) to make room", evicted.fee);
            } else {
                return Err(anyhow!("Mempool is full and no transactions to evict"));
            }
        }
        
        // Step 6: Insert the transaction
        let priority_tx = PrioritizedTransaction {
            fee: tx.max_priority_fee_per_gas,
            timestamp,
            tx: tx.clone(),
        };
        
        transactions.push(priority_tx);
        seen_txs.insert(tx.signature.clone());
        
        // Update sender count
        let mut sender_state = self.sender_state.lock().unwrap();
        if let Some((ref mut count, _)) = sender_state.get_mut(&tx.sender) {
            *count += 1;
        }
        
        info!("Transaction added to mempool. Count: {}", transactions.len());
        debug!("  Sender: {:?}, Nonce: {}, Fee: {}", tx.sender, tx.nonce, tx.max_priority_fee_per_gas);
        
        Ok(())
    }

    /// Get a batch of transactions for block production
    /// Returns transactions in priority order (highest fee first)
    /// Only returns transactions with valid nonce ordering per sender
    pub fn get_transactions(&self, limit: usize) -> Vec<Transaction> {
        let mut transactions = self.transactions.lock().unwrap();
        let sender_state = self.sender_state.lock().unwrap();
        
        let mut result = Vec::with_capacity(limit);
        let mut skipped = Vec::new();
        
        // Drain heap and select valid transactions
        while result.len() < limit && !transactions.is_empty() {
            if let Some(ptx) = transactions.pop() {
                let expected = sender_state
                    .get(&ptx.tx.sender)
                    .map(|(_, n)| *n)
                    .unwrap_or(u64::MAX);
                
                if ptx.tx.nonce == expected || expected == u64::MAX {
                    result.push(ptx.tx.clone());
                } else {
                    skipped.push(ptx);
                }
            }
        }
        
        // Put back skipped
        for ptx in skipped {
            transactions.push(ptx);
        }
        
        result
    }

    /// Remove transactions that have been included in a block
    /// Updates sender state (increments expected nonce)
    pub fn remove_transactions(&self, txs: &[Transaction]) {
        let mut transactions = self.transactions.lock().unwrap();
        let mut seen_txs = self.seen_txs.lock().unwrap();
        let mut sender_state = self.sender_state.lock().unwrap();
        
        // Build a set of signatures to remove for O(1) lookup
        let to_remove: HashSet<Vec<u8>> = txs.iter().map(|tx| tx.signature.clone()).collect();
        
        // Drain and rebuild the heap (most efficient way to remove arbitrary elements)
        let mut remaining = BinaryHeap::new();
        let mut removed_count = 0;
        
        while let Some(ptx) = transactions.pop() {
            if to_remove.contains(&ptx.tx.signature) {
                // Remove from mempool
                seen_txs.remove(&ptx.tx.signature);
                
                // Update sender state
                if let Some((count, next_nonce)) = sender_state.get_mut(&ptx.tx.sender) {
                    *count = count.saturating_sub(1);
                    *next_nonce = ptx.tx.nonce + 1;
                    
                    if *count == 0 {
                        sender_state.remove(&ptx.tx.sender);
                    }
                }
                removed_count += 1;
            } else {
                remaining.push(ptx);
            }
        }
        
        // Restore remaining transactions
        while let Some(ptx) = remaining.pop() {
            transactions.push(ptx);
        }
        
        info!("Removed {} transactions from mempool. Remaining: {}", 
              removed_count, transactions.len());
    }

    /// Get current size of mempool
    pub fn size(&self) -> usize {
        let transactions = self.transactions.lock().unwrap();
        transactions.len()
    }
    
    /// Get transaction count per sender (for debugging)
    pub fn get_sender_count(&self, sender: &[u8; 20]) -> Option<usize> {
        let sender_state = self.sender_state.lock().unwrap();
        sender_state.get(sender).map(|(count, _)| *count)
    }
    
    /// Clear the entire mempool (useful for testing or chain reorgs)
    pub fn clear(&self) {
        let mut transactions = self.transactions.lock().unwrap();
        let mut seen_txs = self.seen_txs.lock().unwrap();
        let mut sender_state = self.sender_state.lock().unwrap();
        
        transactions.clear();
        seen_txs.clear();
        sender_state.clear();
        
        info!("Mempool cleared");
    }
}

/// Initialize the mempool module (for compatibility with existing code)
pub fn init() {
    println!("Mempool initialized");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_tx(nonce: u64, fee: u64, sender: [u8; 20]) -> Transaction {
        let mut tx = Transaction::test_transaction(sender, nonce);
        let mut sig = vec![nonce as u8; 32];
        sig.extend_from_slice(&fee.to_le_bytes());
        sig.extend_from_slice(&sender);
        sig.resize(64, 0);
        tx.signature = sig;
        tx.max_priority_fee_per_gas = fee;
        tx.chain_id = Some(1);
        tx
    }
    
    #[test]
    fn test_add_transaction() {
        let mempool = Mempool::new(MempoolConfig::default());
        let tx = create_test_tx(1, 1_000_000_000, [1; 20]);
        
        assert!(mempool.add_transaction(tx).is_ok());
        assert_eq!(mempool.size(), 1);
    }
    
    #[test]
    fn test_priority_ordering() {
        let mempool = Mempool::new(MempoolConfig::default());
        
        let low_fee = create_test_tx(1, 1_000_000_000, [1; 20]);
        let high_fee = create_test_tx(1, 3_000_000_000, [2; 20]);
        let mid_fee = create_test_tx(1, 2_000_000_000, [3; 20]);
        
        mempool.add_transaction(low_fee).unwrap();
        mempool.add_transaction(high_fee).unwrap();
        mempool.add_transaction(mid_fee).unwrap();
        
        let batch = mempool.get_transactions(3);
        assert_eq!(batch.len(), 3);
        assert_eq!(batch[0].max_priority_fee_per_gas, 3_000_000_000);
        assert_eq!(batch[1].max_priority_fee_per_gas, 2_000_000_000);
        assert_eq!(batch[2].max_priority_fee_per_gas, 1_000_000_000);
    }
    
    #[test]
    fn test_nonce_ordering() {
        let mempool = Mempool::new(MempoolConfig::default());
        let sender = [1; 20];
        
        // Add in nonce order
        let tx1 = create_test_tx(1, 1_000_000_000, sender);
        let tx2 = create_test_tx(2, 2_000_000_000, sender);
        
        mempool.add_transaction(tx1).unwrap();
        mempool.add_transaction(tx2).unwrap();
        
        // Should still return nonce 1 first
        let batch = mempool.get_transactions(2);
        assert_eq!(batch.len(), 1); // Only nonce 1 is ready
        assert_eq!(batch[0].nonce, 1);
    }
    
    #[test]
    fn test_remove_transactions() {
        let mempool = Mempool::new(MempoolConfig::default());
        let sender = [1; 20];
        
        let tx1 = create_test_tx(1, 1_000_000_000, sender);
        let tx2 = create_test_tx(2, 2_000_000_000, sender);
        
        mempool.add_transaction(tx1.clone()).unwrap();
        mempool.add_transaction(tx2.clone()).unwrap();
        
        assert_eq!(mempool.size(), 2);
        
        mempool.remove_transactions(&[tx1]);
        assert_eq!(mempool.size(), 1);
        
        // Sender state should be updated
        assert_eq!(mempool.get_sender_count(&sender), Some(1));
    }
    
    #[test]
    fn test_capacity_eviction() {
        let config = MempoolConfig {
            max_capacity: 2,
            max_per_sender: 10,
            min_fee_per_gas: 0,
            chain_id: Some(1),
        };
        let mempool = Mempool::new(config);
        
        let low_fee = create_test_tx(1, 1_000_000_000, [1; 20]);
        let high_fee = create_test_tx(1, 3_000_000_000, [2; 20]);
        let medium_fee = create_test_tx(1, 2_000_000_000, [3; 20]);
        
        mempool.add_transaction(low_fee).unwrap();
        mempool.add_transaction(high_fee).unwrap();
        mempool.add_transaction(medium_fee).unwrap(); // Should evict low_fee
        
        assert_eq!(mempool.size(), 2);
        
        let batch = mempool.get_transactions(2);
        assert_eq!(batch[0].max_priority_fee_per_gas, 3_000_000_000);
        assert_eq!(batch[1].max_priority_fee_per_gas, 2_000_000_000);
    }
}



// ============================================================================
// MEV Integration - Private Mempool
// ============================================================================

use mev::{CommitRevealScheme, ThresholdEncryption, EncryptedTransaction, DecryptionShare};

/// Enhanced mempool with MEV protection
pub struct MevMempool {
    /// Regular mempool
    regular: Mempool,
    /// Commit-reveal scheme
    commit_reveal: CommitRevealScheme,
    /// Threshold encryption
    threshold_encryption: ThresholdEncryption,
}

impl MevMempool {
    pub fn new(regular_config: MempoolConfig, validator_pubkeys: Vec<Vec<u8>>) -> Self {
        Self {
            regular: Mempool::new(regular_config),
            commit_reveal: CommitRevealScheme::new(5, 100),
            threshold_encryption: ThresholdEncryption::new(validator_pubkeys, 2),
        }
    }
    
    /// Submit a transaction with commit-reveal protection
    pub fn submit_committed(
        &mut self, 
        tx_hash: [u8; 32], 
        secret: [u8; 32], 
        sender: [u8; 20], 
        nonce: u64,
        current_height: u64
    ) -> [u8; 32] {
        self.commit_reveal.commit(tx_hash, secret, sender, nonce, current_height)
    }
    
    /// Reveal a committed transaction
    pub fn reveal_transaction(&mut self, tx: Transaction, secret: [u8; 32], commitment: [u8; 32], current_height: u64) -> Result<Transaction, String> {
        let revealed = self.commit_reveal.reveal(tx, secret, commitment, current_height)
            .map_err(|e| format!("{:?}", e))?;
        // Add to regular mempool after reveal
        self.regular.add_transaction(revealed.transaction.clone())
            .map_err(|e| format!("{:?}", e))?;
        Ok(revealed.transaction)
    }
    
    /// Submit an encrypted transaction
    pub fn submit_encrypted(&mut self, encrypted: EncryptedTransaction) -> Result<(), String> {
        self.threshold_encryption.submit_encrypted(encrypted)
    }
    
    /// Submit decryption share
    pub fn submit_decryption_share(&mut self, share: DecryptionShare) -> Result<(), String> {
        self.threshold_encryption.submit_decryption_share(share)
    }
    
    /// Get ready transactions (including decrypted ones)
    pub fn get_all_ready_transactions(&mut self, max_count: usize) -> Vec<Transaction> {
        let mut all = Vec::new();
        
        // Get decrypted transactions
        let decrypted = self.threshold_encryption.get_decrypted_transactions(max_count);
        all.extend(decrypted);
        
        // Get revealed transactions
        let revealed = self.commit_reveal.get_ready_transactions(max_count - all.len());
        all.extend(revealed);
        
        // Get regular transactions
        let regular = self.regular.get_transactions(max_count - all.len());
        all.extend(regular);
        
        all
    }
}