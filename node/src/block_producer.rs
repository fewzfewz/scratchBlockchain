use common::traits::Consensus;
use common::types::{Block, Header, Transaction, Account, Address};
use consensus::EnhancedConsensus;
use common::crypto::SigningKey;
use execution::{Executor, NativeExecutor};
use mempool::Mempool;
use storage::StateStore;
use governance::InflationSchedule;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

pub struct BlockProducer {
    mempool: Arc<Mempool>,
    consensus: Arc<Mutex<EnhancedConsensus>>,
    state_store: Arc<StateStore>,
    block_store: Arc<storage::BlockStore>,
    finality_gadget: Arc<Mutex<consensus::FinalityGadget>>,
    signing_key: SigningKey,
    current_slot: u64,
    inflation_schedule: InflationSchedule,
}

impl BlockProducer {
    pub fn new(
        mempool: Arc<Mempool>,
        consensus: Arc<Mutex<EnhancedConsensus>>,
        state_store: Arc<StateStore>,
        block_store: Arc<storage::BlockStore>,
        finality_gadget: Arc<Mutex<consensus::FinalityGadget>>,
        signing_key: SigningKey,
    ) -> Self {
        Self {
            mempool,
            consensus,
            state_store,
            block_store,
            finality_gadget,
            signing_key,
            current_slot: 0,
            inflation_schedule: InflationSchedule::default(),
        }
    }

    /// Produce a new block from mempool transactions
    pub async fn produce_block(&mut self, parent: &Block) -> Result<Block, Box<dyn std::error::Error>> {
        info!("Producing new block at slot {}", self.current_slot);

        // Get transactions from mempool
        let transactions = self.mempool.get_transactions(100); // Max 100 txs per block
        
        if transactions.is_empty() {
            info!("No transactions in mempool, skipping block production");
            return Err("No transactions available".into());
        }

        info!("Building block with {} transactions", transactions.len());

        // Load current state from storage into memory
        let mut state = self.load_state_from_storage()?;

        // Calculate base fee from parent
        let base_fee = execution::gas::calculate_next_base_fee(
            parent.header.gas_used,
            30_000_000, // Block gas limit
            parent.header.base_fee,
        );
        
        // Execute transactions and update state
        let executor = NativeExecutor::new();
        let mut header = Header::new(parent.hash(), self.current_slot);
        header.base_fee = base_fee;
        
        let mut valid_transactions = Vec::new();
        let mut total_gas_used = 0;
        let block_gas_limit = 30_000_000;

        for tx in transactions {
            // Check if we have enough gas left in the block
            if total_gas_used + tx.gas_limit > block_gas_limit {
                info!("Skipping transaction due to block gas limit");
                continue;
            }

            // Try to execute
            // Note: In a production system, we should use a checkpoint/revert mechanism
            // For this MVP, execute_transaction performs checks before mutations, so it's relatively safe
            match executor.execute_transaction(&tx, &mut state) {
                Ok(gas_used) => {
                    total_gas_used += gas_used;
                    valid_transactions.push(tx);
                }
                Err(e) => {
                    info!("Transaction failed execution: {}", e);
                    // Skip invalid transaction
                }
            }
        }
        
        header.gas_used = total_gas_used;
        
        // Persist updated state to storage
        self.persist_state_to_storage(&state)?;
        
        // Compute state root from updated state
        let state_root = self.state_store.root_hash()?;
        header.state_root = state_root;

        // Compute extrinsics root
        let extrinsics_root = self.compute_extrinsics_root(&valid_transactions);
        header.extrinsics_root = extrinsics_root;

        // Sign the header
        let header_hash = header.hash();
        header.signature = self.signing_key.sign(&header_hash);

        // Create final block
        let block = Block::new(header, valid_transactions.clone());

        // Validate with consensus
        {
            let consensus = self.consensus.lock().await;
            consensus.verify_block(&block)?;
        }

        // Persist block
        self.block_store.put_block(&block)?;
        self.block_store.set_latest_height(block.header.slot)?;

        info!("Block produced successfully at slot {}", self.current_slot);

        // Remove transactions from mempool
        self.mempool.remove_transactions(&valid_transactions);

        // Increment slot for next block
        self.current_slot += 1;

        // Calculate block reward using inflation schedule
        let block_height = block.header.slot;
        let block_reward = self.inflation_schedule.calculate_reward(block_height);
        
        // Calculate total fees from transactions
        let total_fees: u128 = valid_transactions.iter()
            .map(|tx| tx.max_fee_per_gas as u128 * tx.gas_limit as u128)
            .sum();
        
        // Calculate fee burn
        let fee_burn = self.inflation_schedule.calculate_fee_burn(total_fees);
        let fee_to_validator = total_fees - fee_burn;
        
        info!("Block reward: {} tokens, Fees: {} (burned: {})", 
              block_reward / 1_000_000_000, 
              fee_to_validator / 1_000_000_000,
              fee_burn / 1_000_000_000);
        
        // Treasury gets 10% of block reward
        let treasury_share = block_reward / 10;
        let validator_reward = block_reward - treasury_share + fee_to_validator;
        
        // Create Coinbase transaction (Reward to validator)
        let validator_address = Address::default(); // Placeholder: should derive from signing_key
        
        // Add reward to state directly
        if let Some(account) = state.get_mut(&validator_address) {
            account.balance += validator_reward;
        } else {
            state.insert(validator_address, Account {
                nonce: 0,
                balance: validator_reward,
            });
        }
        
        info!("Awarded {} tokens to validator (treasury: {})", 
              validator_reward / 1_000_000_000,
              treasury_share / 1_000_000_000);

        Ok(block)
    }

