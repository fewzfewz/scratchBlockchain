//! # MEV Protection Module
//!
//! This module implements advanced MEV protection mechanisms:
//! - Proposer-Builder Separation (PBS)
//! - Threshold encryption for private mempool
//! - Commit-reveal scheme for frontrunning protection
//! - MEV auction with builder competition
//! - Block building optimization

use common::types::{Block, Transaction, Hash};
use common::crypto::SigningKey;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use sha2::{Digest, Sha256};
use rand::Rng;
use tracing::{info, warn, debug, error};

// ============================================================================
// Commit-Reveal Scheme for Frontrunning Protection
// ============================================================================

/// Commitment to a transaction (hash of transaction + secret)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TransactionCommitment {
    /// Hash of (tx_hash + secret)
    pub commitment: [u8; 32],
    /// Block height when commitment was made
    pub commit_height: u64,
    /// Sender address
    pub sender: [u8; 20],
    /// Unique nonce for this commitment
    pub nonce: u64,
}

/// Revealed transaction with its secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevealedTransaction {
    /// Original transaction
    pub transaction: Transaction,
    /// Secret used to create commitment
    pub secret: [u8; 32],
    /// Commitment this reveals
    pub commitment: [u8; 32],
}

/// Commit-reveal scheme for private transaction submission
pub struct CommitRevealScheme {
    /// Pending commitments (commitment -> metadata)
    pending_commitments: HashMap<[u8; 32], CommitmentMetadata>,
    /// Revealed transactions waiting for inclusion
    revealed_transactions: VecDeque<RevealedTransaction>,
    /// Minimum blocks between commit and reveal
    min_reveal_delay: u64,
    /// Maximum blocks before commitment expires
    max_commit_age: u64,
}

#[derive(Debug, Clone)]
struct CommitmentMetadata {
    commit_height: u64,
    sender: [u8; 20],
    nonce: u64,
    timestamp: Instant,
}

impl CommitRevealScheme {
    pub fn new(min_reveal_delay: u64, max_commit_age: u64) -> Self {
        Self {
            pending_commitments: HashMap::new(),
            revealed_transactions: VecDeque::new(),
            min_reveal_delay,
            max_commit_age,
        }
    }

    /// Create a commitment for a transaction
    pub fn commit(&mut self, tx_hash: [u8; 32], secret: [u8; 32], sender: [u8; 20], nonce: u64, current_height: u64) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&tx_hash);
        hasher.update(&secret);
        let commitment = hasher.finalize().into();
        
        self.pending_commitments.insert(commitment, CommitmentMetadata {
            commit_height: current_height,
            sender,
            nonce,
            timestamp: Instant::now(),
        });
        
        debug!("Commitment created at height {}", current_height);
        commitment
    }

    /// Reveal a transaction after the delay period
    pub fn reveal(&mut self, tx: Transaction, secret: [u8; 32], commitment: [u8; 32], current_height: u64) -> Result<RevealedTransaction, String> {
        // Verify commitment exists
        let metadata = self.pending_commitments.get(&commitment)
            .ok_or("Commitment not found")?;
        
        // Check delay period
        if current_height - metadata.commit_height < self.min_reveal_delay {
            return Err(format!("Too early to reveal. Wait {} more blocks", 
                self.min_reveal_delay - (current_height - metadata.commit_height)));
        }
        
        // Verify the secret matches the commitment
        let tx_hash = tx.hash();
        let mut hasher = Sha256::new();
        hasher.update(&tx_hash);
        hasher.update(&secret);
        let computed_commitment: [u8; 32] = hasher.finalize().into();
        
        if computed_commitment != commitment {
            return Err("Invalid secret for commitment".into());
        }
        
        // Verify sender matches
        if tx.sender != metadata.sender {
            return Err("Sender mismatch".into());
        }
        
        // Remove commitment and add revealed transaction
        self.pending_commitments.remove(&commitment);
        
        let revealed = RevealedTransaction {
            transaction: tx,
            secret,
            commitment,
        };
        
        self.revealed_transactions.push_back(revealed.clone());
        info!("Transaction revealed after {} blocks", current_height - metadata.commit_height);
        
        Ok(revealed)
    }

    /// Get ready-to-include revealed transactions
    pub fn get_ready_transactions(&mut self, max_count: usize) -> Vec<Transaction> {
        let mut ready = Vec::new();
        
        for _ in 0..max_count {
            if let Some(revealed) = self.revealed_transactions.pop_front() {
                ready.push(revealed.transaction);
            } else {
                break;
            }
        }
        
        ready
    }

    /// Clean up expired commitments
    pub fn cleanup_expired(&mut self, current_height: u64) {
        let expired: Vec<[u8; 32]> = self.pending_commitments
            .iter()
            .filter(|(_, meta)| current_height - meta.commit_height > self.max_commit_age)
            .map(|(commit, _)| *commit)
            .collect();
        
        for commit in expired {
            self.pending_commitments.remove(&commit);
            debug!("Expired commitment cleaned up");
        }
    }

    /// Get number of pending commitments
    pub fn pending_count(&self) -> usize {
        self.pending_commitments.len()
    }
}

