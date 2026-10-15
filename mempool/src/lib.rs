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
use parking_lot::Mutex;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info, warn};

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

/// Per-sender `(pending tx count, next expected nonce)`.
type SenderState = HashMap<[u8; 20], (usize, u64)>;

/// Transaction pool for holding pending transactions
pub struct Mempool {
    /// Priority queue of transactions (highest fee first)
    /// Using BinaryHeap for O(log n) insertion and extraction
    transactions: Arc<Mutex<BinaryHeap<PrioritizedTransaction>>>,

    /// Quick lookup to check if transaction exists (by signature)
    seen_txs: Arc<Mutex<HashSet<Vec<u8>>>>,

    /// Track transaction count and next nonce per sender
    /// Key: sender address, Value: (count, next_expected_nonce)
    sender_state: Arc<Mutex<SenderState>>,

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

        // Check 2: Signature is valid, binds `public_key` to `sender`, and
        // (when configured) covers the expected chain ID.
        self.verify_signer(tx)?;

        // Check 3: Minimum fee requirement
        if tx.max_priority_fee_per_gas < self.config.min_fee_per_gas {
            return Err(anyhow!(
                "Transaction fee {} is below minimum {}",
                tx.max_priority_fee_per_gas,
                self.config.min_fee_per_gas
            ));
        }

        // Check 4: Not already in mempool
        let seen_txs = self.seen_txs.lock();
        if seen_txs.contains(&tx.signature) {
            return Err(anyhow!("Transaction already in mempool"));
        }

