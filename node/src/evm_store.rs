//! Persistent EVM state backed by `ChainStore`.

use execution::evm::{EvmStore, StoredAccount};
use revm::primitives::{Bytecode, B256, U256};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::sync::Arc;
use storage::ChainStore;

const PREFIX_ACCOUNT: &[u8] = b"evm:acc:";
const PREFIX_STORAGE: &[u8] = b"evm:sto:";
const PREFIX_CODE: &[u8] = b"evm:cod:";

/// RocksDB-backed EVM store using the chain state column family.
pub struct ChainStoreEvmStore {
    chain: Arc<ChainStore>,
}

impl ChainStoreEvmStore {
    pub fn new(chain: Arc<ChainStore>) -> Self {
        Self { chain }
    }

    fn key_account(address: &revm::primitives::Address) -> Vec<u8> {
        let mut k = PREFIX_ACCOUNT.to_vec();
        k.extend_from_slice(address.as_slice());
        k
    }

    fn key_storage(address: &revm::primitives::Address, slot: &U256) -> Vec<u8> {
        let mut k = PREFIX_STORAGE.to_vec();
        k.extend_from_slice(address.as_slice());
        k.extend_from_slice(slot.as_le_slice());
        k
    }

    fn key_code(code_hash: &B256) -> Vec<u8> {
        let mut k = PREFIX_CODE.to_vec();
        k.extend_from_slice(code_hash.as_slice());
        k
    }

    fn encode_account(account: &StoredAccount) -> Vec<u8> {
        let mut buf = Vec::with_capacity(65);
        buf.extend_from_slice(account.balance.as_le_slice());
        buf.extend_from_slice(&account.nonce.to_le_bytes());
        match account.code_hash {
            Some(h) => {
                buf.push(1);
                buf.extend_from_slice(h.as_slice());
            }
            None => buf.push(0),
        }
        buf
    }

    fn decode_account(bytes: &[u8]) -> Option<StoredAccount> {
        if bytes.len() < 41 {
            return None;
        }
        let balance = U256::from_le_slice(&bytes[..32]);
        let nonce = u64::from_le_bytes(bytes[32..40].try_into().ok()?);
        let code_hash = if bytes[40] == 1 && bytes.len() >= 73 {
            Some(B256::from_slice(&bytes[41..73]))
        } else {
            None
        };
        Some(StoredAccount {
            balance,
            nonce,
            code_hash,
        })
    }
}

impl EvmStore for ChainStoreEvmStore {
    fn get_account(&self, address: &revm::primitives::Address) -> Option<StoredAccount> {
        self.chain
            .get_state(&Self::key_account(address))
            .ok()
            .flatten()
            .and_then(|b| Self::decode_account(&b))
    }

    fn put_account(&self, address: revm::primitives::Address, account: StoredAccount) {
        let _ = self.chain.put_state(
            &Self::key_account(&address),
            &Self::encode_account(&account),
        );
    }

    fn get_storage(&self, address: &revm::primitives::Address, slot: &U256) -> Option<U256> {
        self.chain
            .get_state(&Self::key_storage(address, slot))
            .ok()
            .flatten()
            .map(|b| U256::from_le_slice(&b))
    }

    fn put_storage(&self, address: revm::primitives::Address, slot: U256, value: U256) {
        let _ = self
            .chain
            .put_state(&Self::key_storage(&address, &slot), value.as_le_slice());
    }

    fn get_code(&self, code_hash: &B256) -> Option<Bytecode> {
        use revm::primitives::Bytes;
        self.chain
            .get_state(&Self::key_code(code_hash))
            .ok()
            .flatten()
            .map(|b| Bytecode::new_raw(Bytes::from(b)))
    }

    fn put_code(&self, code_hash: B256, bytecode: Bytecode) {
        let _ = self
            .chain
            .put_state(&Self::key_code(&code_hash), bytecode.bytecode.as_ref());
    }

    fn delete_account(&self, address: &revm::primitives::Address) {
        let _ = self.chain.delete_state(&Self::key_account(address));
    }

    fn state_root(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        if let Ok(iter) = self.chain.iter_state() {
            let mut pairs: Vec<(Vec<u8>, Vec<u8>)> = iter.collect();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            for (k, v) in pairs {
                if k.starts_with(PREFIX_ACCOUNT)
                    || k.starts_with(PREFIX_STORAGE)
                    || k.starts_with(PREFIX_CODE)
                {
                    hasher.update(&k);
                    hasher.update(&v);
                }
            }
        }
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use execution::evm::InMemoryStore;
    use revm::primitives::Address;
    use std::sync::Arc;
    use storage::{ChainStore, MemDb};

    #[test]
    fn chain_store_evm_roundtrip() {
        let inner: Arc<dyn storage::KeyValueStore> = Arc::new(MemDb::new());
        let chain = Arc::new(ChainStore::new(inner));
        let store = ChainStoreEvmStore::new(chain);
        let addr = Address::from_slice(&[1u8; 20]);
        store.put_account(
            addr,
            StoredAccount {
                balance: U256::from(1000u64),
                nonce: 2,
                code_hash: None,
            },
        );
        let got = store.get_account(&addr).unwrap();
        assert_eq!(got.balance, U256::from(1000u64));
        assert_eq!(got.nonce, 2);
        assert_ne!(store.state_root(), [0u8; 32]);
    }

    #[test]
    fn in_memory_state_root() {
        let store = InMemoryStore::default();
        assert_eq!(store.state_root(), [0u8; 32]);
    }
}