// ============================================================================
// Proposer-Builder Separation (PBS)
// ============================================================================

/// Bid from a block builder to the proposer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderBid {
    /// Builder's public key
    pub builder_pubkey: Vec<u8>,
    /// Full block being offered
    pub block: Block,
    /// Bid amount in native tokens
    pub bid_amount: u128,
    /// Signature over (block_hash + bid_amount)
    pub signature: Vec<u8>,
    /// Transaction root for verification
    pub tx_root: [u8; 32],
    /// Timestamp of bid
    pub timestamp: u64,
    /// MEV value extracted (for transparency)
    pub mev_value: u128,
}

/// Block builder with advanced optimization
pub struct BlockBuilder {
    /// Builder's identity
    pub pubkey: Vec<u8>,
    signing_key: SigningKey,
    /// Optimization strategies
    strategies: Vec<BuildStrategy>,
    /// Performance metrics
    performance: BuilderPerformance,
}

#[derive(Debug, Clone)]
pub enum BuildStrategy {
    /// Maximize gas fees
    GasMaximization,
    /// Maximize MEV extraction
    MevExtraction,
    /// Balance fees and MEV
    Balanced,
    /// Prioritize user transactions
    UserPriority,
}

#[derive(Debug, Default)]
struct BuilderPerformance {
    blocks_built: u64,
    total_bids_submitted: u64,
    total_bids_won: u64,
    avg_bid_amount: u128,
    total_mev_extracted: u128,
}

impl BlockBuilder {
    pub fn new(pubkey: Vec<u8>, signing_key: SigningKey, strategy: BuildStrategy) -> Self {
        Self {
            pubkey,
            signing_key,
            strategies: vec![strategy],
            performance: BuilderPerformance::default(),
        }
    }