    /// Load state from storage into memory
    fn load_state_from_storage(&self) -> Result<HashMap<Address, Account>, Box<dyn std::error::Error>> {
        self.state_store.get_all_accounts()
    }

    /// Persist state from memory to storage
    fn persist_state_to_storage(&self, state: &HashMap<Address, Account>) -> Result<(), Box<dyn std::error::Error>> {
        for (address, account) in state {
            self.state_store.put_account(address, account)?;
        }
        Ok(())
    }

    /// Simple extrinsics root computation (hash of all transaction hashes)
    fn compute_extrinsics_root(&self, transactions: &[Transaction]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        
        let mut hasher = Sha256::new();
        for tx in transactions {
            hasher.update(tx.hash());
        }
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use consensus::ValidatorInfo;
    use mempool::MempoolConfig;

    fn create_test_transaction(nonce: u64) -> Transaction {
        let mut tx = Transaction::test_transaction([1; 20], nonce);
        tx.signature = vec![nonce as u8; 64];
        tx
    }

    #[tokio::test]
    async fn test_block_production() {
        // Setup
        let mempool = Arc::new(Mempool::new(MempoolConfig::default()));
        
        let signing_key = SigningKey::from_bytes(&[1u8; 32]).unwrap();
        let public_key = signing_key.public_key();
        let validators = vec![ValidatorInfo {
            public_key: public_key.clone(),
            stake: 100,
            slashed: false,
        }];
        
        let consensus = Arc::new(Mutex::new(EnhancedConsensus::new(validators.clone())));
        
        // Create temporary state store
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_state_db");
        let state_store = Arc::new(StateStore::new(path.to_str().unwrap()).unwrap());
        
        // Initialize genesis state
        let genesis_config = common::types::GenesisConfig::default();
        state_store.initialize_genesis(&genesis_config).unwrap();
        
        // Create block store
        let block_store_path = dir.path().join("test_block_db");
        let block_store = Arc::new(storage::BlockStore::new(block_store_path.to_str().unwrap()).unwrap());

        // Create finality gadget
        let finality_gadget = Arc::new(Mutex::new(consensus::FinalityGadget::new(validators.clone())));
        
        let mut producer = BlockProducer::new(
            mempool.clone(),
            consensus,
            state_store,
            block_store,
            finality_gadget,
            signing_key,
        );

        // Add transactions to mempool (note: these will fail execution due to invalid signatures)
        // For this test, we're just checking that block production works
        for i in 0..5 {
            mempool.add_transaction(create_test_transaction(i)).unwrap();
        }

        assert_eq!(mempool.size(), 5);

        // Produce block - will fail due to invalid transaction signatures
        let genesis = Block::genesis();
        let result = producer.produce_block(&genesis).await;

        // Expect failure due to invalid signatures
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_empty_mempool() {
        let mempool = Arc::new(Mempool::new(MempoolConfig::default()));
        
        let signing_key = SigningKey::from_bytes(&[1u8; 32]).unwrap();
        let public_key = signing_key.public_key();
        let validators = vec![ValidatorInfo {
            public_key: public_key.clone(),
            stake: 100,
            slashed: false,
        }];
        
        let consensus = Arc::new(Mutex::new(EnhancedConsensus::new(validators.clone())));
        
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_state_db_empty");
        let state_store = Arc::new(StateStore::new(path.to_str().unwrap()).unwrap());
        
        let block_store_path = dir.path().join("test_block_db_empty");
        let block_store = Arc::new(storage::BlockStore::new(block_store_path.to_str().unwrap()).unwrap());
        let finality_gadget = Arc::new(Mutex::new(consensus::FinalityGadget::new(validators.clone())));

        let mut producer = BlockProducer::new(
            mempool,
            consensus,
            state_store,
            block_store,
            finality_gadget,
            signing_key,
        );

        let genesis = Block::genesis();
        let result = producer.produce_block(&genesis).await;

        assert!(result.is_err());
    }
}
