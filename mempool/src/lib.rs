pub mod mev_protection;

use anyhow::{anyhow, Result};
use common::types::Transaction;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

/// Mempool configuration
#[derive(Debug, Clone)]
pub struct MempoolConfig {
    /// Maximum number of transactions in the mempool
    pub max_capacity: usize,
    /// Maximum transactions per sender address
    pub max_per_sender: usize,
    /// Minimum fee per gas to accept transaction
    pub min_fee_per_gas: u64,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            max_capacity: 10000,
            max_per_sender: 100,
            min_fee_per_gas: 1_000_000_000, // 1 Gwei
        }
    }
}

/// Transaction pool for holding pending transactions
#[derive(Debug, Clone)]
pub struct Mempool {
    /// Thread-safe storage for transactions
    /// Using a simple VecDeque for FIFO ordering for now
    /// In production, this would be a priority queue based on fees/nonce
    transactions: Arc<Mutex<VecDeque<Transaction>>>,
    /// Quick lookup to check if transaction exists (by signature/hash)
    /// For this MVP, we'll use signature as a unique ID
    seen_txs: Arc<Mutex<HashMap<Vec<u8>, ()>>>,
    /// Track transaction count per sender
    sender_counts: Arc<Mutex<HashMap<[u8; 20], usize>>>,
    config: MempoolConfig,
}

impl Mempool {
    pub fn new(config: MempoolConfig) -> Self {
        Self {
            transactions: Arc::new(Mutex::new(VecDeque::new())),
            seen_txs: Arc::new(Mutex::new(HashMap::new())),
            sender_counts: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Add a transaction to the mempool
    pub fn add_transaction(&self, tx: Transaction) -> Result<()> {
        // Basic validation
        if tx.signature.is_empty() {
            return Err(anyhow!("Transaction signature is empty"));
        }

        // Validate minimum fee
        if tx.max_priority_fee_per_gas < self.config.min_fee_per_gas {
            return Err(anyhow!(
                "Transaction fee {} is below minimum {}",
                tx.max_priority_fee_per_gas,
                self.config.min_fee_per_gas
            ));
        }

        let mut transactions = self.transactions.lock().unwrap();
        let mut seen_txs = self.seen_txs.lock().unwrap();
        let mut sender_counts = self.sender_counts.lock().unwrap();

        // Check for duplicates
        if seen_txs.contains_key(&tx.signature) {
            return Err(anyhow!("Transaction already in mempool"));
        }

        // Check sender limit
        let sender_count = sender_counts.get(&tx.sender).copied().unwrap_or(0);
        if sender_count >= self.config.max_per_sender {
            return Err(anyhow!(
                "Sender has reached maximum transactions limit ({})",
                self.config.max_per_sender
            ));
        }

        // Check capacity - evict lowest fee transaction if full
        if transactions.len() >= self.config.max_capacity {
            // Find and remove lowest fee transaction
            if let Some((idx, _)) = transactions
                .iter()
                .enumerate()
                .min_by_key(|(_, t)| t.max_priority_fee_per_gas)
            {
                if let Some(removed_tx) = transactions.remove(idx) {
                    seen_txs.remove(&removed_tx.signature);
                    if let Some(count) = sender_counts.get_mut(&removed_tx.sender) {
                        *count = count.saturating_sub(1);
                    }
                    info!("Evicted low-fee transaction to make room");
                }
            } else {
                return Err(anyhow!("Mempool is full and no transactions to evict"));
            }
        }

        // Add to pool
        seen_txs.insert(tx.signature.clone(), ());
        *sender_counts.entry(tx.sender).or_insert(0) += 1;
        transactions.push_back(tx);
        
        info!("Transaction added to mempool. Count: {}", transactions.len());

        Ok(())
    }

    /// Get a batch of transactions for block production
    pub fn get_transactions(&self, limit: usize) -> Vec<Transaction> {
        let transactions = self.transactions.lock().unwrap();
        
        // Sort by priority fee (descending)
        let mut txs: Vec<Transaction> = transactions.iter().cloned().collect();
        txs.sort_by(|a, b| b.max_priority_fee_per_gas.cmp(&a.max_priority_fee_per_gas));
        
        txs.into_iter().take(limit).collect()
    }

    /// Remove transactions that have been included in a block
    pub fn remove_transactions(&self, txs: &[Transaction]) {
        let mut transactions = self.transactions.lock().unwrap();
        let mut seen_txs = self.seen_txs.lock().unwrap();
        let mut sender_counts = self.sender_counts.lock().unwrap();

        for tx in txs {
            // Remove from seen set
            seen_txs.remove(&tx.signature);
            
            // Update sender count
            if let Some(count) = sender_counts.get_mut(&tx.sender) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    sender_counts.remove(&tx.sender);
                }
            }
            
            // Remove from queue (inefficient for VecDeque, but fine for MVP)
            // In production, use a better data structure
            if let Some(pos) = transactions.iter().position(|t| t.signature == tx.signature) {
                transactions.remove(pos);
            }
        }
        
        info!("Removed {} transactions from mempool. Remaining: {}", txs.len(), transactions.len());
    }

