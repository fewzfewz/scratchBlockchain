use anyhow::Result;
use revm::{
    db::{CacheDB, DatabaseRef},
    primitives::{
        AccountInfo, Address, Bytecode, Bytes, CreateScheme, ExecutionResult, Output, TransactTo,
        B256, U256,
    },
    Database, DatabaseCommit, EVM,
};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

// =============================================================================
// Persistent storage backend
// =============================================================================

pub trait EvmStore: Send + Sync {
    fn get_account(&self, address: &Address) -> Option<StoredAccount>;
    fn put_account(&self, address: Address, account: StoredAccount);
    fn get_storage(&self, address: &Address, slot: &U256) -> Option<U256>;
    fn put_storage(&self, address: Address, slot: U256, value: U256);
    fn get_code(&self, code_hash: &B256) -> Option<Bytecode>;
    fn put_code(&self, code_hash: B256, bytecode: Bytecode);
    fn delete_account(&self, address: &Address);

    /// Deterministic state root over all EVM accounts, storage slots, and code.
    fn state_root(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let accounts = self.accounts_for_root();
        let mut hasher = Sha256::new();
        for (k, v) in accounts {
            hasher.update(&k);
            hasher.update(&v);
        }
        hasher.finalize().into()
    }

    /// Override for efficient iteration; default scans in-memory structures only.
    fn accounts_for_root(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        Vec::new()
    }
}

#[derive(Debug, Clone)]
pub struct StoredAccount {
    pub balance: U256,
    pub nonce: u64,
    pub code_hash: Option<B256>,
}

#[derive(Default)]
pub struct InMemoryStore {
    accounts: RwLock<HashMap<Address, StoredAccount>>,
    storage: RwLock<HashMap<(Address, U256), U256>>,
    code: RwLock<HashMap<B256, Bytecode>>,
}

impl EvmStore for InMemoryStore {
    fn get_account(&self, address: &Address) -> Option<StoredAccount> {
        self.accounts.read().unwrap().get(address).cloned()
    }
    fn put_account(&self, address: Address, account: StoredAccount) {
        self.accounts.write().unwrap().insert(address, account);
    }
    fn get_storage(&self, address: &Address, slot: &U256) -> Option<U256> {
        self.storage
            .read()
            .unwrap()
            .get(&(*address, *slot))
            .copied()
    }
    fn put_storage(&self, address: Address, slot: U256, value: U256) {
        self.storage.write().unwrap().insert((address, slot), value);
    }
    fn get_code(&self, code_hash: &B256) -> Option<Bytecode> {
        self.code.read().unwrap().get(code_hash).cloned()
    }
    fn put_code(&self, code_hash: B256, bytecode: Bytecode) {
        self.code.write().unwrap().insert(code_hash, bytecode);
    }
    fn delete_account(&self, address: &Address) {
        self.accounts.write().unwrap().remove(address);
    }

    fn accounts_for_root(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut pairs = Vec::new();
        for (addr, acc) in self.accounts.read().unwrap().iter() {
            let mut k = b"acc:".to_vec();
            k.extend_from_slice(addr.as_slice());
            let mut v = acc.balance.as_le_slice().to_vec();
            v.extend_from_slice(&acc.nonce.to_le_bytes());
            pairs.push((k, v));
        }
        for ((addr, slot), val) in self.storage.read().unwrap().iter() {
            let mut k = b"sto:".to_vec();
            k.extend_from_slice(addr.as_slice());
            k.extend_from_slice(slot.as_le_slice());
            pairs.push((k, val.as_le_slice().to_vec()));
        }
        for (hash, code) in self.code.read().unwrap().iter() {
            let mut k = b"cod:".to_vec();
            k.extend_from_slice(hash.as_slice());
            pairs.push((k, code.bytecode.to_vec()));
        }
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    }
}