    /// Build the optimal block based on strategy
    pub fn build_block(&mut self, parent_block: &Block, available_txs: Vec<Transaction>) -> Result<BuilderBid, String> {
        let start_time = Instant::now();
        
        // Sort and select transactions based on strategy
        let (selected_txs, mev_value) = self.select_transactions(available_txs)?;
        
        // Build block with selected transactions
        let mut block = self.construct_block(parent_block, selected_txs)?;
        
        // Calculate bid amount (percentage of extracted value)
        let bid_amount = match self.strategies.first().unwrap_or(&BuildStrategy::Balanced) {
            BuildStrategy::GasMaximization => mev_value * 80 / 100, // Bid 80% of value
            BuildStrategy::MevExtraction => mev_value * 90 / 100,   // Bid 90% of value
            BuildStrategy::Balanced => mev_value * 70 / 100,         // Bid 70% of value
            BuildStrategy::UserPriority => mev_value * 50 / 100,     // Bid 50% of value
        };
        
        // Create and sign bid
        let tx_root = block.extrinsics_root();
        let mut bid = BuilderBid {
            builder_pubkey: self.pubkey.clone(),
            block: block.clone(),
            bid_amount,
            signature: vec![],
            tx_root,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            mev_value,
        };
        
        // Sign bid
        let message = self.serialize_bid(&bid);
        bid.signature = self.signing_key.sign(&message);
        
        self.performance.blocks_built += 1;
        self.performance.total_bids_submitted += 1;
        self.performance.avg_bid_amount = (self.performance.avg_bid_amount * (self.performance.total_bids_submitted - 1) + bid_amount) / self.performance.total_bids_submitted;
        self.performance.total_mev_extracted += mev_value;
        
        let build_time = start_time.elapsed();
        info!("Block built in {}ms with {} txs, MEV: {}, Bid: {}", 
              build_time.as_millis(), block.extrinsics.len(), mev_value, bid_amount);
        
        Ok(bid)
    }

    /// Select optimal transactions based on strategy
    fn select_transactions(&self, txs: Vec<Transaction>) -> Result<(Vec<Transaction>, u128), String> {
        let mut selected = Vec::new();
        let mut total_mev = 0u128;
        let mut total_gas = 0u64;
        let max_gas_per_block = 30_000_000;
        
        // Score each transaction based on strategy
        let mut scored_txs: Vec<(Transaction, u128, u64)> = txs.into_iter()
            .map(|tx| {
                let mev = self.calculate_mev(&tx);
                let score = match self.strategies.first().unwrap_or(&BuildStrategy::Balanced) {
                    BuildStrategy::GasMaximization => mev + (tx.gas_limit as u128 * tx.max_fee_per_gas as u128),
                    BuildStrategy::MevExtraction => mev * 10,
                    BuildStrategy::Balanced => mev + (tx.gas_limit as u128 * tx.max_fee_per_gas as u128 / 2),
                    BuildStrategy::UserPriority => (tx.gas_limit as u128 * tx.max_fee_per_gas as u128) * 2,
                };
                (tx, score, mev)
            })
            .collect();
        
        // Sort by score descending
        scored_txs.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Select transactions within gas limit
        for (tx, score, mev) in scored_txs {
            if total_gas + tx.gas_limit <= max_gas_per_block {
                selected.push(tx);
                total_gas += tx.gas_limit;
                total_mev += mev;
            }
        }
        
        Ok((selected, total_mev))
    }

    /// Calculate MEV value of a transaction
    fn calculate_mev(&self, tx: &Transaction) -> u128 {
        // Simplified MEV calculation
        // In production, this would analyze DEX arbitrage, liquidations, etc.
        
        // Basic MEV from gas price
        let gas_mev = (tx.gas_limit as u128) * (tx.max_priority_fee_per_gas as u128);
        
        // Check for DEX interactions (simplified)
        let dex_mev = if tx.payload.len() > 4 && &tx.payload[0..4] == &[0x7f, 0x00, 0x00, 0x00] {
            10_000_000_000 // 10 tokens if DEX swap
        } else {
            0
        };
        
        gas_mev + dex_mev
    }

    /// Construct block from selected transactions
    fn construct_block(&self, parent: &Block, txs: Vec<Transaction>) -> Result<Block, String> {
        let mut block = Block::new(parent.header.clone(), txs);
        // In production, would compute state root, etc.
        Ok(block)
    }

    /// Serialize bid for signing
    fn serialize_bid(&self, bid: &BuilderBid) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&bid.bid_amount.to_le_bytes());
        bytes.extend_from_slice(&bid.tx_root);
        bytes.extend_from_slice(&bid.timestamp.to_le_bytes());
        bytes
    }

    /// Get builder performance metrics
    pub fn get_performance(&self) -> &BuilderPerformance {
        &self.performance
    }

    /// Record when a bid wins
    pub fn record_win(&mut self) {
        self.performance.total_bids_won += 1;
    }
}

