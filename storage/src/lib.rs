mod db;
pub mod pruner;
pub mod receipt_store;
pub mod trie;

use common::traits::Storage;
use db::KeyValueStore;
use std::error::Error;
use std::sync::Arc;

pub use db::{ChainStore, ColumnFamily, DbError, DbMetrics, MemDb, WriteBatch};
pub use pruner::{PruneConfig, PruneStats};

pub struct MemStore {
    db: Box<dyn KeyValueStore>,
}

impl Default for MemStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemStore {
    pub fn new() -> Self {
        Self {
            db: Box::new(MemDb::new()),
        }
    }
}

impl Storage for MemStore {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        self.db.get(ColumnFamily::State, key)
    }

    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Box<dyn Error>> {
        self.db.put(ColumnFamily::State, key, value)
    }

    fn contains(&self, key: &[u8]) -> Result<bool, Box<dyn Error>> {
        self.db.contains(ColumnFamily::State, key)
    }

    fn delete(&self, key: &[u8]) -> Result<(), Box<dyn Error>> {
        self.db.delete(ColumnFamily::State, key)
    }

    fn write_batch(
        &self,
        operations: Vec<(Vec<u8>, Option<Vec<u8>>)>,
    ) -> Result<(), Box<dyn Error>> {
        use crate::db::WriteBatch;
        let mut batch = WriteBatch::new();
        for (key, value) in operations {
            match value {
                Some(v) => batch.put(ColumnFamily::State, key, v),
                None => batch.delete(ColumnFamily::State, key),
            }
        }
        self.db.write_batch(batch)
    }
}

#[cfg(feature = "sled-legacy")]
mod sled_legacy {
    use super::*;
    use common::traits::Storage;
    use sled::Db;
    use std::error::Error;

    /// Persistent storage using sled key-value store
    pub struct PersistentStore {
        db: Db,
    }

    impl PersistentStore {
        pub fn new(path: &str) -> Result<Self, Box<dyn Error>> {
            let db = sled::open(path)?;
            Ok(Self { db })
        }

        pub fn iter(&self) -> sled::Iter {
            self.db.iter()
        }
    }

    impl Default for PersistentStore {
        fn default() -> Self {
            Self::new("node_db").unwrap()
        }
    }

    impl Storage for PersistentStore {
        fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
            match self.db.get(key)? {
                Some(ivec) => Ok(Some(ivec.to_vec())),
                None => Ok(None),
            }
        }
        fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Box<dyn Error>> {
            self.db.insert(key, value)?;
            self.db.flush()?;
            Ok(())
        }
        fn contains(&self, key: &[u8]) -> Result<bool, Box<dyn Error>> {
            Ok(self.db.contains_key(key)?)
        }
        fn delete(&self, key: &[u8]) -> Result<(), Box<dyn Error>> {
            self.db.remove(key)?;
            self.db.flush()?;
            Ok(())
        }
        fn write_batch(
            &self,
            operations: Vec<(Vec<u8>, Option<Vec<u8>>)>,
        ) -> Result<(), Box<dyn Error>> {
            for (key, value) in operations {
                match value {
                    Some(v) => {
                        self.db.insert(key, v)?;
                    }
                    None => {
                        self.db.remove(key)?;
                    }
                }
            }
            self.db.flush()?;
            Ok(())
        }
    }
}

#[cfg(feature = "sled-legacy")]
pub use sled_legacy::PersistentStore;

/// State storage using RocksDB (or fallback to MemDb)
pub struct StateStore {
    chain: std::sync::Arc<ChainStore>,
}

impl StateStore {
    pub fn open(path: &str) -> Result<Self, Box<dyn Error>> {
        let store = ChainStore::open(path)?;
        Ok(Self {
            chain: std::sync::Arc::new(store),
        })
    }

    pub fn new_mem() -> Self {
        Self {
            chain: std::sync::Arc::new(ChainStore::new(std::sync::Arc::new(MemDb::new()))),
        }
    }

    pub fn get_account(
        &self,
        address: &[u8; 20],
    ) -> Result<Option<common::types::Account>, Box<dyn Error>> {
        match self.chain.get_state(address)? {
            Some(data) => {
                let account: common::types::Account = serde_json::from_slice(&data)?;
                Ok(Some(account))
            }
            None => Ok(None),
        }
    }

