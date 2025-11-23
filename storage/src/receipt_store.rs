use anyhow::Result;
use common::types::{Hash, TransactionReceipt};
use sled::Db;
use std::path::Path;

pub struct ReceiptStore {
    db: Db,
}

impl ReceiptStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Store a transaction receipt
    pub fn put_receipt(&self, receipt: &TransactionReceipt) -> Result<()> {
        let key = receipt.tx_hash;
        let value = bincode::serialize(receipt)?;
        self.db.insert(key, value)?;
        self.db.flush()?;
        Ok(())
    }

    /// Get a transaction receipt by hash
    pub fn get_receipt(&self, tx_hash: &Hash) -> Result<Option<TransactionReceipt>> {
        match self.db.get(tx_hash)? {
            Some(bytes) => {
                let receipt: TransactionReceipt = bincode::deserialize(&bytes)?;
                Ok(Some(receipt))
            }
            None => Ok(None),
        }
    }

    /// Check if a receipt exists
    pub fn has_receipt(&self, tx_hash: &Hash) -> Result<bool> {
        Ok(self.db.contains_key(tx_hash)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::types::ExecutionStatus;
    use tempfile::tempdir;

    #[test]
    fn test_receipt_store() {
        let dir = tempdir().unwrap();
        let store = ReceiptStore::new(dir.path().join("receipts")).unwrap();

        let receipt = TransactionReceipt::new(
            [1u8; 32],
            [2u8; 32],
            100,
            0,
            21000,
            21000,
            ExecutionStatus::Success,
            [3u8; 20],
            Some([4u8; 20]),
        );

        store.put_receipt(&receipt).unwrap();
        let retrieved = store.get_receipt(&[1u8; 32]).unwrap().unwrap();
        
        assert_eq!(retrieved.tx_hash, receipt.tx_hash);
        assert_eq!(retrieved.block_height, 100);
        assert_eq!(retrieved.gas_used, 21000);
    }
}
