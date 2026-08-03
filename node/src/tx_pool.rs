//! Unified transaction pool: MEV-protected mempool + account abstraction bundler.

use common::types::Transaction;
use execution::account_abstraction::{AccountAbstractionExecutor, UserOperation};
use mempool::{MempoolConfig, MevMempool};
use mev::{DecryptionShare, EncryptedTransaction};
use std::sync::{Arc, Mutex};

const AA_MAX_BUNDLE: usize = 32;

pub struct TxPool {
    mev: Arc<Mutex<MevMempool>>,
    aa: Arc<Mutex<AccountAbstractionExecutor>>,
}

impl TxPool {
    pub fn new(config: MempoolConfig, validator_pubkeys: Vec<Vec<u8>>) -> Self {
        Self {
            mev: Arc::new(Mutex::new(MevMempool::new(config, validator_pubkeys))),
            aa: Arc::new(Mutex::new(AccountAbstractionExecutor::new(AA_MAX_BUNDLE))),
        }
    }

    pub fn add_transaction(&self, tx: Transaction) -> Result<(), String> {
        self.mev
            .lock()
            .unwrap()
            .add_transaction(tx)
            .map_err(|e| e.to_string())
    }

    pub fn remove_transactions(&self, txs: &[Transaction]) {
        self.mev.lock().unwrap().remove_transactions(txs);
    }

    pub fn size(&self) -> usize {
        self.mev.lock().unwrap().size() + self.aa.lock().unwrap().pending_operations()
    }

    pub fn mempool_snapshot(&self, max: usize) -> Vec<Transaction> {
        self.mev.lock().unwrap().get_transactions(max)
    }

    /// Transactions ready for block inclusion: AA bundles first, then MEV-ready pool.
    pub fn get_transactions_for_block(&self, max: usize) -> Vec<Transaction> {
        let mut out = Vec::new();
        if max == 0 {
            return out;
        }
        {
            let mut aa = self.aa.lock().unwrap();
            out.extend(aa.get_bundled_transactions(max));
        }
        let remaining = max.saturating_sub(out.len());
        if remaining > 0 {
            out.extend(
                self.mev
                    .lock()
                    .unwrap()
                    .get_all_ready_transactions(remaining),
            );
        }
        out
    }

    pub fn submit_user_operation(&self, op: UserOperation) -> Result<[u8; 32], String> {
        let hash = op.hash();
        self.aa
            .lock()
            .unwrap()
            .submit_operation(op)
            .map_err(|e| e.to_string())?;
        Ok(hash)
    }

    pub fn pending_user_operations(&self) -> usize {
        self.aa.lock().unwrap().pending_operations()
    }

    pub fn submit_committed(
        &self,
        tx_hash: [u8; 32],
        secret: [u8; 32],
        sender: [u8; 20],
        nonce: u64,
        current_height: u64,
    ) -> [u8; 32] {
        self.mev
            .lock()
            .unwrap()
            .submit_committed(tx_hash, secret, sender, nonce, current_height)
    }

    pub fn reveal_transaction(
        &self,
        tx: Transaction,
        secret: [u8; 32],
        commitment: [u8; 32],
        current_height: u64,
    ) -> Result<Transaction, String> {
        self.mev
            .lock()
            .unwrap()
            .reveal_transaction(tx, secret, commitment, current_height)
    }

    pub fn submit_encrypted(&self, encrypted: EncryptedTransaction) -> Result<(), String> {
        self.mev.lock().unwrap().submit_encrypted(encrypted)
    }

    pub fn submit_decryption_share(&self, share: DecryptionShare) -> Result<(), String> {
        self.mev.lock().unwrap().submit_decryption_share(share)
    }
}