    pub fn put_account(
        &self,
        address: &[u8; 20],
        account: &common::types::Account,
    ) -> Result<(), Box<dyn Error>> {
        let data = serde_json::to_vec(account)?;
        self.chain.put_state(address, &data)
    }

    pub fn get_all_accounts(
        &self,
    ) -> Result<
        std::collections::HashMap<common::types::Address, common::types::Account>,
        Box<dyn Error>,
    > {
        let mut accounts = std::collections::HashMap::new();
        for (key, value) in self.chain.iter_state()? {
            if key.len() == 20 {
                let mut address = [0u8; 20];
                address.copy_from_slice(&key);
                let account: common::types::Account = serde_json::from_slice(&value)?;
                accounts.insert(address, account);
            }
        }
        Ok(accounts)
    }

    pub fn root_hash(&self) -> Result<[u8; 32], Box<dyn Error>> {
        use common::merkle::MerkleTree;
        use sha2::{Digest, Sha256};

        let mut leaves = Vec::new();
        let mut iter = self.chain.iter_state()?;
        while let Some((key, value)) = iter.next() {
            if key.len() == 20 {
                let account: common::types::Account = serde_json::from_slice(&value)?;
                let mut hasher = Sha256::new();
                hasher.update(&key);
                hasher.update(account.nonce.to_le_bytes());
                hasher.update(account.balance.to_le_bytes());
                leaves.push(hasher.finalize().into());
            }
        }

        if leaves.is_empty() {
            return Ok([0u8; 32]);
        }
        leaves.sort();
        let tree = MerkleTree::new(leaves);
        Ok(tree.root())
    }

    pub fn compute_root(
        state: &std::collections::HashMap<common::types::Address, common::types::Account>,
    ) -> [u8; 32] {
        use common::merkle::MerkleTree;
        use sha2::{Digest, Sha256};

        let mut leaves = Vec::new();
        for (address, account) in state {
            let mut hasher = Sha256::new();
            hasher.update(address);
            hasher.update(account.nonce.to_le_bytes());
            hasher.update(account.balance.to_le_bytes());
            leaves.push(hasher.finalize().into());
        }
        if leaves.is_empty() {
            return [0u8; 32];
        }
        leaves.sort();
        MerkleTree::new(leaves).root()
    }

    pub fn initialize_genesis(
        &self,
        genesis: &common::types::GenesisConfig,
    ) -> Result<(), Box<dyn Error>> {
        for genesis_account in &genesis.accounts {
            let account = common::types::Account {
                nonce: 0,
                balance: genesis_account.balance,
            };
            self.put_account(&genesis_account.address, &account)?;
        }
        Ok(())
    }
}

/// State storage using Patricia Merkle Trie
pub struct TrieStateStore {
    trie: std::sync::Arc<std::sync::Mutex<trie::PatriciaTrie>>,
}

impl TrieStateStore {
    pub fn new(db: Arc<dyn KeyValueStore>) -> Result<Self, Box<dyn Error>> {
        let trie = trie::PatriciaTrie::new(db)?;
        Ok(Self {
            trie: std::sync::Arc::new(std::sync::Mutex::new(trie)),
        })
    }
}

impl Default for TrieStateStore {
    fn default() -> Self {
        Self::new(Arc::new(MemDb::new())).unwrap()
    }
}

impl TrieStateStore {
    pub fn get_account(
        &self,
        address: &[u8; 20],
    ) -> Result<Option<common::types::Account>, Box<dyn Error>> {
        let trie = self.trie.lock().unwrap();
        match trie.get(address)? {
            Some(data) => {
                let account: common::types::Account = serde_json::from_slice(&data)?;
                Ok(Some(account))
            }
            None => Ok(None),
        }
    }

    pub fn put_account(
        &self,
        address: &[u8; 20],
        account: &common::types::Account,
    ) -> Result<(), Box<dyn Error>> {
        let data = serde_json::to_vec(account)?;
        let mut trie = self.trie.lock().unwrap();
        trie.insert(address, &data)
    }

