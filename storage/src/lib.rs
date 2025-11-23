mod db;
pub mod trie;
pub mod receipt_store;

use common::traits::Storage;
use db::{KeyValueStore, MemDb};
use sled::Db;
use std::error::Error;

pub struct MemStore {
    db: Box<dyn KeyValueStore>,
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
        self.db.get(key)
    }

    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Box<dyn Error>> {
        self.db.put(key, value)
    }

    fn contains(&self, key: &[u8]) -> Result<bool, Box<dyn Error>> {
        self.db.contains(key)
    }
}

/// Persistent storage using sled key-value store
pub struct PersistentStore {
    db: Db,
}

impl PersistentStore {
    pub fn new(path: &str) -> Result<Self, Box<dyn Error>> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }
    pub fn default() -> Result<Self, Box<dyn Error>> {
        Self::new("node_db")
    }

    pub fn iter(&self) -> sled::Iter {
        self.db.iter()
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
}

/// State storage for blockchain accounts
pub struct StateStore {
    store: PersistentStore,
}

impl StateStore {
    pub fn new(path: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            store: PersistentStore::new(path)?,
        })
    }

    pub fn default() -> Result<Self, Box<dyn Error>> {
        Self::new("state_db")
    }

    pub fn get_account(&self, address: &[u8; 20]) -> Result<Option<common::types::Account>, Box<dyn Error>> {
        match self.store.get(address)? {
            Some(data) => {
                let account: common::types::Account = serde_json::from_slice(&data)?;
                Ok(Some(account))
            }
            None => Ok(None),
        }
    }

    pub fn put_account(&self, address: &[u8; 20], account: &common::types::Account) -> Result<(), Box<dyn Error>> {
        let data = serde_json::to_vec(account)?;
        self.store.put(address, &data)
    }

    pub fn get_all_accounts(&self) -> Result<std::collections::HashMap<common::types::Address, common::types::Account>, Box<dyn Error>> {
        let mut accounts = std::collections::HashMap::new();
        for item in self.store.iter() {
            let (key, value) = item?;
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
        use sha2::{Digest, Sha256};
        use common::merkle::MerkleTree;
        
        // Collect all account hashes as leaves
        let mut leaves = Vec::new();
        
        // Iterate through all keys in the store
        // For production, you'd want to maintain a sorted list of accounts
        let genesis = common::types::GenesisConfig::default();
        for genesis_account in &genesis.accounts {
            if let Some(account) = self.get_account(&genesis_account.address)? {
                // Hash the account data: address + nonce + balance
                let mut hasher = Sha256::new();
                hasher.update(&genesis_account.address);
                hasher.update(&account.nonce.to_le_bytes());
                hasher.update(&account.balance.to_le_bytes());
                leaves.push(hasher.finalize().into());
            }
        }
        
        // If no accounts, return zero hash
        if leaves.is_empty() {
            return Ok([0u8; 32]);
        }
        
        // Sort leaves for deterministic root
        leaves.sort();
        
        // Compute Merkle root
        let tree = MerkleTree::new(leaves);
        Ok(tree.root())
    }

    /// Compute state root from in-memory state
    pub fn compute_root(state: &std::collections::HashMap<common::types::Address, common::types::Account>) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        use common::merkle::MerkleTree;
        
        let mut leaves = Vec::new();
        
        for (address, account) in state {
            let mut hasher = Sha256::new();
            hasher.update(address);
            hasher.update(&account.nonce.to_le_bytes());
            hasher.update(&account.balance.to_le_bytes());
            leaves.push(hasher.finalize().into());
        }
        
        if leaves.is_empty() {
            return [0u8; 32];
        }
        
        leaves.sort();
        MerkleTree::new(leaves).root()
    }

    /// Initialize state from genesis configuration
    pub fn initialize_genesis(&self, genesis: &common::types::GenesisConfig) -> Result<(), Box<dyn Error>> {
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
    pub fn new(path: &str) -> Result<Self, Box<dyn Error>> {
        let trie_path = format!("{}/trie", path);
        let trie = trie::PatriciaTrie::new(&trie_path)?;
        Ok(Self {
            trie: std::sync::Arc::new(std::sync::Mutex::new(trie)),
        })
    }

    pub fn default() -> Result<Self, Box<dyn Error>> {
        Self::new("state_trie_db")
    }

    pub fn get_account(&self, address: &[u8; 20]) -> Result<Option<common::types::Account>, Box<dyn Error>> {
        let trie = self.trie.lock().unwrap();
        match trie.get(address)? {
            Some(data) => {
                let account: common::types::Account = serde_json::from_slice(&data)?;
                Ok(Some(account))
            }
            None => Ok(None),
        }
    }

    pub fn put_account(&self, address: &[u8; 20], account: &common::types::Account) -> Result<(), Box<dyn Error>> {
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
    pub fn compute_root(state: &std::collections::HashMap<common::types::Address, common::types::Account>) -> Result<[u8; 32], Box<dyn Error>> {
        // Create temporary trie
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path().join("temp_trie");
        let mut trie = trie::PatriciaTrie::new(temp_path.to_str().unwrap())?;
        
        // Insert all accounts
        for (address, account) in state {
            let data = serde_json::to_vec(account)?;
            trie.insert(address, &data)?;
        }
        
        Ok(trie.root_hash())
    }

    /// Initialize state from genesis configuration
    pub fn initialize_genesis(&self, genesis: &common::types::GenesisConfig) -> Result<(), Box<dyn Error>> {
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
    pub fn get_all_accounts(&self) -> Result<std::collections::HashMap<common::types::Address, common::types::Account>, Box<dyn Error>> {
        // This is a simplified implementation
        // In a real system, you'd want to iterate the trie more efficiently
        // For now, we'll return an error suggesting this isn't the best approach
        Err("get_all_accounts is not efficiently supported with trie - use specific queries instead".into())
    }
}

pub fn init() {
    println!("Storage initialized (use MemStore::new or PersistentStore::default)");
}

/// Block storage for persisting blockchain history
pub struct BlockStore {
    store: PersistentStore,
}

impl BlockStore {
    pub fn new(path: &str) -> Result<Self, Box<dyn Error>> {
        let store = PersistentStore::new(path)?;
        Ok(Self { store })
    }

    /// Store a block by its hash
    pub fn put_block(&self, block: &common::types::Block) -> Result<(), Box<dyn Error>> {
        let hash = block.hash();
        let data = serde_json::to_vec(block)?;
        self.store.put(&hash, &data)?;
        
        // Also store by height for easy retrieval
        let height_key = format!("height_{}", block.header.slot);
        self.store.put(height_key.as_bytes(), &hash)?;
        
        Ok(())
    }

    /// Get a block by its hash
    pub fn get_block_by_hash(&self, hash: &[u8; 32]) -> Result<Option<common::types::Block>, Box<dyn Error>> {
        match self.store.get(hash)? {
            Some(data) => {
                let block: common::types::Block = serde_json::from_slice(&data)?;
                Ok(Some(block))
            }
            None => Ok(None),
        }
    }

    /// Get a block by its height (slot number)
    pub fn get_block_by_height(&self, height: u64) -> Result<Option<common::types::Block>, Box<dyn Error>> {
        let height_key = format!("height_{}", height);
        match self.store.get(height_key.as_bytes())? {
            Some(hash_data) => {
                if hash_data.len() == 32 {
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(&hash_data);
                    self.get_block_by_hash(&hash)
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Get the latest block height
    pub fn get_latest_height(&self) -> Result<Option<u64>, Box<dyn Error>> {
        // Store latest height separately for quick access
        match self.store.get(b"latest_height")? {
            Some(data) => {
                if data.len() == 8 {
                    let height = u64::from_le_bytes(data.try_into().unwrap());
                    Ok(Some(height))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Update the latest block height
    pub fn set_latest_height(&self, height: u64) -> Result<(), Box<dyn Error>> {
        self.store.put(b"latest_height", &height.to_le_bytes())?;
        Ok(())
    }

    /// Mark a block as finalized
    pub fn mark_finalized(&self, height: u64) -> Result<(), Box<dyn Error>> {
        let key = format!("finalized_{}", height);
        self.store.put(key.as_bytes(), &[1u8])?;
        
        // Update latest finalized height
        if let Some(current_finalized) = self.get_latest_finalized_height()? {
            if height > current_finalized {
                self.store.put(b"latest_finalized", &height.to_le_bytes())?;
            }
        } else {
            self.store.put(b"latest_finalized", &height.to_le_bytes())?;
        }
        
        Ok(())
    }

    /// Check if a block is finalized
    pub fn is_finalized(&self, height: u64) -> Result<bool, Box<dyn Error>> {
        let key = format!("finalized_{}", height);
        Ok(self.store.contains(key.as_bytes())?)
    }

    /// Get the latest finalized block height
    pub fn get_latest_finalized_height(&self) -> Result<Option<u64>, Box<dyn Error>> {
        match self.store.get(b"latest_finalized")? {
            Some(data) => {
                if data.len() == 8 {
                    let height = u64::from_le_bytes(data.try_into().unwrap());
                    Ok(Some(height))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::traits::Storage;
    use tempfile;

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
    fn test_persistent_store_put_get() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db");
        let store = PersistentStore::new(path.to_str().unwrap()).unwrap();
        
        let key = b"p_key";
        let value = b"p_value";
        store.put(key, value).unwrap();
        let retrieved = store.get(key).unwrap();
        assert_eq!(retrieved, Some(value.to_vec()));
    }

    #[test]
    fn test_persistent_store_contains() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db");
        let store = PersistentStore::new(path.to_str().unwrap()).unwrap();
        
        let key = b"p_key";
        assert!(!store.contains(key).unwrap());
        store.put(key, b"v").unwrap();
        assert!(store.contains(key).unwrap());
    }

    #[test]
    fn test_state_store() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state_db");
        let store = StateStore::new(path.to_str().unwrap()).unwrap();
        
        let address = [1u8; 20];
        let account = common::types::Account::new(1000);
        
        store.put_account(&address, &account).unwrap();
        let retrieved = store.get_account(&address).unwrap();
        
        assert_eq!(retrieved, Some(account));
    }

    #[test]
    fn test_genesis_initialization() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("genesis_state_db");
        let store = StateStore::new(path.to_str().unwrap()).unwrap();
        
        let genesis = common::types::GenesisConfig::default();
        store.initialize_genesis(&genesis).unwrap();
        
        // Verify genesis accounts were created
        for genesis_account in &genesis.accounts {
            let account = store.get_account(&genesis_account.address).unwrap();
            assert!(account.is_some());
            let account = account.unwrap();
            assert_eq!(account.balance, genesis_account.balance);
            assert_eq!(account.nonce, 0);
        }
    }

    #[test]
    fn test_block_store() {
        use common::types::{Block, Header};
        
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_block_db");
        let block_store = BlockStore::new(path.to_str().unwrap()).unwrap();
        
        // Create a test block
        let header = Header::new([0u8; 32], 1);
        let block = Block::new(header, vec![]);
        
        // Store the block
        block_store.put_block(&block).unwrap();
        block_store.set_latest_height(1).unwrap();
        
        // Retrieve by hash
        let hash = block.hash();
        let retrieved = block_store.get_block_by_hash(&hash).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().header.slot, 1);
        
        // Retrieve by height
        let by_height = block_store.get_block_by_height(1).unwrap();
        assert!(by_height.is_some());
        assert_eq!(by_height.unwrap().header.slot, 1);
        
        // Check latest height
        let latest = block_store.get_latest_height().unwrap();
        assert_eq!(latest, Some(1));
    }

    #[test]
    fn test_block_store_multiple_blocks() {
        use common::types::{Block, Header};
        
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_multi_block_db");
        let block_store = BlockStore::new(path.to_str().unwrap()).unwrap();
        
        // Store multiple blocks
        for i in 0..5 {
            let header = Header::new([i as u8; 32], i);
            let block = Block::new(header, vec![]);
            block_store.put_block(&block).unwrap();
            block_store.set_latest_height(i).unwrap();
        }
        
        // Retrieve each block by height
        for i in 0..5 {
            let block = block_store.get_block_by_height(i).unwrap();
            assert!(block.is_some());
            assert_eq!(block.unwrap().header.slot, i);
        }
        
        // Check latest height
        assert_eq!(block_store.get_latest_height().unwrap(), Some(4));
    }

    #[test]
    fn test_block_finality() {
        use common::types::{Block, Header};
        
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_finality_db");
        let block_store = BlockStore::new(path.to_str().unwrap()).unwrap();
        
        // Store a block
        let header = Header::new([0u8; 32], 1);
        let block = Block::new(header, vec![]);
        block_store.put_block(&block).unwrap();
        
        // Initially not finalized
        assert!(!block_store.is_finalized(1).unwrap());
        assert_eq!(block_store.get_latest_finalized_height().unwrap(), None);
        
        // Mark as finalized
        block_store.mark_finalized(1).unwrap();
        
        // Now finalized
        assert!(block_store.is_finalized(1).unwrap());
        assert_eq!(block_store.get_latest_finalized_height().unwrap(), Some(1));
        
        // Add and finalize another block
        let header2 = Header::new([1u8; 32], 2);
        let block2 = Block::new(header2, vec![]);
        block_store.put_block(&block2).unwrap();
        block_store.mark_finalized(2).unwrap();
        
        // Both finalized, latest is 2
        assert!(block_store.is_finalized(1).unwrap());
        assert!(block_store.is_finalized(2).unwrap());
        assert_eq!(block_store.get_latest_finalized_height().unwrap(), Some(2));
    }
    
    #[test]
    fn test_trie_state_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = TrieStateStore::new(dir.path().to_str().unwrap()).unwrap();
        
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
        let dir = tempfile::tempdir().unwrap();
        let store = TrieStateStore::new(dir.path().to_str().unwrap()).unwrap();
        
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
            common::types::Account { nonce: 1, balance: 100 },
        );
        state.insert(
            [2u8; 20],
            common::types::Account { nonce: 2, balance: 200 },
        );
        
        let root = TrieStateStore::compute_root(&state).unwrap();
        assert_ne!(root, [0u8; 32]);
    }
}