// ============================================================================
// MEV Auction
// ============================================================================

/// MEV auction for block space
pub struct MEVAuction {
    /// Current block height
    current_height: u64,
    /// Bids for current block
    current_bids: Vec<BuilderBid>,
    /// Auction deadline
    deadline: Instant,
    /// Minimum bid increment
    min_bid_increment: u128,
    /// Historical bids for analytics
    bid_history: VecDeque<AuctionResult>,
}

#[derive(Debug, Clone)]
struct AuctionResult {
    height: u64,
    winning_bid: u128,
    mev_value: u128,
    num_bidders: usize,
    timestamp: u64,
}

impl MEVAuction {
    pub fn new(min_bid_increment: u128, auction_duration_secs: u64) -> Self {
        Self {
            current_height: 0,
            current_bids: Vec::new(),
            deadline: Instant::now() + Duration::from_secs(auction_duration_secs),
            min_bid_increment,
            bid_history: VecDeque::with_capacity(1000),
        }
    }

    /// Submit a bid to the auction
    pub fn submit_bid(&mut self, bid: BuilderBid, current_height: u64) -> Result<(), String> {
        // Verify block height
        if current_height != self.current_height {
            // New block height - reset auction
            self.current_height = current_height;
            self.current_bids.clear();
            self.deadline = Instant::now() + Duration::from_secs(12); // One block time
        }
        
        // Check if auction is still open
        if Instant::now() > self.deadline {
            return Err("Auction closed".into());
        }
        
        // Verify bid meets minimum increment
        if let Some(highest_bid) = self.current_bids.iter().map(|b| b.bid_amount).max() {
            if bid.bid_amount < highest_bid + self.min_bid_increment {
                return Err(format!("Bid must be at least {} higher", self.min_bid_increment));
            }
        }
        
        // Verify signature
        if !self.verify_bid_signature(&bid) {
            return Err("Invalid bid signature".into());
        }
        
        self.current_bids.push(bid);
        debug!("Bid submitted: {} tokens", self.current_bids.last().unwrap().bid_amount);
        
        Ok(())
    }

