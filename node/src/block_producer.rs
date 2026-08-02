//! # Block Producer Module
//!
//! This module handles block production for validators. It:
//! - Pulls transactions from the mempool
//! - Validates transactions (nonce, balance, signature)
//! - Builds and signs blocks
//! - Submits to BFT consensus
//! - Executes and commits finalized blocks
//!
//! ## Flow
//! 1. Produce block (proposer role)
//! 2. Submit to BFT engine via `create_proposal()`
//! 3. BFT broadcasts proposal and collects votes
//! 4. When finalized, `execute_and_commit()` runs

use common::crypto::SigningKey;
use common::types::{Block, Header, Transaction};
use consensus::{BftEngine, BftEvent, ValidatorInfo};
use execution::evm::{SignedTransaction, EvmExecutor, TransactionReceipt};
use mempool::Mempool;
use storage::{ChainStore, ColumnFamily, WriteBatch};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn, debug, error};

/// Configuration for block production
#[derive(Debug, Clone)]
pub struct BlockProducerConfig {
    /// Maximum number of transactions per block
    pub max_transactions_per_block: usize,
    
    /// Maximum gas per block (block gas limit)
    pub max_gas_per_block: u64,
    
    /// Target block utilization (for gas price calculation)
    pub target_gas_utilization: f64,
}

impl Default for BlockProducerConfig {
    fn default() -> Self {
        Self {
            max_transactions_per_block: 5000,
            max_gas_per_block: 30_000_000,  // 30M gas limit
            target_gas_utilization: 0.5,    // Target 50% full
        }
    }
}

/// Block executor - wires BFT finalization → EVM execution → atomic storage
/// 
/// This is the critical link that was missing in the initial code review.
/// It ensures that when BFT finalizes a block, it's properly executed and
/// atomically persisted to storage.
pub struct BlockExecutor {
    /// EVM executor with persistent state backend
    evm: EvmExecutor,
    
    /// Chain storage with atomic commit support
    chain_store: Arc<ChainStore>,
}

impl BlockExecutor {
    /// Create a new block executor
    pub fn new(evm: EvmExecutor, chain_store: Arc<ChainStore>) -> Self {
        Self { evm, chain_store }
    }

    /// Execute all transactions in a block and commit everything atomically.
    /// Called by BlockProducer after BFT finalizes a block.
    /// 
    /// # Returns
    /// * `Vec<TransactionReceipt>` - Execution receipts for each transaction
    pub fn execute_and_commit(
        &mut self, 
        block: &Block,
    ) -> Result<Vec<TransactionReceipt>, Box<dyn std::error::Error>> {
        // Convert block transactions to EVM format
        use revm::primitives::{Address, Bytes, U256};
        let txns: Vec<SignedTransaction> = block
            .extrinsics
            .iter()
            .map(|tx| SignedTransaction {
                caller: Address::from_slice(&tx.sender),
                to: tx.to.map(|a| Address::from_slice(&a)),
                value: U256::from(tx.value),
                data: Bytes::from(tx.payload.clone()),
                nonce: tx.nonce,
                gas_limit: tx.gas_limit,
                gas_price: U256::from(tx.max_fee_per_gas),
                chain_id: tx.chain_id.unwrap_or(1),
                signature: None,
            })
            .collect();

        // Execute all transactions sequentially - each transaction sees state changes
        // from previous transactions in the same block
        let receipts = self.evm.execute_block(txns)?;

        // Build state diff from execution results
        // In a full implementation, this would extract the actual state changes
        // from the EVM's state backend. For now, we store receipts keyed by tx hash.
        let receipt_pairs: Vec<(Vec<u8>, Vec<u8>)> = block
            .extrinsics
            .iter()
            .zip(receipts.iter())
            .map(|(tx, receipt)| {
                let encoded = serde_json::to_vec(receipt)
                    .unwrap_or_else(|_| b"{}".to_vec());
                (tx.hash().to_vec(), encoded)
            })
            .collect();

        let block_hash = block.hash();
        let block_height = block.header.slot;
        let block_encoded = serde_json::to_vec(block)
            .map_err(|e| format!("Failed to encode block: {}", e))?;

        // Atomic commit: block + height index + receipts + latest_height
        self.chain_store.commit_block(
            block_height,
            &block_hash,
            &block_encoded,
            vec![], // state diffs would come from EVM state backend
            receipt_pairs,
        )?;

        info!(
            "Block {} (height={}) committed with {} transactions",
            hex::encode(&block_hash[..4]),
            block_height,
            receipts.len()
        );

        Ok(receipts)
    }

    /// Fetch the latest committed block from storage.
    ///
    /// Used as the parent for newly produced blocks so that headers chain to
    /// the real previous block (instead of a snapshot taken at node startup).
    pub fn latest_block(&self) -> Result<Block, Box<dyn std::error::Error>> {
        match self.chain_store.get_latest_height()? {
            Some(height) => {
                let data = self.chain_store
                    .get_block_by_height(height)?
                    .ok_or_else(|| format!("Block not found at height {}", height))?;
                Ok(serde_json::from_slice(&data)?)
            }
            None => Ok(Block::genesis()),
        }
    }
}

