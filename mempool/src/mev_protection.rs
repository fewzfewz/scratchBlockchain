use anyhow::Result;
use common::types::Transaction;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Commit-reveal scheme for MEV protection
/// Transactions are first committed (hash only), then revealed after a delay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionCommitment {
    /// Hash of the transaction
    pub commitment: [u8; 32],
    /// Timestamp when committed
    pub commit_time: u64,
    /// Block height when committed
    pub commit_height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevealedTransaction {
    /// The actual transaction
    pub transaction: Transaction,
    /// Original commitment
    pub commitment: TransactionCommitment,
    /// Timestamp when revealed
    pub reveal_time: u64,
}

/// MEV protection configuration
#[derive(Debug, Clone)]
pub struct MevProtectionConfig {
    /// Minimum time (seconds) between commit and reveal
    pub min_reveal_delay: u64,
    /// Maximum time (seconds) allowed for reveal
    pub max_reveal_delay: u64,
    /// Enable encrypted mempool
    pub enable_encrypted_mempool: bool,
}

impl Default for MevProtectionConfig {
    fn default() -> Self {
        Self {
            min_reveal_delay: 2,  // 2 seconds minimum
            max_reveal_delay: 30, // 30 seconds maximum
            enable_encrypted_mempool: true,
        }
    }
}

/// MEV protection layer for mempool
pub struct MevProtection {
    config: MevProtectionConfig,
    /// Pending commitments waiting to be revealed
    commitments: HashMap<[u8; 32], TransactionCommitment>,
    /// Revealed transactions ready for inclusion
    revealed: Vec<RevealedTransaction>,
}

impl MevProtection {
    pub fn new(config: MevProtectionConfig) -> Self {
        Self {
            config,
            commitments: HashMap::new(),
            revealed: Vec::new(),
        }
    }

    /// Commit a transaction (submit hash only)
    pub fn commit_transaction(
        &mut self,
        tx_hash: [u8; 32],
        current_height: u64,
    ) -> Result<TransactionCommitment> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let commitment = TransactionCommitment {
            commitment: tx_hash,
            commit_time: now,
            commit_height: current_height,
        };

        self.commitments.insert(tx_hash, commitment.clone());
        Ok(commitment)
    }

    /// Reveal a previously committed transaction
    pub fn reveal_transaction(
        &mut self,
        tx: Transaction,
        _current_height: u64,
    ) -> Result<()> {
        let tx_hash = tx.hash();

        // Check if commitment exists
        let commitment = self
            .commitments
            .get(&tx_hash)
            .ok_or_else(|| anyhow::anyhow!("No commitment found for transaction"))?
            .clone();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Verify reveal timing
        let elapsed = now.saturating_sub(commitment.commit_time);
        if elapsed < self.config.min_reveal_delay {
            return Err(anyhow::anyhow!(
                "Reveal too early: {} < {}",
                elapsed,
                self.config.min_reveal_delay
            ));
        }

        if elapsed > self.config.max_reveal_delay {
            return Err(anyhow::anyhow!(
                "Reveal too late: {} > {}",
                elapsed,
                self.config.max_reveal_delay
            ));
        }

        // Add to revealed transactions
        let revealed = RevealedTransaction {
            transaction: tx,
            commitment,
            reveal_time: now,
        };

        self.revealed.push(revealed);
        self.commitments.remove(&tx_hash);

        Ok(())
    }

    /// Get transactions ready for inclusion (sorted by commit time for fairness)
    pub fn get_ready_transactions(&mut self, limit: usize) -> Vec<Transaction> {
        // Sort by commit time (FIFO based on commit, not reveal)
        self.revealed.sort_by_key(|r| r.commitment.commit_time);

        self.revealed
            .iter()
            .take(limit)
            .map(|r| r.transaction.clone())
            .collect()
    }

    /// Remove transactions that have been included in a block
    pub fn remove_transactions(&mut self, txs: &[Transaction]) {
        let tx_hashes: Vec<[u8; 32]> = txs.iter().map(|tx| tx.hash()).collect();

        self.revealed
            .retain(|r| !tx_hashes.contains(&r.transaction.hash()));
    }

    /// Clean up expired commitments that were never revealed
    pub fn cleanup_expired(&mut self, _current_height: u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.commitments.retain(|_, commitment| {
            let elapsed = now.saturating_sub(commitment.commit_time);
            elapsed <= self.config.max_reveal_delay
        });
    }

    /// Get number of pending commitments
    pub fn pending_commitments(&self) -> usize {
        self.commitments.len()
    }

    /// Get number of revealed transactions
    pub fn revealed_count(&self) -> usize {
        self.revealed.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::types::Transaction;
    use std::thread;
    use std::time::Duration;

    fn create_test_transaction(nonce: u64) -> Transaction {
        let mut tx = Transaction::test_transaction([0; 20], nonce);
        tx.signature = vec![nonce as u8; 64];
        tx
    }

    #[test]
    fn test_commit_reveal_flow() {
        let config = MevProtectionConfig {
            min_reveal_delay: 0, // No delay for testing
            max_reveal_delay: 10,
            enable_encrypted_mempool: true,
        };

        let mut mev = MevProtection::new(config);
        let tx = create_test_transaction(1);
        let tx_hash = tx.hash();

        // Commit
        let commitment = mev.commit_transaction(tx_hash, 1).unwrap();
        assert_eq!(mev.pending_commitments(), 1);

        // Reveal
        mev.reveal_transaction(tx.clone(), 1).unwrap();
        assert_eq!(mev.pending_commitments(), 0);
        assert_eq!(mev.revealed_count(), 1);

        // Get ready transactions
        let ready = mev.get_ready_transactions(10);
        assert_eq!(ready.len(), 1);
    }

    #[test]
    fn test_reveal_too_early() {
        let config = MevProtectionConfig {
            min_reveal_delay: 5,
            max_reveal_delay: 10,
            enable_encrypted_mempool: true,
        };

        let mut mev = MevProtection::new(config);
        let tx = create_test_transaction(1);
        let tx_hash = tx.hash();

        mev.commit_transaction(tx_hash, 1).unwrap();

        // Try to reveal immediately
        let result = mev.reveal_transaction(tx, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_fifo_ordering() {
        let config = MevProtectionConfig {
            min_reveal_delay: 0,
            max_reveal_delay: 10,
            enable_encrypted_mempool: true,
        };

        let mut mev = MevProtection::new(config);
        let mut transactions = Vec::new();

        // Commit transactions in order
        for i in 0..5 {
            let tx = create_test_transaction(i);
            let tx_hash = tx.hash();
            transactions.push(tx);
            mev.commit_transaction(tx_hash, 1).unwrap();
            thread::sleep(Duration::from_millis(100)); // Ensure different timestamps
        }

        // Reveal in reverse order
        for i in (0..5).rev() {
            mev.reveal_transaction(transactions[i].clone(), 1).unwrap();
        }

        // Should still be ordered by commit time
        let ready = mev.get_ready_transactions(5);
        assert_eq!(ready.len(), 5);
        
        // Debug: print actual order
        println!("Transaction order:");
        for (idx, tx) in ready.iter().enumerate() {
            println!("  idx={}, nonce={}", idx, tx.nonce);
        }
        
        for (idx, tx) in ready.iter().enumerate() {
            assert_eq!(tx.nonce, idx as u64, "Transaction at index {} has nonce {} instead of {}", idx, tx.nonce, idx);
        }
    }
}
