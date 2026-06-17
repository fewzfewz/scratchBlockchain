use common::types::{Hash, TransactionReceipt};
use crate::db::{KeyValueStore, ColumnFamily};
use std::error::Error;
use std::sync::Arc;

pub struct ReceiptStore {
    db: Arc<dyn KeyValueStore>,
}

impl ReceiptStore {
    pub fn new(db: Arc<dyn KeyValueStore>) -> Self {
        Self { db }
    }

    pub fn put_receipt(&self, receipt: &TransactionReceipt) -> Result<(), Box<dyn Error>> {
        let key = receipt.tx_hash;
        let value = bincode::serialize(receipt)?;
        self.db.put(ColumnFamily::Receipts, &key, &value)?;
        Ok(())
    }

    pub fn get_receipt(&self, tx_hash: &Hash) -> Result<Option<TransactionReceipt>, Box<dyn Error>> {
        match self.db.get(ColumnFamily::Receipts, tx_hash)? {
            Some(bytes) => {
                let receipt: TransactionReceipt = bincode::deserialize(&bytes)?;
                Ok(Some(receipt))
            }
            None => Ok(None),
        }
    }

    pub fn has_receipt(&self, tx_hash: &Hash) -> Result<bool, Box<dyn Error>> {
        Ok(self.db.contains(ColumnFamily::Receipts, tx_hash)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::types::ExecutionStatus;
    use crate::db::MemDb;

    #[test]
    fn test_receipt_store() {
        let db = Arc::new(MemDb::new());
        let store = ReceiptStore::new(db);

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
