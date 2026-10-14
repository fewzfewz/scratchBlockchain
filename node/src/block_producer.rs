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

use crate::tx_pool::TxPool;
use common::crypto::SigningKey;
use common::types::{Block, Header, Transaction};
use consensus::BftEngine;
use execution::evm::{EvmExecutor, SignedTransaction, TransactionReceipt};
use governance::ChainGovernance;
use std::sync::Arc;
use storage::ChainStore;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

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
            max_gas_per_block: 30_000_000, // 30M gas limit
            target_gas_utilization: 0.5,   // Target 50% full
        }
    }
}

/// State writes from one transaction: `(key, Some(value))` puts, `(key, None)` deletes.
type StateDiff = Vec<(Vec<u8>, Option<Vec<u8>>)>;

/// Block executor - wires BFT finalization → EVM execution → atomic storage
///
/// This is the critical link that was missing in the initial code review.
/// It ensures that when BFT finalizes a block, it's properly executed and
/// atomically persisted to storage.
pub struct BlockExecutor {
    /// EVM executor with persistent state backend
    evm: EvmExecutor,

    /// Chain storage with atomic commit support
    pub(crate) chain_store: Arc<ChainStore>,
}

impl BlockExecutor {
    /// Create a new block executor
    pub fn new(evm: EvmExecutor, chain_store: Arc<ChainStore>) -> Self {
        Self { evm, chain_store }
    }

    /// `WASMDEPLOY:<name>:<module>`: validate the module and register it under
    /// a name that is not taken yet (no overwriting someone else's contract).
    fn execute_wasm_deploy(payload: &[u8], chain_store: &Arc<ChainStore>) -> TransactionReceipt {
        let rest = &payload[execution::WASM_DEPLOY_PREFIX.len()..];
        let Some(sep) = rest.iter().position(|b| *b == b':') else {
            return Self::failed_receipt("Invalid WASM deploy payload");
        };
        let Ok(name) = std::str::from_utf8(&rest[..sep]) else {
            return Self::failed_receipt("Invalid WASM contract name");
        };
        let module = &rest[sep + 1..];
        let registry = crate::wasm_registry::WasmRegistry::new(chain_store.clone());
        if matches!(registry.get(name), Ok(Some(_))) {
            return Self::failed_receipt(format!("WASM contract '{}' already exists", name));
        }
        let valid = execution::WasmExecutor::new()
            .map_err(|e| e.to_string())
            .and_then(|exec| exec.validate_module(module).map_err(|e| e.to_string()));
        if let Err(e) = valid {
            return Self::failed_receipt(format!("Invalid WASM module: {}", e));
        }
        match registry.deploy(name, module) {
            Ok(()) => Self::success_receipt(),
            Err(e) => Self::failed_receipt(e.to_string()),
        }
    }

    fn execute_wasm_tx(payload: &[u8], chain_store: &Arc<ChainStore>) -> TransactionReceipt {
        let rest = match std::str::from_utf8(&payload[execution::WASM_TX_PREFIX.len()..]) {
            Ok(s) => s,
            Err(_) => {
                return TransactionReceipt {
                    success: false,
                    gas_used: 21_000,
                    output: vec![],
                    created_address: None,
                    revert_reason: Some("Invalid WASM payload encoding".into()),
                    logs: vec![],
                };
            }
        };
        let parts: Vec<&str> = rest.splitn(3, ':').collect();
        if parts.len() < 2 {
            return TransactionReceipt {
                success: false,
                gas_used: 21_000,
                output: vec![],
                created_address: None,
                revert_reason: Some("Invalid WASM payload".into()),
                logs: vec![],
            };
        }
        let name = parts[0];
        let func = parts[1];
        let arg: i32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
        let registry = crate::wasm_registry::WasmRegistry::new(chain_store.clone());
        let wasm = match registry.get(name) {
            Ok(Some(b)) => b,
            _ => {
                return TransactionReceipt {
                    success: false,
                    gas_used: 21_000,
                    output: vec![],
                    created_address: None,
                    revert_reason: Some(format!("WASM contract '{}' not found", name)),
                    logs: vec![],
                };
            }
        };
        match execution::WasmExecutor::new() {
            Ok(exec) => match exec.execute_i32(&wasm, func, arg) {
                Ok((_result, gas)) => TransactionReceipt {
                    success: true,
                    gas_used: gas.max(21_000),
                    output: vec![],
                    created_address: None,
                    revert_reason: None,
                    logs: vec![],
                },
                Err(e) => TransactionReceipt {
                    success: false,
                    gas_used: 21_000,
                    output: vec![],
                    created_address: None,
                    revert_reason: Some(e.to_string()),
                    logs: vec![],
                },
            },
            Err(e) => TransactionReceipt {
                success: false,
                gas_used: 21_000,
                output: vec![],
                created_address: None,
                revert_reason: Some(e.to_string()),
                logs: vec![],
            },
        }
    }

    /// A plain value transfer has no payload. Anything with a payload
    /// (calldata, bytecode) goes to the EVM.
    fn is_native_transfer(tx: &Transaction) -> bool {
        tx.to.is_some() && tx.payload.is_empty()
    }