    pub fn root_hash(&self) -> Result<[u8; 32], Box<dyn Error>> {
        let trie = self.trie.lock().unwrap();
        Ok(trie.root_hash())
    }

    pub fn get_proof(&self, address: &[u8; 20]) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        let trie = self.trie.lock().unwrap();
        trie.get_proof(address)
    }

    pub fn delete_account(&self, address: &[u8; 20]) -> Result<(), Box<dyn Error>> {
        let mut trie = self.trie.lock().unwrap();
        trie.delete(address)
    }

    /// Compute state root from in-memory state
    pub fn compute_root(
        state: &std::collections::HashMap<common::types::Address, common::types::Account>,
    ) -> Result<[u8; 32], Box<dyn Error>> {
        // Create temporary trie
        let mut trie = trie::PatriciaTrie::new(Arc::new(MemDb::new()))?;

        // Insert all accounts
        for (address, account) in state {
            let data = serde_json::to_vec(account)?;
            trie.insert(address, &data)?;
        }

        Ok(trie.root_hash())
    }

    /// Initialize state from genesis configuration
    pub fn initialize_genesis(
        &self,
        genesis: &common::types::GenesisConfig,
    ) -> Result<(), Box<dyn Error>> {
        for genesis_account in &genesis.accounts {
            let account = common::types::Account {
                nonce: 0,
                balance: genesis_account.balance,
            };
            self.put_account(&genesis_account.address, &account)?;
        }
        Ok(())
    }

    /// Get all accounts (expensive operation - iterates entire trie)
    /// Note: This is less efficient than the old implementation
    /// In production, consider maintaining a separate index
    pub fn get_all_accounts(
        &self,
    ) -> Result<
        std::collections::HashMap<common::types::Address, common::types::Account>,
        Box<dyn Error>,
    > {
        // This is a simplified implementation
        // In a real system, you'd want to iterate the trie more efficiently
        // For now, we'll return an error suggesting this isn't the best approach
        Err("get_all_accounts is not efficiently supported with trie - use specific queries instead".into())
    }
}

pub fn init() {
    println!("Storage initialized — use ChainStore::open or MemStore::new");
}

/// Block storage (sled-backed, available with `sled-legacy` feature)
#[cfg(feature = "sled-legacy")]
pub mod block_store {
    use super::sled_legacy::PersistentStore;
    use common::traits::Storage;
    use std::error::Error;

    pub struct BlockStore {
        store: PersistentStore,
    }

    impl BlockStore {
        pub fn new(path: &str) -> Result<Self, Box<dyn Error>> {
            let store = PersistentStore::new(path)?;
            Ok(Self { store })
        }

        pub fn put_block(&self, block: &common::types::Block) -> Result<(), Box<dyn Error>> {
            // ... (identical to original)
            let hash = block.hash();
            let data = serde_json::to_vec(block)?;
            self.store.put(&hash, &data)?;
            let height_key = format!("height_{}", block.header.slot);
            self.store.put(height_key.as_bytes(), &hash)?;
            Ok(())
        }

        pub fn get_block_by_hash(
            &self,
            hash: &[u8; 32],
        ) -> Result<Option<common::types::Block>, Box<dyn Error>> {
            match self.store.get(hash)? {
                Some(data) => {
                    let block: common::types::Block = serde_json::from_slice(&data)?;
                    Ok(Some(block))
                }
                None => Ok(None),
            }
        }

        pub fn get_block_by_height(
            &self,
            height: u64,
        ) -> Result<Option<common::types::Block>, Box<dyn Error>> {
            let height_key = format!("height_{}", height);
            match self.store.get(height_key.as_bytes())? {
                Some(hash_data) if hash_data.len() == 32 => {
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(&hash_data);
                    self.get_block_by_hash(&hash)
                }
                _ => Ok(None),
            }
        }

        pub fn get_latest_height(&self) -> Result<Option<u64>, Box<dyn Error>> {
            match self.store.get(b"latest_height")? {
                Some(data) if data.len() == 8 => {
                    Ok(Some(u64::from_le_bytes(data.try_into().unwrap())))
                }
                _ => Ok(None),
            }
        }

        pub fn set_latest_height(&self, height: u64) -> Result<(), Box<dyn Error>> {
            self.store.put(b"latest_height", &height.to_le_bytes())?;
            Ok(())
        }