    /// Get current size of mempool
    pub fn size(&self) -> usize {
        let transactions = self.transactions.lock().unwrap();
        transactions.len()
    }
}

pub fn init() {
    println!("Mempool initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_dummy_tx(nonce: u64) -> Transaction {
        let mut tx = Transaction::test_transaction([1; 20], nonce);
        // Set a unique signature based on nonce to avoid "seen_txs" collision
        tx.signature = vec![nonce as u8; 64];
        tx
    }

    #[test]
    fn test_add_transaction() {
        let mempool = Mempool::new(MempoolConfig::default());
        let tx = create_dummy_tx(1);

        assert!(mempool.add_transaction(tx).is_ok());
        assert_eq!(mempool.size(), 1);
    }

    #[test]
    fn test_duplicate_transaction() {
        let mempool = Mempool::new(MempoolConfig::default());
        let tx = create_dummy_tx(1);

        mempool.add_transaction(tx.clone()).unwrap();
        assert!(mempool.add_transaction(tx).is_err());
    }

    #[test]
    fn test_capacity_limit() {
        let config = MempoolConfig { 
            max_capacity: 2,
            max_per_sender: 100,
            min_fee_per_gas: 0,
        };
        let mempool = Mempool::new(config);

        mempool.add_transaction(create_dummy_tx(1)).unwrap();
        mempool.add_transaction(create_dummy_tx(2)).unwrap();
        
        // Should succeed by evicting lowest fee transaction
        mempool.add_transaction(create_dummy_tx(3)).unwrap();
        assert_eq!(mempool.size(), 2); // Still 2, but tx1 was evicted
    }

    #[test]
    fn test_get_and_remove_transactions() {
        let mempool = Mempool::new(MempoolConfig::default());
        
        let tx1 = create_dummy_tx(1);
        let tx2 = create_dummy_tx(2);
        let tx3 = create_dummy_tx(3);

        mempool.add_transaction(tx1.clone()).unwrap();
        mempool.add_transaction(tx2.clone()).unwrap();
        mempool.add_transaction(tx3.clone()).unwrap();

        let batch = mempool.get_transactions(2);
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].nonce, 1);
        assert_eq!(batch[1].nonce, 2);

        // Remove them
        mempool.remove_transactions(&batch);
        assert_eq!(mempool.size(), 1);
        
        let remaining = mempool.get_transactions(10);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].nonce, 3);
    }

    #[test]
    fn test_transaction_sorting() {
        let mempool = Mempool::new(MempoolConfig::default());
        
        let mut tx1 = create_dummy_tx(1);
        tx1.max_priority_fee_per_gas = 1_000_000_000;
        
        let mut tx2 = create_dummy_tx(2);
        tx2.max_priority_fee_per_gas = 3_000_000_000;
        
        let mut tx3 = create_dummy_tx(3);
        tx3.max_priority_fee_per_gas = 2_000_000_000;

        mempool.add_transaction(tx1).unwrap();
        mempool.add_transaction(tx2).unwrap();
        mempool.add_transaction(tx3).unwrap();

        let batch = mempool.get_transactions(3);
        assert_eq!(batch.len(), 3);
        assert_eq!(batch[0].max_priority_fee_per_gas, 3_000_000_000);
        assert_eq!(batch[1].max_priority_fee_per_gas, 2_000_000_000);
        assert_eq!(batch[2].max_priority_fee_per_gas, 1_000_000_000);
    }    
}