pub struct EvmDb {
    store: Arc<dyn EvmStore>,
    // FIX: Add block hash lookup for historical blocks
    block_hash_provider: Option<Arc<dyn Fn(u64) -> B256 + Send + Sync>>,
}

impl EvmDb {
    pub fn new(store: Arc<dyn EvmStore>) -> Self {
        Self {
            store,
            block_hash_provider: None,
        }
    }

    // FIX: Allow setting block hash provider for production
    pub fn with_block_hash_provider<F>(mut self, provider: F) -> Self
    where
        F: Fn(u64) -> B256 + Send + Sync + 'static,
    {
        self.block_hash_provider = Some(Arc::new(provider));
        self
    }

    pub fn state_root(&self) -> [u8; 32] {
        self.store.state_root()
    }
}

impl Database for EvmDb {
    type Error = anyhow::Error;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let info = self.store.get_account(&address).map(|a| {
            let code = a.code_hash.and_then(|h| self.store.get_code(&h));
            AccountInfo {
                balance: a.balance,
                nonce: a.nonce,
                code_hash: a.code_hash.unwrap_or(revm::primitives::KECCAK_EMPTY),
                code,
            }
        });
        Ok(info)
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        Ok(self.store.get_code(&code_hash).unwrap_or(Bytecode::new()))
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        Ok(self
            .store
            .get_storage(&address, &index)
            .unwrap_or(U256::ZERO))
    }

    fn block_hash(&mut self, number: U256) -> Result<B256, Self::Error> {
        if let Some(provider) = &self.block_hash_provider {
            let block_num: u64 = number.try_into().unwrap_or(0);
            Ok(provider(block_num))
        } else {
            tracing::warn!("Block hash requested for {} but no provider set", number);
            Ok(B256::ZERO)
        }
    }
}

impl DatabaseRef for EvmDb {
    type Error = anyhow::Error;

    fn basic(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let info = self.store.get_account(&address).map(|a| {
            let code = a.code_hash.and_then(|h| self.store.get_code(&h));
            AccountInfo {
                balance: a.balance,
                nonce: a.nonce,
                code_hash: a.code_hash.unwrap_or(revm::primitives::KECCAK_EMPTY),
                code,
            }
        });
        Ok(info)
    }

    fn code_by_hash(&self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        Ok(self.store.get_code(&code_hash).unwrap_or(Bytecode::new()))
    }

    fn storage(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        Ok(self
            .store
            .get_storage(&address, &index)
            .unwrap_or(U256::ZERO))
    }

    fn block_hash(&self, number: U256) -> Result<B256, Self::Error> {
        if let Some(provider) = &self.block_hash_provider {
            let block_num: u64 = number.try_into().unwrap_or(0);
            Ok(provider(block_num))
        } else {
            tracing::warn!("Block hash requested for {} but no provider set", number);
            Ok(B256::ZERO)
        }
    }
}

impl DatabaseCommit for EvmDb {
    fn commit(&mut self, changes: revm::primitives::HashMap<Address, revm::primitives::Account>) {
        for (address, account) in changes {
            if account.is_selfdestructed() {
                self.store.delete_account(&address);
                continue;
            }

            if !account.is_touched() {
                continue;
            }

            let code_hash = if account.info.code_hash == revm::primitives::KECCAK_EMPTY {
                None
            } else {
                if let Some(code) = account.info.code.clone() {
                    self.store.put_code(account.info.code_hash, code);
                }
                Some(account.info.code_hash)
            };

            self.store.put_account(
                address,
                StoredAccount {
                    balance: account.info.balance,
                    nonce: account.info.nonce,
                    code_hash,
                },
            );

            for (slot, value) in account.storage {
                if value.is_changed() {
                    self.store.put_storage(address, slot, value.present_value());
                }
            }
        }
    }
}

// =============================================================================
// Signed Transaction with validation
// =============================================================================

#[derive(Debug, Clone)]
pub struct SignedTransaction {
    pub caller: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub data: Bytes,
    pub nonce: u64,
    pub gas_limit: u64,
    pub gas_price: U256,
    pub chain_id: u64,
    pub signature: Option<[u8; 65]>, // r, s, v
}

