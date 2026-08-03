//! # Core Traits for Blockchain Components
//!
//! This module defines the core interfaces for:
//! - Consensus engines
//! - Storage backends
//! - Transaction execution
//! - State management
//! - Block production

use crate::consensus_types::ValidatorInfo;
use crate::types::{Account, Address, Block, Hash, Header, Transaction, TransactionReceipt};
use std::error::Error;

/// Consensus engine interface
pub trait Consensus {
    /// Verify a block header (signature, validator set, etc.)
    fn verify_header(&self, header: &Header) -> Result<(), Box<dyn Error>>;

    /// Verify a complete block
    fn verify_block(&self, block: &Block) -> Result<(), Box<dyn Error>>;

    /// Check if a block is finalized
    fn is_finalized(&self, hash: &Hash) -> bool;

    /// Get current validator set
    fn validators(&self) -> Vec<ValidatorInfo> {
        vec![]
    }

    /// Get current round
    fn current_round(&self) -> u64 {
        0
    }
}

/// Storage interface for persistence
pub trait Storage {
    /// Get a value by key
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn Error>>;

    /// Put a key-value pair
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Box<dyn Error>>;

    /// Check if a key exists
    fn contains(&self, key: &[u8]) -> Result<bool, Box<dyn Error>>;

    /// Delete a key
    fn delete(&self, key: &[u8]) -> Result<(), Box<dyn Error>>;

    /// Write multiple operations atomically
    fn write_batch(
        &self,
        operations: Vec<(Vec<u8>, Option<Vec<u8>>)>,
    ) -> Result<(), Box<dyn Error>>;
}

/// Transaction execution interface
pub trait Executor {
    /// Execute a complete block, return new state root and receipts
    fn execute_block(
        &self,
        block: &Block,
    ) -> Result<(Hash, Vec<TransactionReceipt>), Box<dyn Error>>;

    /// Execute a single transaction
    fn execute_transaction(
        &self,
        tx: &Transaction,
        state_root: &Hash,
    ) -> Result<TransactionReceipt, Box<dyn Error>>;

    /// Apply transaction to state
    fn apply_transaction(&self, tx: &Transaction) -> Result<(), Box<dyn Error>>;

    /// Get current state root
    fn state_root(&self) -> Result<Hash, Box<dyn Error>>;
}

/// Mempool interface
pub trait Mempool {
    /// Submit a transaction to the pool
    fn submit(&self, tx: Transaction) -> Result<(), Box<dyn Error>>;

    /// Get transactions for block production (ordered by priority)
    fn get_pending(&self, limit: usize) -> Vec<Transaction>;

    /// Remove transactions that have been included in a block
    fn remove(&self, txs: &[Transaction]);

    /// Get current pool size
    fn size(&self) -> usize;

    /// Check if a transaction is in the pool
    fn contains(&self, tx_hash: &Hash) -> bool;
}

/// State management interface
pub trait State {
    /// Get account by address
    fn get_account(&self, address: &Address) -> Result<Option<Account>, Box<dyn Error>>;

    /// Set account
    fn set_account(&mut self, address: &Address, account: &Account) -> Result<(), Box<dyn Error>>;

    /// Get storage value at address+key
    fn get_storage(&self, address: &Address, key: &[u8; 32]) -> Result<[u8; 32], Box<dyn Error>>;

    /// Set storage value
    fn set_storage(
        &mut self,
        address: &Address,
        key: &[u8; 32],
        value: &[u8; 32],
    ) -> Result<(), Box<dyn Error>>;

    /// Get code at address
    fn get_code(&self, address: &Address) -> Result<Vec<u8>, Box<dyn Error>>;

    /// Set code at address
    fn set_code(&mut self, address: &Address, code: Vec<u8>) -> Result<(), Box<dyn Error>>;

    /// Commit state changes
    fn commit(&mut self) -> Result<Hash, Box<dyn Error>>;

    /// Rollback to previous state
    fn rollback(&mut self);
}

/// Block producer interface
pub trait BlockProducer {
    /// Produce a new block
    fn produce_block(&mut self, parent: &Block) -> Result<Block, Box<dyn Error>>;

    /// Set the block building strategy
    fn set_strategy(&mut self, strategy: BlockBuildingStrategy);

    /// Get pending block (if any)
    fn pending_block(&self) -> Option<Block>;
}

/// Block building strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockBuildingStrategy {
    /// Maximize gas fees
    GasMaximization,
    /// Maximize MEV extraction
    MevExtraction,
    /// Balance fees and fairness
    Balanced,
    /// Prioritize user transactions
    UserPriority,
}

/// Validator interface
pub trait Validator {
    /// Get validator public key
    fn public_key(&self) -> Vec<u8>;

    /// Sign a message
    fn sign(&self, message: &[u8]) -> Vec<u8>;

    /// Verify a signature
    fn verify(&self, message: &[u8], signature: &[u8]) -> bool;

    /// Get validator stake
    fn stake(&self) -> u64;

    /// Check if validator is active
    fn is_active(&self) -> bool;
}