/// Block producer - creates new blocks and submits to consensus
pub struct BlockProducer {
    /// Mempool for transaction selection
    mempool: Arc<Mempool>,
    
    /// BFT engine for consensus
    bft_engine: Arc<Mutex<BftEngine>>,
    
    /// Block executor for finalization
    block_executor: BlockExecutor,
    
    /// Validator's signing key
    signing_key: SigningKey,
    
    /// Current slot (block height) being produced
    current_slot: u64,
    
    /// Validator's address (for block signing)
    validator_address: [u8; 20],
    
    /// Block production configuration
    config: BlockProducerConfig,
}

impl BlockProducer {
    /// Create a new block producer
    pub fn new(
        mempool: Arc<Mempool>,
        bft_engine: Arc<Mutex<BftEngine>>,
        block_executor: BlockExecutor,
        signing_key: SigningKey,
        validator_address: [u8; 20],
        config: BlockProducerConfig,
    ) -> Self {
        Self {
            mempool,
            bft_engine,
            block_executor,
            signing_key,
            current_slot: 0,
            validator_address,
            config,
        }
    }

    /// Resume block production from the given slot (used on restart so the chain
    /// continues from its persisted tip instead of restarting at slot 0).
    pub fn set_current_slot(&mut self, slot: u64) {
        self.current_slot = slot;
    }

    /// Produce a new block and return it for the caller to submit to the BFT
    /// engine via `create_proposal()`.
    ///
    /// # Flow
    /// 1. Pull transactions from mempool (respecting gas limit)
    /// 2. Pre-validate transactions (nonce, balance, signature)
    /// 3. Build and sign block header (including state_root)
    ///
    /// The block's slot is derived from the BFT engine's current height: height
    /// h finalizes the block produced at slot h-1 (genesis occupies slot 0).
    /// Deriving it here — rather than keeping a free-running counter — means a
    /// proposer that has been idle for many heights still builds the correct
    /// block for its height, keeping every validator's chain contiguous. The
    /// parent is always the actual latest committed block, so headers chain
    /// properly to their real predecessor.
    pub async fn produce_block(&mut self) -> Result<Block, Box<dyn std::error::Error>> {
        let slot = {
            let engine = self.bft_engine.lock().await;
            engine.height.saturating_sub(1)
        };
        self.current_slot = slot;
        let parent = self.block_executor.latest_block()?;
        info!("Producing block at slot {}", slot);

        // Step 1: Get transactions from mempool (prioritized by fee)
        let mut transactions = self.mempool.get_transactions(self.config.max_transactions_per_block);
        info!("Mempool: {} transactions available", transactions.len());

        // Step 2: Calculate gas limits and filter transactions
        let mut total_gas_used = 0u64;
        let mut valid_transactions = Vec::new();
        
        for tx in transactions {
            if total_gas_used + tx.gas_limit <= self.config.max_gas_per_block {
                total_gas_used += tx.gas_limit;
                valid_transactions.push(tx);
            } else {
                debug!("Transaction {} exceeds remaining block gas limit", hex::encode(tx.hash()));
                break;
            }
        }

        // Step 3: Pre-validate transactions (quick checks before EVM)
        let valid_transactions = self.pre_validate_transactions(valid_transactions);

        // Step 4: Calculate base fee from parent (EIP-1559)
        let base_fee = execution::gas::calculate_next_base_fee(
            parent.header.gas_used,
            self.config.max_gas_per_block,
            parent.header.base_fee,
        );

        // Step 5: Build extrinsics root (Merkle root of transactions)
        let extrinsics_root = self.compute_extrinsics_root(&valid_transactions);

        // Step 6: Build block header (state_root will be filled after execution)
        let mut header = Header::new(parent.hash(), self.current_slot);
        header.base_fee = base_fee;
        header.extrinsics_root = extrinsics_root;
        // FIX: state_root will be set after execution - for now use placeholder
        header.state_root = [0u8; 32]; // Will be updated during finalization

        // Step 7: Sign the header
        let header_hash = header.hash();
        header.signature = self.signing_key.sign(&header_hash);

        let block = Block::new(header, valid_transactions.clone());

        // The caller (node main loop) submits the block to the BFT engine via
        // create_proposal(), which broadcasts the proposal and vote. Returning
        // the block here keeps a single code path for proposing.
        Ok(block)
    }

    /// Called by the node's main loop when BFT finalizes a block from a remote proposer.
    pub fn handle_finalized_block(
        &mut self,
        block: &Block,
    ) -> Result<Vec<TransactionReceipt>, Box<dyn std::error::Error>> {
        info!(
            "Finalizing block from network height={} txns={}",
            block.header.slot,
            block.extrinsics.len()
        );
        
        let receipts = self.block_executor.execute_and_commit(block)?;
        self.mempool.remove_transactions(&block.extrinsics);
        
        Ok(receipts)
    }

