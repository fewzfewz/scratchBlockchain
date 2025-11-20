use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, RwLock};

pub trait KeyValueStore: Send + Sync {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn Error>>;
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Box<dyn Error>>;
    fn contains(&self, key: &[u8]) -> Result<bool, Box<dyn Error>>;
}

#[derive(Clone)]
pub struct MemDb {
    store: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
}

impl MemDb {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl KeyValueStore for MemDb {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        let read_guard = self.store.read().map_err(|_| "RwLock poisoned")?;
        Ok(read_guard.get(key).cloned())
    }

    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut write_guard = self.store.write().map_err(|_| "RwLock poisoned")?;
        write_guard.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn contains(&self, key: &[u8]) -> Result<bool, Box<dyn Error>> {
        let read_guard = self.store.read().map_err(|_| "RwLock poisoned")?;
        Ok(read_guard.contains_key(key))
    }
}