    /// Select the winning bid when auction closes
    pub fn select_winner(&mut self) -> Option<BuilderBid> {
        if Instant::now() < self.deadline {
            return None;
        }
        
        let winner = self.current_bids
            .iter()
            .max_by_key(|bid| bid.bid_amount)
            .cloned();
        
        if let Some(winner) = &winner {
            // Record auction result
            self.bid_history.push_back(AuctionResult {
                height: self.current_height,
                winning_bid: winner.bid_amount,
                mev_value: winner.mev_value,
                num_bidders: self.current_bids.len(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
            
            // Keep history manageable
            while self.bid_history.len() > 1000 {
                self.bid_history.pop_front();
            }
            
            info!("Auction winner: bid of {} tokens from {} bidders", 
                  winner.bid_amount, self.current_bids.len());
        }
        
        winner
    }

    /// Verify bid signature
    fn verify_bid_signature(&self, bid: &BuilderBid) -> bool {
        use ed25519_dalek::{Signature, VerifyingKey};
        
        if bid.signature.len() != 64 {
            return false;
        }
        
        let signature = match Signature::from_slice(&bid.signature) {
            Ok(sig) => sig,
            Err(_) => return false,
        };
        
        let verifying_key = match VerifyingKey::from_bytes(bid.builder_pubkey.as_slice().try_into().unwrap_or(&[0u8; 32])) {
            Ok(key) => key,
            Err(_) => return false,
        };
        
        let mut message = Vec::new();
        message.extend_from_slice(&bid.bid_amount.to_le_bytes());
        message.extend_from_slice(&bid.tx_root);
        message.extend_from_slice(&bid.timestamp.to_le_bytes());
        
        verifying_key.verify(&message, &signature).is_ok()
    }

    /// Get current highest bid
    pub fn highest_bid(&self) -> Option<u128> {
        self.current_bids.iter().map(|b| b.bid_amount).max()
    }

    /// Get auction statistics
    pub fn get_stats(&self) -> AuctionStats {
        let total_bids: u128 = self.bid_history.iter().map(|r| r.winning_bid).sum();
        let avg_bid = if !self.bid_history.is_empty() {
            total_bids / self.bid_history.len() as u128
        } else {
            0
        };
        
        AuctionStats {
            total_auctions: self.bid_history.len(),
            avg_winning_bid: avg_bid,
            total_mev_extracted: self.bid_history.iter().map(|r| r.mev_value).sum(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuctionStats {
    pub total_auctions: usize,
    pub avg_winning_bid: u128,
    pub total_mev_extracted: u128,
}

// ============================================================================
// Threshold Encryption for Private Mempool
// ============================================================================

/// Encrypted transaction with threshold decryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedTransaction {
    /// Encrypted transaction data
    pub encrypted_data: Vec<u8>,
    /// Nonce to prevent replay
    pub nonce: u64,
    /// Decryption threshold (minimum shares needed)
    pub threshold: usize,
    /// Validator set ID for decryption
    pub validator_set_id: u64,
}

/// Decryption share from a validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecryptionShare {
    /// Transaction nonce
    pub nonce: u64,
    /// Validator's public key
    pub validator: Vec<u8>,
    /// Share of the decryption key
    pub share: Vec<u8>,
    /// Signature over the share
    pub signature: Vec<u8>,
}

/// Threshold encryption for private mempool
pub struct ThresholdEncryption {
    /// Pending encrypted transactions
    encrypted_txs: HashMap<u64, EncryptedTransaction>,
    /// Collected decryption shares (nonce -> validator -> share)
    decryption_shares: HashMap<u64, HashMap<Vec<u8>, Vec<u8>>>,
    /// Decrypted transactions ready for inclusion
    decrypted_txs: VecDeque<Transaction>,
    /// Total validators
    total_validators: usize,
    /// Validator public keys
    validator_pubkeys: Vec<Vec<u8>>,
}

impl ThresholdEncryption {
    pub fn new(validator_pubkeys: Vec<Vec<u8>>, threshold: usize) -> Self {
        Self {
            encrypted_txs: HashMap::new(),
            decryption_shares: HashMap::new(),
            decrypted_txs: VecDeque::new(),
            total_validators: validator_pubkeys.len(),
            validator_pubkeys,
        }
    }

    /// Encrypt a transaction using threshold encryption
    pub fn encrypt_transaction(&mut self, tx: &Transaction, nonce: u64, threshold: usize) -> EncryptedTransaction {
        // In production, this would use actual threshold encryption (e.g., Shamir's Secret Sharing)
        // For now, we use a simplified XOR-based encryption
        
        let tx_bytes = bincode::serialize(tx).unwrap_or_default();
        let key = self.generate_shared_key(nonce);
        
        let encrypted_data: Vec<u8> = tx_bytes
            .iter()
            .zip(key.iter().cycle())
            .map(|(b, k)| b ^ k)
            .collect();
        
        EncryptedTransaction {
            encrypted_data,
            nonce,
            threshold,
            validator_set_id: 0,
        }
    }

    /// Submit an encrypted transaction to the mempool
    pub fn submit_encrypted(&mut self, encrypted: EncryptedTransaction) -> Result<(), String> {
        if self.encrypted_txs.contains_key(&encrypted.nonce) {
            return Err("Transaction with same nonce already exists".into());
        }
        
        self.encrypted_txs.insert(encrypted.nonce, encrypted);
        debug!("Encrypted transaction submitted with nonce {}", encrypted.nonce);
        
        Ok(())
    }

    /// Submit a decryption share from a validator
    pub fn submit_decryption_share(&mut self, share: DecryptionShare) -> Result<(), String> {
        // Verify validator is authorized
        if !self.validator_pubkeys.contains(&share.validator) {
            return Err("Unauthorized validator".into());
        }
        
        // Verify signature
        if !self.verify_decryption_share(&share) {
            return Err("Invalid share signature".into());
        }
        
        // Get the encrypted transaction
        let encrypted = match self.encrypted_txs.get(&share.nonce) {
            Some(tx) => tx,
            None => return Err("Transaction not found".into()),
        };
        
        // Store the share
        let shares = self.decryption_shares
            .entry(share.nonce)
            .or_insert_with(HashMap::new);
        shares.insert(share.validator, share.share);
        
        // Check if threshold is met
        if shares.len() >= encrypted.threshold {
            self.try_decrypt_transaction(share.nonce)?;
        }
        
        Ok(())
    }

    /// Try to decrypt a transaction when enough shares are collected
    fn try_decrypt_transaction(&mut self, nonce: u64) -> Result<(), String> {
        let encrypted = match self.encrypted_txs.get(&nonce) {
            Some(tx) => tx,
            None => return Err("Transaction not found".into()),
        };
        
        // In production, combine shares using polynomial interpolation
        // For now, we reconstruct the key by XORing shares
        
        let shares = self.decryption_shares.get(&nonce).unwrap();
        let mut combined_key = vec![0u8; 32];
        
        for share in shares.values() {
            for (i, byte) in share.iter().enumerate() {
                if i < combined_key.len() {
                    combined_key[i] ^= byte;
                }
            }
        }
        
        // Decrypt the transaction
        let decrypted_bytes: Vec<u8> = encrypted.encrypted_data
            .iter()
            .zip(combined_key.iter().cycle())
            .map(|(b, k)| b ^ k)
            .collect();
        
        match bincode::deserialize::<Transaction>(&decrypted_bytes) {
            Ok(tx) => {
                self.decrypted_txs.push_back(tx);
                self.encrypted_txs.remove(&nonce);
                self.decryption_shares.remove(&nonce);
                info!("Transaction decrypted successfully for nonce {}", nonce);
                Ok(())
            }
            Err(e) => Err(format!("Decryption failed: {}", e)),
        }
    }

    /// Get decrypted transactions ready for inclusion
    pub fn get_decrypted_transactions(&mut self, max_count: usize) -> Vec<Transaction> {
        let mut ready = Vec::new();
        
        for _ in 0..max_count {
            if let Some(tx) = self.decrypted_txs.pop_front() {
                ready.push(tx);
            } else {
                break;
            }
        }
        
        ready
    }

    /// Generate a shared key from nonce (simplified)
    fn generate_shared_key(&self, nonce: u64) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(nonce.to_le_bytes());
        hasher.finalize().to_vec()
    }

    /// Verify decryption share signature
    fn verify_decryption_share(&self, share: &DecryptionShare) -> bool {
        use ed25519_dalek::{Signature, VerifyingKey};
        
        if share.signature.len() != 64 {
            return false;
        }
        
        let signature = match Signature::from_slice(&share.signature) {
            Ok(sig) => sig,
            Err(_) => return false,
        };
        
        let verifying_key = match VerifyingKey::from_bytes(share.validator.as_slice().try_into().unwrap_or(&[0u8; 32])) {
            Ok(key) => key,
            Err(_) => return false,
        };
        
        let mut message = Vec::new();
        message.extend_from_slice(&share.nonce.to_le_bytes());
        message.extend_from_slice(&share.share);
        
        verifying_key.verify(&message, &signature).is_ok()
    }

    /// Get number of pending encrypted transactions
    pub fn pending_encrypted_count(&self) -> usize {
        self.encrypted_txs.len()
    }
}

// ============================================================================
// Main MEV Manager
// ============================================================================

/// Main MEV management system
pub struct MevManager {
    /// Commit-reveal scheme
    pub commit_reveal: CommitRevealScheme,
    /// MEV auction
    pub auction: MEVAuction,
    /// Threshold encryption
    pub threshold_encryption: ThresholdEncryption,
    /// Block builders
    builders: HashMap<Vec<u8>, BlockBuilder>,
    /// Current block height
    current_height: u64,
}

impl MevManager {
    pub fn new(validator_pubkeys: Vec<Vec<u8>>) -> Self {
        Self {
            commit_reveal: CommitRevealScheme::new(5, 100),
            auction: MEVAuction::new(1_000_000_000, 12),
            threshold_encryption: ThresholdEncryption::new(validator_pubkeys, 2),
            builders: HashMap::new(),
            current_height: 0,
        }
    }

    /// Register a block builder
    pub fn register_builder(&mut self, pubkey: Vec<u8>, signing_key: SigningKey, strategy: BuildStrategy) {
        let builder = BlockBuilder::new(pubkey.clone(), signing_key, strategy);
        self.builders.insert(pubkey, builder);
    }

    /// Update current block height
    pub fn update_height(&mut self, height: u64) {
        self.current_height = height;
        self.commit_reveal.cleanup_expired(height);
    }

    /// Process MEV for new block
    pub fn process_block_production(&mut self, parent_block: &Block, available_txs: Vec<Transaction>) -> Option<BuilderBid> {
        // Let builders create bids
        for (_, builder) in self.builders.iter_mut() {
            if let Ok(bid) = builder.build_block(parent_block, available_txs.clone()) {
                let _ = self.auction.submit_bid(bid, self.current_height);
            }
        }
        
        // Select winner
        let winner = self.auction.select_winner();
        
        if let Some(ref bid) = winner {
            // Record win for builder
            if let Some(builder) = self.builders.get_mut(&bid.builder_pubkey) {
                builder.record_win();
            }
        }
        
        winner
    }

    /// Get MEV statistics
    pub fn get_stats(&self) -> MevStats {
        let auction_stats = self.auction.get_stats();
        
        MevStats {
            total_auctions: auction_stats.total_auctions,
            avg_winning_bid: auction_stats.avg_winning_bid,
            total_mev_extracted: auction_stats.total_mev_extracted,
            pending_commitments: self.commit_reveal.pending_count(),
            pending_encrypted: self.threshold_encryption.pending_encrypted_count(),
            active_builders: self.builders.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MevStats {
    pub total_auctions: usize,
    pub avg_winning_bid: u128,
    pub total_mev_extracted: u128,
    pub pending_commitments: usize,
    pub pending_encrypted: usize,
    pub active_builders: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_commit_reveal() {
        let mut scheme = CommitRevealScheme::new(2, 10);
        let tx = Transaction::test_transaction([1; 20], 0);
        let tx_hash = tx.hash();
        let secret = [42u8; 32];
        let sender = [1; 20];
        
        let commitment = scheme.commit(tx_hash, secret, sender, 0, 1);
        assert_eq!(scheme.pending_count(), 1);
        
        // Try to reveal too early
        assert!(scheme.reveal(tx, secret, commitment, 2).is_err());
        
        // Reveal after delay
        let result = scheme.reveal(tx.clone(), secret, commitment, 3);
        assert!(result.is_ok());
        
        let ready = scheme.get_ready_transactions(1);
        assert_eq!(ready.len(), 1);
    }
    
    #[test]
    fn test_auction() {
        let mut auction = MEVAuction::new(100, 12);
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        
        let block = Block::genesis();
        let builder = BlockBuilder::new(vec![1; 32], signing_key, BuildStrategy::Balanced);
        let bid = builder.build_block(&block, vec![]).unwrap();
        
        assert!(auction.submit_bid(bid, 1).is_ok());
        assert_eq!(auction.highest_bid(), Some(0));
    }
}