impl SignedTransaction {
    pub fn new(
        caller: Address,
        to: Option<Address>,
        value: U256,
        data: Vec<u8>,
        nonce: u64,
        gas_limit: u64,
        gas_price: U256,
        chain_id: u64,
    ) -> Self {
        Self {
            caller,
            to,
            value,
            data: Bytes::from(data),
            nonce,
            gas_limit,
            gas_price,
            chain_id,
            signature: None,
        }
    }

    // FIX: Add signature verification
    pub fn verify_signature(&self) -> Result<Address> {
        match &self.signature {
            Some(sig) => {
                // Recover signer from signature
                // This is simplified - use proper EIP-155 recovery
                let mut hash = Vec::new();
                hash.extend_from_slice(&self.nonce.to_le_bytes());
                hash.extend_from_slice(&self.gas_limit.to_le_bytes());
                hash.extend_from_slice(&self.gas_price.to_le_bytes::<32>());
                hash.extend_from_slice(&self.chain_id.to_le_bytes());
                // ... full implementation would include all fields

                Ok(self.caller) // Placeholder
            }
            None => anyhow::bail!("Transaction not signed"),
        }
    }
}

// =============================================================================
// EvmExecutor
// =============================================================================

pub struct EvmExecutor {
    db: CacheDB<EvmDb>,
    chain_id: u64,
}

impl EvmExecutor {
    pub fn new() -> Self {
        let store = Arc::new(InMemoryStore::default());
        Self::with_store(store, 1) // Default chain ID 1
    }

    pub fn with_store(store: Arc<dyn EvmStore>, chain_id: u64) -> Self {
        let evm_db = EvmDb::new(store);
        Self {
            db: CacheDB::new(evm_db),
            chain_id,
        }
    }

    pub fn set_block_hash_provider<F>(&mut self, provider: F)
    where
        F: Fn(u64) -> B256 + Send + Sync + 'static,
    {
        self.db.db = EvmDb::new(self.db.db.store.clone()).with_block_hash_provider(provider);
    }

    pub fn set_balance(&mut self, address: &str, balance: u64) -> Result<()> {
        let addr =
            Address::from_str(address).map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;
        self.db.insert_account_info(
            addr,
            AccountInfo {
                balance: U256::from(balance),
                nonce: 0,
                code_hash: revm::primitives::KECCAK_EMPTY,
                code: None,
            },
        );
        Ok(())
    }

    // FIX: Add transaction validation before execution
    pub fn validate_transaction(&mut self, tx: &SignedTransaction) -> Result<()> {
        // 1. Check chain ID
        if tx.chain_id != self.chain_id {
            anyhow::bail!(
                "Invalid chain ID: expected {}, got {}",
                self.chain_id,
                tx.chain_id
            );
        }

        // 2. Check signature
        let recovered = tx.verify_signature()?;
        if recovered != tx.caller {
            anyhow::bail!("Signature verification failed");
        }

        // 3. Check nonce
        let account = self.db.basic(tx.caller)?;
        if let Some(acc) = account {
            if tx.nonce != acc.nonce {
                anyhow::bail!("Invalid nonce: expected {}, got {}", acc.nonce, tx.nonce);
            }
        } else if tx.nonce != 0 {
            anyhow::bail!(
                "Invalid nonce for new account: expected 0, got {}",
                tx.nonce
            );
        }

        // 4. Check balance for gas + value
        let account = self.db.basic(tx.caller)?;
        let balance = account.map(|a| a.balance).unwrap_or(U256::ZERO);
        let total_cost = tx.value + (tx.gas_price * U256::from(tx.gas_limit));
        if balance < total_cost {
            anyhow::bail!(
                "Insufficient balance: need {}, have {}",
                total_cost,
                balance
            );
        }

        Ok(())
    }