    /// Transactions whose receipt succeeded, i.e. that were authenticated and
    /// applied. Receipts are produced 1:1 with `block.extrinsics`.
    pub fn successful_txs(block: &Block, receipts: &[TransactionReceipt]) -> Vec<Transaction> {
        block
            .extrinsics
            .iter()
            .zip(receipts)
            .filter(|(_, r)| r.success)
            .map(|(tx, _)| tx.clone())
            .collect()
    }

    /// Account state as of the transactions already executed in this block:
    /// the latest pending diff for `addr` wins over the committed store, so two
    /// txs from one sender in the same block see each other's nonce bump.
    fn load_account_state(
        chain_store: &ChainStore,
        pending: &StateDiff,
        addr: &[u8],
    ) -> (u128, u64) {
        let pending_value = pending
            .iter()
            .rev()
            .find(|(k, _)| k.as_slice() == addr)
            .map(|(_, v)| v.clone());
        match pending_value {
            Some(v) => crate::accounts::decode(v.as_deref()),
            None => crate::accounts::load(chain_store, addr),
        }
    }

    fn encode_account_state(balance: u128, nonce: u64) -> Vec<u8> {
        crate::accounts::encode(balance, nonce)
    }

    fn success_receipt() -> TransactionReceipt {
        TransactionReceipt {
            success: true,
            gas_used: 21_000,
            output: vec![],
            created_address: None,
            revert_reason: None,
            logs: vec![],
        }
    }

    fn failed_receipt(reason: impl Into<String>) -> TransactionReceipt {
        TransactionReceipt {
            success: false,
            gas_used: 21_000,
            output: vec![],
            created_address: None,
            revert_reason: Some(reason.into()),
            logs: vec![],
        }
    }

    /// Check the sender's nonce and consume it. Used by tx kinds that carry no
    /// value (governance, WASM) so a signed tx cannot be replayed.
    fn consume_nonce(
        tx: &Transaction,
        chain_store: &ChainStore,
        pending: &StateDiff,
    ) -> Result<StateDiff, String> {
        let (balance, nonce) = Self::load_account_state(chain_store, pending, &tx.sender);
        if nonce != tx.nonce {
            return Err(format!(
                "Invalid nonce: expected {}, got {}",
                nonce, tx.nonce
            ));
        }
        Ok(vec![(
            tx.sender.to_vec(),
            Some(Self::encode_account_state(balance, nonce + 1)),
        )])
    }

    fn execute_native_transfer(
        tx: &Transaction,
        chain_store: &ChainStore,
        pending: &StateDiff,
    ) -> (TransactionReceipt, StateDiff) {
        let to = match tx.to {
            Some(to) => to,
            None => {
                return (
                    Self::failed_receipt("Native executor only supports transfers"),
                    vec![],
                );
            }
        };

        let (sender_balance, sender_nonce) =
            Self::load_account_state(chain_store, pending, &tx.sender);
        if sender_nonce != tx.nonce {
            return (
                Self::failed_receipt(format!(
                    "Invalid nonce: expected {}, got {}",
                    sender_nonce, tx.nonce
                )),
                vec![],
            );
        }

        let gas_fee = tx.gas_limit as u128 * tx.max_fee_per_gas as u128;
        let total_cost = tx.value as u128 + gas_fee; // both < 2^64·2^64: no overflow
        if sender_balance < total_cost {
            return (
                Self::failed_receipt(format!(
                    "Insufficient balance: need {}, have {}",
                    total_cost, sender_balance
                )),
                vec![],
            );
        }

        let (recipient_balance, recipient_nonce) =
            Self::load_account_state(chain_store, pending, &to);
        let new_sender_balance = sender_balance - total_cost; // checked above
                                                              // A self-transfer must see the debit, or it would mint `value`.
        let recipient_balance = if to == tx.sender {
            new_sender_balance
        } else {
            recipient_balance
        };
        let Some(new_recipient_balance) = recipient_balance.checked_add(tx.value as u128) else {
            return (Self::failed_receipt("Recipient balance overflow"), vec![]);
        };
        if to == tx.sender {
            return (
                Self::success_receipt(),
                vec![(
                    tx.sender.to_vec(),
                    Some(Self::encode_account_state(
                        new_recipient_balance,
                        sender_nonce + 1,
                    )),
                )],
            );
        }

        let state_diffs = vec![
            (
                tx.sender.to_vec(),
                Some(Self::encode_account_state(
                    new_sender_balance,
                    sender_nonce + 1,
                )),
            ),
            (
                to.to_vec(),
                Some(Self::encode_account_state(
                    new_recipient_balance,
                    recipient_nonce,
                )),
            ),
        ];

        (
            TransactionReceipt {
                success: true,
                gas_used: 21_000,
                output: vec![],
                created_address: None,
                revert_reason: None,
                logs: vec![],
            },
            state_diffs,
        )
    }

