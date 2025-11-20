mod db;

use common::traits::Storage;
use db::{KeyValueStore, MemDb};
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

pub fn init() {
    println!("Storage initialized (use MemStore::new)");
}

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
}