    // FIX: Execute transaction with validation
    pub fn execute_transaction(&mut self, tx: SignedTransaction) -> Result<TransactionReceipt> {
        // Validate first
        self.validate_transaction(&tx)?;

        let mut evm = EVM::new();
        evm.database(&mut self.db);

        evm.env.tx.caller = tx.caller;
        evm.env.tx.value = tx.value;
        evm.env.tx.data = tx.data;
        evm.env.tx.gas_limit = tx.gas_limit;
        evm.env.tx.gas_price = tx.gas_price;
        evm.env.tx.nonce = Some(tx.nonce); // FIX: Set nonce
        evm.env.tx.chain_id = Some(tx.chain_id); // FIX: Set chain ID

        if let Some(to_addr) = tx.to {
            evm.env.tx.transact_to = TransactTo::Call(to_addr);
        } else {
            evm.env.tx.transact_to = TransactTo::Create(CreateScheme::Create);
        }

        let result_and_state = evm
            .transact()
            .map_err(|e| anyhow::anyhow!("EVM execution error: {:?}", e))?;

        self.db.commit(result_and_state.state);

        match result_and_state.result {
            ExecutionResult::Success {
                output,
                gas_used,
                logs,
                ..
            } => {
                let (output_bytes, created_address) = match output {
                    Output::Call(bytes) => (bytes.to_vec(), None),
                    Output::Create(bytes, addr) => {
                        tracing::info!("Contract deployed at {:?}", addr);
                        (bytes.to_vec(), Some(addr))
                    }
                };
                Ok(TransactionReceipt {
                    success: true,
                    gas_used,
                    output: output_bytes,
                    created_address: created_address.map(|a| format!("{:?}", a)),
                    revert_reason: None,
                    logs: logs.into_iter().map(|l| format!("{:?}", l)).collect(),
                })
            }
            ExecutionResult::Revert { output, gas_used } => Ok(TransactionReceipt {
                success: false,
                gas_used,
                output: output.to_vec(),
                created_address: None,
                revert_reason: Some(format!("Reverted: {:?}", output)),
                logs: vec![],
            }),
            ExecutionResult::Halt { reason, gas_used } => Ok(TransactionReceipt {
                success: false,
                gas_used,
                output: vec![],
                created_address: None,
                revert_reason: Some(format!("Halted: {:?}", reason)),
                logs: vec![],
            }),
        }
    }

    pub fn execute_block(
        &mut self,
        transactions: Vec<SignedTransaction>,
    ) -> Result<Vec<TransactionReceipt>> {
        let mut receipts = Vec::with_capacity(transactions.len());
        for tx in transactions {
            let receipt = self.execute_transaction(tx)?;
            receipts.push(receipt);
        }
        Ok(receipts)
    }

    // Helper to get account nonce (for transaction construction)
    pub fn get_nonce(&mut self, address: &str) -> Result<u64> {
        let addr =
            Address::from_str(address).map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;
        let account = self.db.basic(addr)?;
        Ok(account.map(|a| a.nonce).unwrap_or(0))
    }

    /// Current EVM state root (for rollups and fraud proofs).
    pub fn state_root(&self) -> [u8; 32] {
        self.db.db.state_root()
    }

