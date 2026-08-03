//! WASM contract registry persisted in chain state.

use std::sync::Arc;
use storage::ChainStore;

const PREFIX: &[u8] = b"wasm:code:";

pub struct WasmRegistry {
    chain: Arc<ChainStore>,
}

impl WasmRegistry {
    pub fn new(chain: Arc<ChainStore>) -> Self {
        Self { chain }
    }

    fn key(name: &str) -> Vec<u8> {
        let mut k = PREFIX.to_vec();
        k.extend_from_slice(name.as_bytes());
        k
    }

    pub fn deploy(&self, name: &str, wasm: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        if name.is_empty() || name.len() > 64 {
            return Err("Invalid contract name".into());
        }
        if wasm.is_empty() || wasm.len() > 512 * 1024 {
            return Err("WASM module too large (max 512KB)".into());
        }
        self.chain.put_state(&Self::key(name), wasm)?;
        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        self.chain.get_state(&Self::key(name))
    }

    pub fn list(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut names = Vec::new();
        for (key, _) in self.chain.iter_state()? {
            if key.starts_with(PREFIX) {
                if let Ok(name) = std::str::from_utf8(&key[PREFIX.len()..]) {
                    names.push(name.to_string());
                }
            }
        }
        names.sort();
        Ok(names)
    }
}