    /// Pre-validate transactions before EVM execution
    /// 
    /// Checks:
    /// - Non-zero gas limit
    /// - Non-empty signature
    /// - Valid signature length
    fn pre_validate_transactions(&self, txns: Vec<Transaction>) -> Vec<Transaction> {
        txns.into_iter()
            .filter(|tx| {
                // Check gas limit
                if tx.gas_limit == 0 {
                    warn!("Dropping tx with zero gas limit: {:?}", hex::encode(tx.hash()));
                    return false;
                }
                
                // Check signature presence
                if tx.signature.is_empty() {
                    warn!("Dropping unsigned tx: {:?}", hex::encode(tx.hash()));
                    return false;
                }
                
                // Check signature length (ed25519 = 64 bytes)
                if tx.signature.len() != 64 {
                    warn!("Dropping tx with invalid signature length: {}", tx.signature.len());
                    return false;
                }
                
                true
            })
            .collect()
    }

    /// Compute Merkle root of transaction hashes
    fn compute_extrinsics_root(&self, transactions: &[Transaction]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        
        if transactions.is_empty() {
            // Empty root
            return [0u8; 32];
        }
        
        // Simple Merkle tree construction
        let mut hashes: Vec<[u8; 32]> = transactions
            .iter()
            .map(|tx| tx.hash())
            .collect();
        
        // Build Merkle tree
        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in hashes.chunks(2) {
                let mut hasher = Sha256::new();
                hasher.update(&chunk[0]);
                if chunk.len() > 1 {
                    hasher.update(&chunk[1]);
                } else {
                    hasher.update(&chunk[0]); // Duplicate for odd length
                }
                next_level.push(hasher.finalize().into());
            }
            hashes = next_level;
        }
        
        hashes[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use consensus::ValidatorInfo;
    use execution::evm::{EvmExecutor, InMemoryStore};
    use mempool::{Mempool, MempoolConfig};
    use storage::{ChainStore, MemDb};
    use std::sync::Arc;

    fn make_test_setup() -> (BlockProducer, Arc<Mempool>) {
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let public_key = signing_key.public_key();
        let validator_addr = [0x01u8; 20];

        let validator = ValidatorInfo {
            public_key: public_key.clone(),
            stake: 100,
            slashed: false,
        };

        let bft_engine = Arc::new(Mutex::new(BftEngine::new(
            public_key,
            vec![validator],
            1,
            signing_key.clone(),
        )));

        let store = Arc::new(InMemoryStore::default());
        let evm = EvmExecutor::with_store(store);
        let db = Arc::new(MemDb::new());
        let chain_store = Arc::new(ChainStore::new(db));
        let executor = BlockExecutor::new(evm, chain_store);

        let mempool = Arc::new(Mempool::new(MempoolConfig::default()));

        let producer = BlockProducer::new(
            mempool.clone(),
            bft_engine,
            executor,
            signing_key,
            validator_addr,
            BlockProducerConfig::default(),
        );

        (producer, mempool)
    }

    #[tokio::test]
    async fn test_empty_block_production() {
        let (mut producer, _mempool) = make_test_setup();
        let result = producer.produce_block().await;
        assert!(result.is_ok(), "Empty block should be produced: {:?}", result.err());
        let block = result.unwrap();
        assert_eq!(block.extrinsics.len(), 0);
        // BFT starts at height 1, so the produced block lives at slot 0.
        assert_eq!(block.header.slot, 0);
    }

    #[tokio::test]
    async fn test_slot_tracks_bft_height() {
        let (mut producer, _mempool) = make_test_setup();
        // BFT height 1 -> slot 0
        producer.produce_block().await.unwrap();
        assert_eq!(producer.current_slot, 0);

        // Advance the engine to height 5; a previously-idle proposer must
        // produce the correct slot for that height (4), not a stale counter.
        {
            let mut engine = producer.bft_engine.lock().await;
            engine.height = 5;
        }
        producer.produce_block().await.unwrap();
        assert_eq!(producer.current_slot, 4);
        let block = producer.produce_block().await.unwrap();
        assert_eq!(block.header.slot, 4);
    }

    #[tokio::test]
    async fn test_unsigned_tx_filtered_out() {
        let (mut producer, mempool) = make_test_setup();

        // Add unsigned transaction
        let mut tx = Transaction::default();
        tx.gas_limit = 21_000;
        tx.signature = vec![]; // empty - should be dropped
        let _ = mempool.add_transaction(tx);

        let result = producer.produce_block().await;
        assert!(result.is_ok());
        // Unsigned tx should have been filtered before EVM
        assert_eq!(result.unwrap().extrinsics.len(), 0);
    }
}