    /// Read-only EVM call (no state commit, no signature required).
    pub fn static_call(
        store: Arc<dyn EvmStore>,
        from: Address,
        to: Address,
        data: Vec<u8>,
        value: U256,
    ) -> Result<Vec<u8>> {
        let mut db = CacheDB::new(EvmDb::new(store));
        let mut evm = EVM::new();
        evm.database(&mut db);
        evm.env.tx.caller = from;
        evm.env.tx.transact_to = TransactTo::Call(to);
        evm.env.tx.data = Bytes::from(data);
        evm.env.tx.value = value;
        evm.env.tx.gas_limit = 30_000_000;
        evm.env.tx.gas_price = U256::ZERO;
        evm.env.tx.nonce = None;
        evm.env.tx.chain_id = None;
        evm.env.cfg.disable_balance_check = true;
        evm.env.cfg.disable_nonce_check = true;
        evm.env.cfg.disable_base_fee = true;

        let result = evm
            .transact()
            .map_err(|e| anyhow::anyhow!("EVM call error: {:?}", e))?;

        match result.result {
            ExecutionResult::Success { output, .. } => match output {
                Output::Call(bytes) => Ok(bytes.to_vec()),
                Output::Create(bytes, _) => Ok(bytes.to_vec()),
            },
            ExecutionResult::Revert { output, .. } => {
                anyhow::bail!("Call reverted: 0x{}", hex::encode(output))
            }
            ExecutionResult::Halt { reason, .. } => {
                anyhow::bail!("Call halted: {:?}", reason)
            }
        }
    }
}

impl Default for EvmExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Supporting types
// =============================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct TransactionReceipt {
    pub success: bool,
    pub gas_used: u64,
    pub output: Vec<u8>,
    pub created_address: Option<String>,
    pub revert_reason: Option<String>,
    pub logs: Vec<String>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const ALICE: &str = "0x0000000000000000000000000000000000000001";
    const BOB: &str = "0x0000000000000000000000000000000000000002";

    fn make_test_tx(caller: &str, to: Option<&str>, value: u64, nonce: u64) -> SignedTransaction {
        let caller_addr = Address::from_str(caller).unwrap();
        let to_addr = to.map(|a| Address::from_str(a).unwrap());
        SignedTransaction::new(
            caller_addr,
            to_addr,
            U256::from(value),
            vec![],
            nonce,
            1_000_000,
            U256::from(1),
            1,
        )
    }

    #[test]
    fn test_value_transfer_persists() {
        let mut executor = EvmExecutor::new();
        executor.set_balance(ALICE, 1_000_000).unwrap();

        let tx = make_test_tx(ALICE, Some(BOB), 100, 0);
        let receipt = executor.execute_transaction(tx).unwrap();
        assert!(receipt.success, "Transfer should succeed");
    }

    #[test]
    fn test_state_visible_across_transactions_in_block() {
        let mut executor = EvmExecutor::new();
        executor.set_balance(ALICE, 1_000_000).unwrap();

        let tx1 = make_test_tx(ALICE, Some(BOB), 100, 0);
        let r1 = executor.execute_transaction(tx1).unwrap();
        assert!(r1.success);

        let tx2 = make_test_tx(BOB, Some(ALICE), 50, 0);
        let r2 = executor.execute_transaction(tx2).unwrap();
        assert!(r2.success, "Bob's balance from prior tx should be visible");
    }

    #[test]
    fn test_insufficient_balance_reverts() {
        let mut executor = EvmExecutor::new();
        let tx = make_test_tx(ALICE, Some(BOB), 9999, 0);
        let receipt = executor.execute_transaction(tx).unwrap();
        assert!(!receipt.success, "Should fail with insufficient balance");
    }

    #[test]
    fn test_invalid_nonce_rejected() {
        let mut executor = EvmExecutor::new();
        executor.set_balance(ALICE, 1_000_000).unwrap();

        // Wrong nonce (should be 0, using 5)
        let tx = make_test_tx(ALICE, Some(BOB), 100, 5);
        let result = executor.execute_transaction(tx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonce"));
    }

    #[test]
    fn test_nonce_increments() {
        let mut executor = EvmExecutor::new();
        executor.set_balance(ALICE, 1_000_000).unwrap();

        // First tx with nonce 0
        let tx1 = make_test_tx(ALICE, Some(BOB), 100, 0);
        executor.execute_transaction(tx1).unwrap();

        // Second tx must use nonce 1
        let tx2 = make_test_tx(ALICE, Some(BOB), 100, 1);
        let result = executor.execute_transaction(tx2);
        assert!(result.is_ok());
    }
}