        Ok(())
    }

    /// Authenticate the transaction: the signature over `tx.hash()` must
    /// verify under `tx.public_key`, and `sender` must be derived from it.
    fn verify_signer(&self, tx: &Transaction) -> Result<()> {
        match self.config.chain_id {
            Some(chain_id) => {
                common::validation::verify_tx_signature(tx, chain_id)?;
            }
            None => {
                if !tx.verify(&tx.public_key) {
                    return Err(anyhow!("Invalid signature"));
                }
                if common::crypto::derive_address_from_public_key(&tx.public_key) != tx.sender {
                    return Err(anyhow!("Public key does not match sender"));
                }
            }
        }
        Ok(())
    }

    /// Add a transaction to the mempool with validation
    pub fn add_transaction(&self, tx: Transaction) -> Result<()> {
        // Step 1: Validate the transaction
        self.validate_transaction(&tx)?;

        // Step 2: Check sender limits & track min nonce
        let mut sender_state = self.sender_state.lock();
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

        let mut timestamp_counter = self.timestamp_counter.lock();
        let timestamp = *timestamp_counter;
        *timestamp_counter += 1;

        // Step 5: Add to priority queue with eviction if needed
        drop(timestamp_counter);
        let mut transactions = self.transactions.lock();
        let mut seen_txs = self.seen_txs.lock();

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
                let mut sender_state = self.sender_state.lock();
                if let Some((count, _)) = sender_state.get_mut(&evicted.tx.sender) {
                    *count = count.saturating_sub(1);
                }

                warn!(
                    "Evicted low-fee transaction (fee={}) to make room",
                    evicted.fee
                );
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
        let mut sender_state = self.sender_state.lock();
        if let Some((ref mut count, _)) = sender_state.get_mut(&tx.sender) {
            *count += 1;
        }

        info!(
            "Transaction added to mempool. Count: {}",
            transactions.len()
        );
        debug!(
            "  Sender: {:?}, Nonce: {}, Fee: {}",
            tx.sender, tx.nonce, tx.max_priority_fee_per_gas
        );

        Ok(())
    }

    /// Select up to `limit` transactions for a block, without removing them.
    ///
    /// Each sender's transactions are taken in nonce order starting from its
    /// next expected nonce (so one sender can have several in a block);
    /// across senders, the highest fee goes first. The pool is unchanged —
    /// transactions leave it only when a block including them is finalized
    /// (`remove_transactions`), so a proposal that fails to finalize loses
    /// nothing.
    pub fn get_transactions(&self, limit: usize) -> Vec<Transaction> {
        let transactions = self.transactions.lock();
        let sender_state = self.sender_state.lock();

        let mut queues: HashMap<[u8; 20], Vec<&PrioritizedTransaction>> = HashMap::new();
        for ptx in transactions.iter() {
            queues.entry(ptx.tx.sender).or_default().push(ptx);
        }
        let mut cursors: HashMap<[u8; 20], (usize, u64)> = HashMap::new();
        for (sender, queue) in queues.iter_mut() {
            queue.sort_by_key(|p| (p.tx.nonce, std::cmp::Reverse(p.fee), p.timestamp));
            let expected = sender_state
                .get(sender)
                .map(|(_, n)| *n)
                .filter(|n| *n != u64::MAX)
                .unwrap_or(queue[0].tx.nonce);
            cursors.insert(*sender, (0, expected));
        }

        let mut result = Vec::with_capacity(limit.min(transactions.len()));
        while result.len() < limit {
            // Each sender's candidate is its first queued tx with the expected
            // nonce (stale lower nonces are skipped; a gap blocks the sender).
            let mut best: Option<(&[u8; 20], usize, &PrioritizedTransaction)> = None;
            for (sender, queue) in queues.iter() {
                let (mut idx, expected) = cursors[sender];
                while idx < queue.len() && queue[idx].tx.nonce < expected {
                    idx += 1;
                }
                if idx < queue.len() && queue[idx].tx.nonce == expected {
                    let candidate = queue[idx];
                    if best.is_none_or(|(_, _, b)| candidate > b) {
                        best = Some((sender, idx, candidate));
                    }
                }
            }
            let Some((sender, idx, ptx)) = best else {
                break;
            };
            result.push(ptx.tx.clone());
            let sender = *sender;
            cursors.insert(sender, (idx + 1, ptx.tx.nonce + 1));
        }

        result
    }

    /// Remove transactions that have been included in a finalized block, and
    /// advance each sender's expected nonce past them. Pooled transactions
    /// whose nonce is now stale are dropped too.
    pub fn remove_transactions(&self, txs: &[Transaction]) {
        let mut transactions = self.transactions.lock();
        let mut seen_txs = self.seen_txs.lock();
        let mut sender_state = self.sender_state.lock();

        let mut next_nonce: HashMap<[u8; 20], u64> = HashMap::new();
        for tx in txs {
            let n = next_nonce.entry(tx.sender).or_insert(0);
            *n = (*n).max(tx.nonce.saturating_add(1));
        }
        let included: HashSet<&Vec<u8>> = txs.iter().map(|tx| &tx.signature).collect();

        let before = transactions.len();
        let remaining: BinaryHeap<PrioritizedTransaction> = std::mem::take(&mut *transactions)
            .into_iter()
            .filter(|ptx| {
                let stale = next_nonce
                    .get(&ptx.tx.sender)
                    .is_some_and(|n| ptx.tx.nonce < *n);
                let drop = stale || included.contains(&ptx.tx.signature);
                if drop {
                    seen_txs.remove(&ptx.tx.signature);
                }
                !drop
            })
            .collect();
        *transactions = remaining;

        // Recount per sender and advance expected nonces.
        let mut counts: HashMap<[u8; 20], usize> = HashMap::new();
        for ptx in transactions.iter() {
            *counts.entry(ptx.tx.sender).or_default() += 1;
        }
        for (sender, next) in next_nonce {
            match counts.get(&sender) {
                Some(count) => {
                    let entry = sender_state.entry(sender).or_insert((0, next));
                    entry.0 = *count;
                    entry.1 = if entry.1 == u64::MAX {
                        next
                    } else {
                        entry.1.max(next)
                    };
                }
                None => {
                    sender_state.remove(&sender);
                }
            }
        }

        info!(
            "Removed {} transactions from mempool. Remaining: {}",
            before - transactions.len(),
            transactions.len()
        );
    }

    /// Get current size of mempool
    pub fn size(&self) -> usize {
        let transactions = self.transactions.lock();
        transactions.len()
    }

    /// Get transaction count per sender (for debugging)
    pub fn get_sender_count(&self, sender: &[u8; 20]) -> Option<usize> {
        let sender_state = self.sender_state.lock();
        sender_state.get(sender).map(|(count, _)| *count)
    }

    /// Clear the entire mempool (useful for testing or chain reorgs)
    pub fn clear(&self) {
        let mut transactions = self.transactions.lock();
        let mut seen_txs = self.seen_txs.lock();
        let mut sender_state = self.sender_state.lock();

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

// ============================================================================
// MEV Integration - Private Mempool
// ============================================================================

use mev::{CommitRevealScheme, DecryptionShare, EncryptedTransaction, ThresholdEncryption};

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
        current_height: u64,
    ) -> [u8; 32] {
        self.commit_reveal
            .commit(tx_hash, secret, sender, nonce, current_height)
    }

    /// Reveal a committed transaction
    pub fn reveal_transaction(
        &mut self,
        tx: Transaction,
        secret: [u8; 32],
        commitment: [u8; 32],
        current_height: u64,
    ) -> Result<Transaction, String> {
        let revealed = self
            .commit_reveal
            .reveal(tx, secret, commitment, current_height)
            .map_err(|e| format!("{:?}", e))?;
        // Add to regular mempool after reveal
        self.regular
            .add_transaction(revealed.transaction.clone())
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
        let decrypted = self
            .threshold_encryption
            .get_decrypted_transactions(max_count);
        all.extend(decrypted);

        // Get revealed transactions
        let revealed = self
            .commit_reveal
            .get_ready_transactions(max_count - all.len());
        all.extend(revealed);

        // Get regular transactions
        let regular = self.regular.get_transactions(max_count - all.len());
        all.extend(regular);

        all
    }

    /// Forward a validated transaction into the regular mempool pool.
    pub fn add_transaction(&mut self, tx: Transaction) -> Result<()> {
        self.regular.add_transaction(tx)
    }

    /// Remove included transactions from the regular pool.
    pub fn remove_transactions(&mut self, txs: &[Transaction]) {
        self.regular.remove_transactions(txs);
    }

    /// Pending transaction count (regular pool only).
    pub fn size(&self) -> usize {
        self.regular.size()
    }

    /// Snapshot of regular mempool transactions (for RPC listing).
    pub fn get_transactions(&mut self, max_count: usize) -> Vec<Transaction> {
        self.regular.get_transactions(max_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ed25519_dalek::SigningKey;

    /// A correctly signed tx from the account whose key is derived from `seed`.
    fn create_test_tx(nonce: u64, fee: u64, seed: [u8; 20]) -> Transaction {
        let key = SigningKey::from_bytes(&[seed[0]; 32]);
        let mut tx = Transaction::test_transaction([0; 20], nonce);
        tx.max_priority_fee_per_gas = fee;
        tx.chain_id = Some(1);
        tx.sign_with(&key);
        tx
    }

    #[test]
    fn rejects_forged_or_mismatched_signatures() {
        let mempool = Mempool::new(MempoolConfig::default());

        let mut forged = create_test_tx(1, 1_000_000_000, [1; 20]);
        forged.value = 1_000_000; // altered after signing
        assert!(mempool.add_transaction(forged).is_err());

        let mut wrong_sender = create_test_tx(1, 1_000_000_000, [1; 20]);
        wrong_sender.sender = [9; 20];
        assert!(mempool.add_transaction(wrong_sender).is_err());

        let mut wrong_chain = Transaction::test_transaction([0; 20], 1);
        wrong_chain.max_priority_fee_per_gas = 1_000_000_000;
        wrong_chain.chain_id = Some(2);
        wrong_chain.sign_with(&SigningKey::from_bytes(&[1; 32]));
        assert!(mempool.add_transaction(wrong_chain).is_err());

        assert_eq!(mempool.size(), 0);
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

        // Both are ready, in nonce order (the higher fee does not jump ahead).
        let batch = mempool.get_transactions(2);
        assert_eq!(
            batch.iter().map(|t| t.nonce).collect::<Vec<_>>(),
            vec![1, 2]
        );
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

        let sender_addr = tx1.sender;
        mempool.remove_transactions(&[tx1]);
        assert_eq!(mempool.size(), 1);

        // Sender state should be updated
        assert_eq!(mempool.get_sender_count(&sender_addr), Some(1));
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

    /// Regression: selecting transactions for a block removed them from the
    /// pool, so finalizing never advanced the sender's nonce and every later
    /// transaction from that sender was stuck forever.
    #[test]
    fn selection_is_non_destructive_and_finalization_advances_nonce() {
        let mempool = Mempool::new(MempoolConfig::default());
        let tx0 = create_test_tx(0, 1_000_000_000, [1; 20]);
        mempool.add_transaction(tx0.clone()).unwrap();

        let block1 = mempool.get_transactions(10);
        assert_eq!(block1, vec![tx0.clone()]);
        assert_eq!(mempool.size(), 1, "selection must not remove");

        // The next tx arrives before block 1 is finalized.
        let tx1 = create_test_tx(1, 1_000_000_000, [1; 20]);
        mempool.add_transaction(tx1.clone()).unwrap();
        mempool.remove_transactions(&block1);
        assert_eq!(mempool.get_transactions(10), vec![tx1.clone()]);

        mempool.remove_transactions(&[tx1]);
        assert_eq!(mempool.size(), 0);
    }

    #[test]
    fn finalized_nonces_evict_stale_duplicates() {
        let mempool = Mempool::new(MempoolConfig::default());
        let a = create_test_tx(0, 1_000_000_000, [1; 20]);
        let b = create_test_tx(0, 2_000_000_000, [1; 20]); // same nonce, other tx
        mempool.add_transaction(a.clone()).unwrap();
        mempool.add_transaction(b).unwrap();
        // `a` finalized (e.g. via another node's block): `b` can never execute.
        mempool.remove_transactions(&[a]);
        assert_eq!(mempool.size(), 0);
    }
}