    /// Execute all transactions in a block and commit everything atomically.
    /// Called by BlockProducer after BFT finalizes a block.
    ///
    /// Governance transactions are detected by their payload and applied to the
    /// node's own state trie (by `governance_store::apply_extrinsics`) rather
    /// than the EVM. A failing transaction produces a failed receipt instead of
    /// aborting the whole block, so a single bad transaction cannot stall the
    /// chain.
    ///
    /// # Returns
    /// * `Vec<TransactionReceipt>` - Execution receipts for each transaction
    pub fn execute_and_commit(
        &mut self,
        block: &Block,
    ) -> Result<Vec<TransactionReceipt>, Box<dyn std::error::Error>> {
        use revm::primitives::{Address, Bytes, U256};

        let mut receipts = Vec::with_capacity(block.extrinsics.len());
        let mut state_diffs = Vec::new();
        let chain_id = self.evm.chain_id();
        for tx in &block.extrinsics {
            // Authenticate once, before any branch below trusts `tx.sender`.
            if let Err(e) = common::validation::verify_tx_signature(tx, chain_id) {
                receipts.push(Self::failed_receipt(e.to_string()));
                continue;
            }

            let body = tx.payload.as_slice();
            // Governance and staking actions are applied afterwards by
            // governance_store::apply_extrinsics (they need the trie).
            let is_governance = ChainGovernance::is_governance_payload(body)
                || crate::governance_store::is_staking_payload(body)
                || crate::bridge_mint::is_bridge_mint_payload(body);
            let is_wasm = body.starts_with(execution::WASM_TX_PREFIX);
            let is_wasm_deploy = body.starts_with(execution::WASM_DEPLOY_PREFIX);
            if is_governance || is_wasm || is_wasm_deploy {
                match Self::consume_nonce(tx, &self.chain_store, &state_diffs) {
                    Ok(diffs) => state_diffs.extend(diffs),
                    Err(e) => {
                        receipts.push(Self::failed_receipt(e));
                        continue;
                    }
                }
            }

            if crate::governance_store::is_staking_payload(body) {
                // Fail the receipt up front if the bond cannot be paid; the
                // registration itself is applied by apply_extrinsics.
                let bond = crate::governance_store::staking_bond_amount(body);
                let (balance, _) =
                    Self::load_account_state(&self.chain_store, &state_diffs, &tx.sender);
                match bond {
                    Some(amount) if amount <= balance => {}
                    Some(amount) => {
                        receipts.push(Self::failed_receipt(format!(
                            "Insufficient balance to bond {}",
                            amount
                        )));
                        continue;
                    }
                    None => {
                        receipts.push(Self::failed_receipt("Invalid staking amount"));
                        continue;
                    }
                }
            }

            if is_governance {
                // Applied afterwards by governance_store::apply_extrinsics.
                receipts.push(TransactionReceipt {
                    success: true,
                    gas_used: 21_000,
                    output: vec![],
                    created_address: None,
                    revert_reason: None,
                    logs: vec![],
                });
                continue;
            }

            if is_wasm_deploy {
                receipts.push(Self::execute_wasm_deploy(body, &self.chain_store));
                continue;
            }

            // WASM contract call: payload `WASM:<name>:<func>:<arg>`
            if is_wasm {
                let receipt = Self::execute_wasm_tx(body, &self.chain_store);
                receipts.push(receipt);
                continue;
            }

            // Native Ed25519 transfer (wallet / SDK transactions)
            if Self::is_native_transfer(tx) {
                let (receipt, diffs) =
                    Self::execute_native_transfer(tx, &self.chain_store, &state_diffs);
                state_diffs.extend(diffs);
                receipts.push(receipt);
                continue;
            }

            let evm_data = Bytes::from(tx.payload.clone());

            let caller = Address::from_slice(&tx.sender);
            let (native_bal, native_nonce) =
                Self::load_account_state(&self.chain_store, &state_diffs, &tx.sender);
            if let Err(e) = self
                .evm
                .prepare_native_account(caller, native_bal, native_nonce)
            {
                warn!(
                    "Failed to sync EVM account for {}: {}",
                    hex::encode(&tx.sender[..6]),
                    e
                );
            }

            let stx = SignedTransaction {
                caller: Address::from_slice(&tx.sender),
                to: tx.to.map(|a| Address::from_slice(&a)),
                value: U256::from(tx.value),
                data: evm_data,
                nonce: tx.nonce,
                gas_limit: tx.gas_limit,
                gas_price: U256::from(tx.max_fee_per_gas),
                chain_id,
            };

            match self.evm.execute_transaction(stx) {
                Ok(receipt) => receipts.push(receipt),
                Err(e) => {
                    warn!("Transaction {} failed: {}", hex::encode(tx.hash()), e);
                    receipts.push(Self::failed_receipt(format!("{}", e)));
                }
            }
        }

        // Build state diff from execution results
        // In a full implementation, this would extract the actual state changes
        // from the EVM's state backend. For now, we store receipts keyed by tx hash.
        let receipt_pairs: Vec<(Vec<u8>, Vec<u8>)> = block
            .extrinsics
            .iter()
            .zip(receipts.iter())
            .map(|(tx, receipt)| {
                let encoded = serde_json::to_vec(receipt).unwrap_or_else(|_| b"{}".to_vec());
                (tx.hash().to_vec(), encoded)
            })
            .collect();

        let block_hash = block.hash();
        let block_height = block.header.slot;
        let block_encoded =
            serde_json::to_vec(block).map_err(|e| format!("Failed to encode block: {}", e))?;

        // Atomic commit: block + height index + receipts + latest_height
        self.chain_store.commit_block(
            block_height,
            &block_hash,
            &block_encoded,
            state_diffs,
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
                let data = self
                    .chain_store
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
    /// Transaction pool (mempool + MEV + account abstraction)
    tx_pool: Arc<TxPool>,

    /// BFT engine for consensus
    bft_engine: Arc<Mutex<BftEngine>>,

    /// Block executor for finalization
    block_executor: BlockExecutor,

    /// Validator's signing key
    signing_key: SigningKey,

    /// Current slot (block height) being produced
    current_slot: u64,

    /// Validator's address (for block signing)
    #[allow(dead_code)]
    validator_address: [u8; 20],

    /// Block production configuration
    config: BlockProducerConfig,
}

impl BlockProducer {
    /// Create a new block producer
    pub fn new(
        tx_pool: Arc<TxPool>,
        bft_engine: Arc<Mutex<BftEngine>>,
        block_executor: BlockExecutor,
        signing_key: SigningKey,
        validator_address: [u8; 20],
        config: BlockProducerConfig,
    ) -> Self {
        Self {
            tx_pool,
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

        // Step 1: Get transactions from pool (AA bundles + MEV-ready + regular)
        let transactions = self
            .tx_pool
            .get_transactions_for_block(self.config.max_transactions_per_block);
        info!("Tx pool: {} transactions available", transactions.len());

        // Step 2: Calculate gas limits and filter transactions
        let mut total_gas_used = 0u64;
        let mut valid_transactions = Vec::new();
        // Room for the header, last commit and JSON framing.
        let mut bytes_left = common::types::MAX_BLOCK_BYTES.saturating_sub(128 * 1024);

        for tx in transactions {
            let tx_bytes = serde_json::to_vec(&tx)
                .map(|b| b.len() + 1)
                .unwrap_or(usize::MAX);
            if tx_bytes > bytes_left {
                debug!("Block size budget reached");
                break;
            }
            if total_gas_used + tx.gas_limit <= self.config.max_gas_per_block {
                bytes_left -= tx_bytes;
                total_gas_used += tx.gas_limit;
                valid_transactions.push(tx);
            } else {
                debug!(
                    "Transaction {} exceeds remaining block gas limit",
                    hex::encode(tx.hash())
                );
                break;
            }
        }

        // Step 3: Drop anything that would make the block invalid (every tx
        // must authenticate; see `validate_block`).
        let valid_transactions = self.pre_validate_transactions(valid_transactions);

        // Step 4: Calculate base fee from parent (EIP-1559)
        let base_fee = execution::gas::calculate_next_base_fee(
            parent.header.gas_used,
            self.config.max_gas_per_block,
            parent.header.base_fee,
        );

        // Step 5: Build extrinsics root (Merkle root of transactions)
        let extrinsics_root = compute_extrinsics_root(&valid_transactions);

        // Step 6: Commit to the state after the parent block.
        let state_root = self
            .block_executor
            .chain_store
            .get_state_root()?
            .ok_or("No committed state root (genesis not initialized)")?;

        // Step 6: Build block header
        let mut header = Header::new(parent.hash(), self.current_slot);
        header.base_fee = base_fee;
        header.extrinsics_root = extrinsics_root;
        header.state_root = state_root;

        // The parent's commit certificate (CometBFT's LastCommit): pays the
        // validators that actually finalized it. Genesis has none.
        let last_commit = if parent.header.slot == 0 {
            None
        } else {
            Some(
                parent
                    .commit
                    .clone()
                    .ok_or("parent block has no commit certificate")?,
            )
        };
        header.last_commit_hash = last_commit.as_ref().map(|c| c.hash()).unwrap_or([0; 32]);
        header.proposer = self
            .signing_key
            .public_key()
            .try_into()
            .map_err(|_| "signing key is not a 32-byte Ed25519 key")?;

        // Step 7: Sign the header
        let header_hash = header.hash();
        header.signature = self.signing_key.sign(&header_hash);

        let mut block = Block::new(header, valid_transactions.clone());
        block.last_commit = last_commit;

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
        self.tx_pool.remove_transactions(&block.extrinsics);

        Ok(receipts)
    }

    /// Pre-validate transactions before EVM execution
    ///
    /// Checks:
    /// - Non-zero gas limit
    /// - Non-empty signature
    /// - Valid signature length
    fn pre_validate_transactions(&self, txns: Vec<Transaction>) -> Vec<Transaction> {
        let chain_id = self.block_executor.evm.chain_id();
        txns.into_iter()
            .filter(|tx| {
                if let Err(e) = common::validation::verify_tx_signature(tx, chain_id) {
                    warn!(
                        "Dropping unauthenticated tx {}: {}",
                        hex::encode(tx.hash()),
                        e
                    );
                    return false;
                }
                // Check gas limit
                if tx.gas_limit == 0 {
                    warn!(
                        "Dropping tx with zero gas limit: {:?}",
                        hex::encode(tx.hash())
                    );
                    return false;
                }

                // Check signature presence
                if tx.signature.is_empty() {
                    warn!("Dropping unsigned tx: {:?}", hex::encode(tx.hash()));
                    return false;
                }

                // Check signature length (ed25519 = 64 bytes)
                if tx.signature.len() != 64 {
                    warn!(
                        "Dropping tx with invalid signature length: {}",
                        tx.signature.len()
                    );
                    return false;
                }

                true
            })
            .collect()
    }
}

/// Merkle root of the block's transaction hashes.
///
/// Leaves and inner nodes are domain-tagged, an odd node is promoted (not
/// duplicated) and the count is committed, so no two distinct transaction
/// lists share a root (duplicating the last leaf lets `[a, b, c]` and
/// `[a, b, c, c]` collide).
pub fn compute_extrinsics_root(transactions: &[Transaction]) -> [u8; 32] {
    use sha2::{Digest, Sha256};

    let mut level: Vec<[u8; 32]> = transactions
        .iter()
        .map(|tx| {
            let mut h = Sha256::new();
            h.update([0u8]);
            h.update(tx.hash());
            h.finalize().into()
        })
        .collect();
    while level.len() > 1 {
        level = level
            .chunks(2)
            .map(|pair| match pair {
                [a, b] => {
                    let mut h = Sha256::new();
                    h.update([1u8]);
                    h.update(a);
                    h.update(b);
                    h.finalize().into()
                }
                [a] => *a,
                _ => unreachable!(),
            })
            .collect();
    }
    let mut h = Sha256::new();
    h.update(b"NEBULA_TXROOT_V1");
    h.update((transactions.len() as u64).to_le_bytes());
    h.update(level.first().copied().unwrap_or([0u8; 32]));
    h.finalize().into()
}

/// Everything a validator checks about a proposed block before voting for it,
/// besides consensus signatures: it extends `parent` at `expected_slot`,
/// commits to our current state root, its extrinsics root and base fee are
/// right, it fits the gas limit, and every transaction authenticates (a block
/// carrying an unauthenticated tx is invalid as a whole).
pub fn validate_block(
    block: &Block,
    parent: &Block,
    parent_validators: &[consensus::ValidatorInfo],
    expected_slot: u64,
    expected_state_root: &[u8; 32],
    chain_id: u64,
    max_gas_per_block: u64,
) -> Result<(), String> {
    let header = &block.header;
    if header.slot != expected_slot {
        return Err(format!(
            "slot {} != expected {}",
            header.slot, expected_slot
        ));
    }
    if header.parent_hash != parent.hash() {
        return Err("parent hash does not match our tip".into());
    }
    if !header.verify_proposer_signature() {
        return Err("header is not signed by its proposer".into());
    }
    let size = serde_json::to_vec(block)
        .map(|b| b.len())
        .unwrap_or(usize::MAX);
    if size > common::types::MAX_BLOCK_BYTES {
        return Err(format!(
            "block is {} bytes, over the {} limit",
            size,
            common::types::MAX_BLOCK_BYTES
        ));
    }
    verify_last_commit(block, parent, parent_validators)?;
    if &header.state_root != expected_state_root {
        return Err(format!(
            "state root {} != ours {}",
            hex::encode(&header.state_root[..8]),
            hex::encode(&expected_state_root[..8])
        ));
    }
    if header.extrinsics_root != compute_extrinsics_root(&block.extrinsics) {
        return Err("extrinsics root mismatch".into());
    }
    let expected_base_fee = execution::gas::calculate_next_base_fee(
        parent.header.gas_used,
        max_gas_per_block,
        parent.header.base_fee,
    );
    if header.base_fee != expected_base_fee {
        return Err(format!(
            "base fee {} != expected {}",
            header.base_fee, expected_base_fee
        ));
    }
    let mut gas = 0u64;
    for tx in &block.extrinsics {
        gas = gas
            .checked_add(tx.gas_limit)
            .filter(|g| *g <= max_gas_per_block)
            .ok_or("block exceeds the gas limit")?;
        common::validation::verify_tx_signature(tx, chain_id)
            .map_err(|e| format!("tx {}: {}", hex::encode(&tx.hash()[..8]), e))?;
    }
    Ok(())
}

/// Check `block.last_commit` — the precommits that finalized `parent` — and
/// return the validators that signed it, sorted (the vote-reward recipients).
///
/// `parent_validators` must be the set that certified `parent` (BFT height
/// `parent.slot + 1`). The header must commit to the certificate, and every
/// block whose parent is not genesis must carry one with a 2/3+ quorum; a
/// proposer can choose which precommits to include but not forge or omit
/// enough of them to fall under quorum.
pub fn verify_last_commit(
    block: &Block,
    parent: &Block,
    parent_validators: &[consensus::ValidatorInfo],
) -> Result<Vec<Vec<u8>>, String> {
    if block.header.last_commit_hash != block.expected_last_commit_hash() {
        return Err("header does not commit to the block's last commit".into());
    }
    match (&block.last_commit, parent.header.slot) {
        (None, 0) => Ok(Vec::new()),
        (Some(_), 0) => Err("block on genesis must not carry a last commit".into()),
        (None, _) => Err("block is missing the parent's commit (last_commit)".into()),
        (Some(cert), slot) => {
            let mut signers = cert
                .verify(&parent.hash(), slot.saturating_add(1), parent_validators)
                .map_err(|e| format!("invalid last commit: {}", e))?;
            signers.sort();
            Ok(signers)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use consensus::ValidatorInfo;
    use execution::evm::{EvmExecutor, InMemoryStore};
    use mempool::MempoolConfig;
    use std::sync::Arc;
    use storage::{ChainStore, MemDb};

    const TEST_ROOT: [u8; 32] = [7u8; 32];

    fn make_test_setup() -> (BlockProducer, Arc<TxPool>) {
        let signing_key = SigningKey::from_bytes(&[1u8; 32]).unwrap();
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
        let evm = EvmExecutor::with_store(store, 1);
        let db = Arc::new(MemDb::new());
        {
            use storage::KeyValueStore;
            db.put(
                storage::ColumnFamily::Meta,
                storage::STATE_ROOT_KEY,
                &TEST_ROOT,
            )
            .unwrap();
        }
        let chain_store = Arc::new(ChainStore::new(db));
        let executor = BlockExecutor::new(evm, chain_store);

        let tx_pool = Arc::new(TxPool::new(MempoolConfig::default(), vec![]));

        let producer = BlockProducer::new(
            tx_pool.clone(),
            bft_engine,
            executor,
            signing_key,
            validator_addr,
            BlockProducerConfig::default(),
        );

        (producer, tx_pool)
    }

    #[tokio::test]
    async fn test_empty_block_production() {
        let (mut producer, _mempool) = make_test_setup();
        let result = producer.produce_block().await;
        assert!(
            result.is_ok(),
            "Empty block should be produced: {:?}",
            result.err()
        );
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
        let mut tx = Transaction::test_transaction([1u8; 20], 0);
        tx.gas_limit = 21_000;
        tx.signature = vec![]; // empty - should be dropped
        let _ = mempool.add_transaction(tx);

        let result = producer.produce_block().await;
        assert!(result.is_ok());
        // Unsigned tx should have been filtered before EVM
        assert_eq!(result.unwrap().extrinsics.len(), 0);
    }

    fn governance_vote_tx(key: &ed25519_dalek::SigningKey, nonce: u64) -> Transaction {
        let mut tx = Transaction::simple_transfer([0u8; 20], [0u8; 20], 0, nonce, 1);
        tx.to = None;
        tx.payload = br#"{"action":"vote","proposal_id":1,"choice":"yes"}"#.to_vec();
        tx.sign_with(key);
        tx
    }

    /// Regression: governance / EVM / WASM payloads used to skip signature checks,
    /// so anyone could vote (or call) as any sender.
    #[test]
    fn test_unauthenticated_txs_are_rejected() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[5u8; 32]);
        let valid = governance_vote_tx(&key, 0);
        let mut forged = governance_vote_tx(&key, 0);
        forged.signature = vec![7u8; 64]; // right sender + key, signature not by that key

        let evm = EvmExecutor::with_store(Arc::new(InMemoryStore::default()), 1);
        let chain_store = Arc::new(ChainStore::new(Arc::new(MemDb::new())));
        let mut executor = BlockExecutor::new(evm, chain_store);

        let block = Block {
            header: Header::new([0u8; 32], 1),
            extrinsics: vec![valid.clone(), forged],
            commit: None,
            last_commit: None,
        };
        let receipts = executor.execute_and_commit(&block).unwrap();

        assert!(receipts[0].success, "{:?}", receipts[0].revert_reason);
        assert!(!receipts[1].success);
        assert_eq!(
            BlockExecutor::successful_txs(&block, &receipts),
            vec![valid]
        );
    }

    /// Governance and WASM txs carry no value, but must still consume a nonce
    /// so a signed vote cannot be replayed (in the same block or a later one).
    #[test]
    fn test_governance_tx_replay_is_rejected() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[5u8; 32]);
        let first = governance_vote_tx(&key, 0);
        let second = governance_vote_tx(&key, 1);

        let evm = EvmExecutor::with_store(Arc::new(InMemoryStore::default()), 1);
        let chain_store = Arc::new(ChainStore::new(Arc::new(MemDb::new())));
        let mut executor = BlockExecutor::new(evm, chain_store);

        let block = Block {
            header: Header::new([0u8; 32], 1),
            extrinsics: vec![first.clone(), first.clone(), second],
            commit: None,
            last_commit: None,
        };
        let receipts = executor.execute_and_commit(&block).unwrap();
        assert!(receipts[0].success);
        assert!(!receipts[1].success, "replay in the same block");
        assert!(receipts[2].success, "next nonce sees the in-block bump");

        let later = Block {
            header: Header::new([0u8; 32], 2),
            extrinsics: vec![first],
            commit: None,
            last_commit: None,
        };
        let receipts = executor.execute_and_commit(&later).unwrap();
        assert!(!receipts[0].success, "replay in a later block");
    }

    fn signed_transfer(seed: u8, nonce: u64) -> Transaction {
        let key = ed25519_dalek::SigningKey::from_bytes(&[seed; 32]);
        let mut tx = Transaction::simple_transfer([0; 20], [9; 20], 1, nonce, 1);
        tx.max_priority_fee_per_gas = 1_000_000_000;
        tx.sign_with(&key);
        tx
    }

    #[tokio::test]
    async fn produced_block_commits_to_state_root_and_validates() {
        let (mut producer, pool) = make_test_setup();
        pool.add_transaction(signed_transfer(3, 0)).unwrap();
        let parent = producer.block_executor.latest_block().unwrap();
        let block = producer.produce_block().await.unwrap();
        assert_eq!(block.header.state_root, TEST_ROOT);
        assert_eq!(block.extrinsics.len(), 1);

        let ok = validate_block(&block, &parent, &[], 0, &TEST_ROOT, 1, 30_000_000);
        assert!(ok.is_ok(), "{:?}", ok);

        // A node whose state differs rejects it.
        assert!(validate_block(&block, &parent, &[], 0, &[8u8; 32], 1, 30_000_000).is_err());

        // Wrong slot / chain id / tampered tx list.
        assert!(validate_block(&block, &parent, &[], 1, &TEST_ROOT, 1, 30_000_000).is_err());
        assert!(validate_block(&block, &parent, &[], 0, &TEST_ROOT, 2, 30_000_000).is_err());
        let mut extra = block.clone();
        extra.extrinsics.push(signed_transfer(4, 0));
        assert!(validate_block(&extra, &parent, &[], 0, &TEST_ROOT, 1, 30_000_000).is_err());
    }

    fn precommit_cert(
        keys: &[ed25519_dalek::SigningKey],
        height: u64,
        block_hash: [u8; 32],
    ) -> common::consensus_types::CommitCertificate {
        use common::consensus_types::{CommitCertificate, Step, Vote};
        use ed25519_dalek::Signer;
        let precommits = keys
            .iter()
            .map(|k| {
                let mut v = Vote {
                    height,
                    round: 0,
                    step: Step::Precommit,
                    block_hash: Some(block_hash),
                    signature: vec![],
                    voter: k.verifying_key().to_bytes().to_vec(),
                };
                v.signature = k.sign(&v.signing_bytes()).to_bytes().to_vec();
                v
            })
            .collect();
        CommitCertificate {
            height,
            round: 0,
            block_hash,
            precommits,
        }
    }

    #[test]
    fn last_commit_must_be_committed_and_carry_a_quorum() {
        let keys: Vec<_> = (10u8..14)
            .map(|i| ed25519_dalek::SigningKey::from_bytes(&[i; 32]))
            .collect();
        let set: Vec<ValidatorInfo> = keys
            .iter()
            .map(|k| ValidatorInfo {
                public_key: k.verifying_key().to_bytes().to_vec(),
                stake: 100,
                slashed: false,
            })
            .collect();
        let parent = Block::new(Header::new([1; 32], 3), vec![]);
        let with_cert = |cert: Option<common::consensus_types::CommitCertificate>| {
            let mut b = Block::new(Header::new(parent.hash(), 4), vec![]);
            b.last_commit = cert;
            b.header.last_commit_hash = b.expected_last_commit_hash();
            b
        };

        // 3 of 4 signed: valid, and those 3 are the reward recipients.
        let block = with_cert(Some(precommit_cert(&keys[..3], 4, parent.hash())));
        let signers = verify_last_commit(&block, &parent, &set).unwrap();
        let mut expected: Vec<Vec<u8>> = set[..3].iter().map(|v| v.public_key.clone()).collect();
        expected.sort();
        assert_eq!(signers, expected);

        // A relayer swapping the certificate breaks the header commitment.
        let mut swapped = block.clone();
        swapped.last_commit = Some(precommit_cert(&keys[1..], 4, parent.hash()));
        assert!(verify_last_commit(&swapped, &parent, &set).is_err());

        // 2 of 4 is not a quorum; a missing or misdirected one is rejected.
        let short = with_cert(Some(precommit_cert(&keys[..2], 4, parent.hash())));
        assert!(verify_last_commit(&short, &parent, &set).is_err());
        assert!(verify_last_commit(&with_cert(None), &parent, &set).is_err());
        let wrong = with_cert(Some(precommit_cert(&keys[..3], 4, [9; 32])));
        assert!(verify_last_commit(&wrong, &parent, &set).is_err());

        // On genesis there is nothing to certify.
        let genesis = Block::genesis();
        let mut first = Block::new(Header::new(genesis.hash(), 1), vec![]);
        assert_eq!(verify_last_commit(&first, &genesis, &[]).unwrap().len(), 0);
        first.last_commit = Some(precommit_cert(&keys[..3], 1, genesis.hash()));
        first.header.last_commit_hash = first.expected_last_commit_hash();
        assert!(verify_last_commit(&first, &genesis, &set).is_err());
    }

    #[test]
    fn block_with_unauthenticated_tx_is_invalid() {
        let parent = Block::genesis();
        let mut forged = signed_transfer(3, 0);
        forged.value = 1_000;
        let mut header = Header::new(parent.hash(), 0);
        header.state_root = TEST_ROOT;
        header.base_fee = execution::gas::calculate_next_base_fee(
            parent.header.gas_used,
            30_000_000,
            parent.header.base_fee,
        );
        header.extrinsics_root = compute_extrinsics_root(std::slice::from_ref(&forged));
        let key = ed25519_dalek::SigningKey::from_bytes(&[9u8; 32]);
        header.proposer = key.verifying_key().to_bytes();
        header.signature = {
            use ed25519_dalek::Signer;
            key.sign(&header.hash()).to_bytes().to_vec()
        };
        let block = Block::new(header, vec![forged]);
        let err = validate_block(&block, &parent, &[], 0, &TEST_ROOT, 1, 30_000_000).unwrap_err();
        assert!(err.contains("Invalid signature"), "{}", err);
    }

    /// A block over MAX_BLOCK_BYTES could not be gossiped, so its round could
    /// never complete: validators must refuse it.
    #[test]
    fn oversized_block_is_invalid() {
        let parent = Block::genesis();
        let key = ed25519_dalek::SigningKey::from_bytes(&[3u8; 32]);
        let mut big = Transaction::simple_transfer([0; 20], [9; 20], 1, 0, 1);
        big.payload = vec![7u8; common::types::MAX_BLOCK_BYTES / 2];
        big.sign_with(&key);
        let mut header = Header::new(parent.hash(), 0);
        header.state_root = TEST_ROOT;
        header.extrinsics_root = compute_extrinsics_root(std::slice::from_ref(&big));
        let proposer = ed25519_dalek::SigningKey::from_bytes(&[9u8; 32]);
        header.proposer = proposer.verifying_key().to_bytes();
        header.signature = {
            use ed25519_dalek::Signer;
            proposer.sign(&header.hash()).to_bytes().to_vec()
        };
        let block = Block::new(header, vec![big]);
        let err = validate_block(&block, &parent, &[], 0, &TEST_ROOT, 1, 30_000_000).unwrap_err();
        assert!(err.contains("over the"), "{}", err);
    }

    /// Regression: the old Merkle construction duplicated an odd last leaf, so
    /// `[a, b, c]` and `[a, b, c, c]` had the same root.
    #[test]
    fn extrinsics_root_is_not_malleable() {
        let (a, b, c) = (
            signed_transfer(1, 0),
            signed_transfer(2, 0),
            signed_transfer(3, 0),
        );
        let three = compute_extrinsics_root(&[a.clone(), b.clone(), c.clone()]);
        let four = compute_extrinsics_root(&[a, b, c.clone(), c]);
        assert_ne!(three, four);
        assert_ne!(compute_extrinsics_root(&[]), [0u8; 32]);
    }

    #[test]
    fn self_transfer_does_not_mint() {
        let evm = EvmExecutor::with_store(Arc::new(InMemoryStore::default()), 1);
        let chain_store = Arc::new(ChainStore::new(Arc::new(MemDb::new())));
        let key = ed25519_dalek::SigningKey::from_bytes(&[6u8; 32]);
        let sender =
            common::crypto::derive_address_from_public_key(&key.verifying_key().to_bytes());
        crate::accounts::store(&chain_store, &sender, 10u128.pow(18), 0).unwrap();

        let mut tx = Transaction::simple_transfer(sender, sender, 500, 0, 1);
        tx.sign_with(&key);
        let mut executor = BlockExecutor::new(evm, chain_store.clone());
        let block = Block::new(Header::new([0; 32], 1), vec![tx.clone()]);
        let receipts = executor.execute_and_commit(&block).unwrap();
        assert!(receipts[0].success);

        let gas = tx.gas_limit as u128 * tx.max_fee_per_gas as u128;
        assert_eq!(
            crate::accounts::load(&chain_store, &sender),
            (10u128.pow(18) - gas, 1)
        );
    }

    #[test]
    fn wasm_deploy_tx_registers_module_once() {
        let evm = EvmExecutor::with_store(Arc::new(InMemoryStore::default()), 1);
        let chain_store = Arc::new(ChainStore::new(Arc::new(MemDb::new())));
        let key = ed25519_dalek::SigningKey::from_bytes(&[8u8; 32]);
        // Smallest valid module: magic + version.
        let module = b"\0asm\x01\0\0\0".to_vec();
        let deploy = |nonce: u64| {
            let mut tx = Transaction::simple_transfer([0; 20], [0; 20], 0, nonce, 1);
            tx.to = None;
            tx.payload = [
                execution::WASM_DEPLOY_PREFIX,
                b"counter:",
                module.as_slice(),
            ]
            .concat();
            tx.sign_with(&key);
            tx
        };
        let mut executor = BlockExecutor::new(evm, chain_store.clone());
        let block = Block::new(Header::new([0; 32], 1), vec![deploy(0), deploy(1)]);
        let receipts = executor.execute_and_commit(&block).unwrap();
        assert!(receipts[0].success, "{:?}", receipts[0].revert_reason);
        assert!(!receipts[1].success, "a name cannot be taken twice");
        let registry = crate::wasm_registry::WasmRegistry::new(chain_store);
        assert_eq!(registry.get("counter").unwrap(), Some(module));
    }
}