        pub fn mark_finalized(&self, height: u64) -> Result<(), Box<dyn Error>> {
            let key = format!("finalized_{}", height);
            self.store.put(key.as_bytes(), &[1u8])?;
            if let Some(current) = self.get_latest_finalized_height()? {
                if height > current {
                    self.store.put(b"latest_finalized", &height.to_le_bytes())?;
                }
            } else {
                self.store.put(b"latest_finalized", &height.to_le_bytes())?;
            }
            Ok(())
        }

        pub fn is_finalized(&self, height: u64) -> Result<bool, Box<dyn Error>> {
            let key = format!("finalized_{}", height);
            self.store.contains(key.as_bytes())
        }

        pub fn get_latest_finalized_height(&self) -> Result<Option<u64>, Box<dyn Error>> {
            match self.store.get(b"latest_finalized")? {
                Some(data) if data.len() == 8 => {
                    Ok(Some(u64::from_le_bytes(data.try_into().unwrap())))
                }
                _ => Ok(None),
            }
        }
    }
}

#[cfg(feature = "sled-legacy")]
pub use block_store::BlockStore;

#[cfg(test)]
mod tests {
    use super::*;
    use common::traits::Storage;

    #[test]
    fn test_memstore_put_get() {
        let store = MemStore::new();
        let key = b"test_key";
        let value = b"test_value";

        store.put(key, value).unwrap();
        let retrieved = store.get(key).unwrap();

        assert_eq!(retrieved, Some(value.to_vec()));
    }

    #[test]
    fn test_memstore_contains() {
        let store = MemStore::new();
        let key = b"test_key";

        assert!(!store.contains(key).unwrap());
        store.put(key, b"value").unwrap();
        assert!(store.contains(key).unwrap());
    }

    #[test]
    fn test_memstore_get_nonexistent() {
        let store = MemStore::new();
        let result = store.get(b"nonexistent").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_state_store_mem() {
        let store = StateStore::new_mem();
        let address = [1u8; 20];
        let account = common::types::Account::new(1000);

        store.put_account(&address, &account).unwrap();
        let retrieved = store.get_account(&address).unwrap();

        assert_eq!(retrieved, Some(account));
    }

    #[test]
    fn test_genesis_initialization_mem() {
        let store = StateStore::new_mem();
        let genesis = common::types::GenesisConfig::default();
        store.initialize_genesis(&genesis).unwrap();

        for genesis_account in &genesis.accounts {
            let account = store.get_account(&genesis_account.address).unwrap();
            assert!(account.is_some());
            let account = account.unwrap();
            assert_eq!(account.balance, genesis_account.balance);
            assert_eq!(account.nonce, 0);
        }
    }

    #[test]
    fn test_trie_state_store() {
        let store = TrieStateStore::new(Arc::new(MemDb::new())).unwrap();

        let address = [1u8; 20];
        let account = common::types::Account {
            nonce: 5,
            balance: 1000,
        };

        store.put_account(&address, &account).unwrap();
        let retrieved = store.get_account(&address).unwrap();

        assert_eq!(retrieved, Some(account));

        // Test root hash
        let root = store.root_hash().unwrap();
        assert_ne!(root, [0u8; 32]); // Should not be empty
    }

    #[test]
    fn test_trie_state_store_proof() {
        let store = TrieStateStore::new(Arc::new(MemDb::new())).unwrap();

        let address = [2u8; 20];
        let account = common::types::Account {
            nonce: 10,
            balance: 5000,
        };

        store.put_account(&address, &account).unwrap();

        // Get proof
        let proof = store.get_proof(&address).unwrap();
        assert!(!proof.is_empty());
    }

    #[test]
    fn test_trie_compute_root() {
        let mut state = std::collections::HashMap::new();
        state.insert(
            [1u8; 20],
            common::types::Account {
                nonce: 1,
                balance: 100,
            },
        );
        state.insert(
            [2u8; 20],
            common::types::Account {
                nonce: 2,
                balance: 200,
            },
        );

        let root = TrieStateStore::compute_root(&state).unwrap();
        assert_ne!(root, [0u8; 32]);
    }
}
