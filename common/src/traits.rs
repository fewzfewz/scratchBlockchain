use crate::types::{Block, Hash, Header, Transaction};
use std::error::Error;

pub trait Consensus {
    fn verify_header(&self, header: &Header) -> Result<(), Box<dyn Error>>;
    fn verify_block(&self, block: &Block) -> Result<(), Box<dyn Error>>;
    fn is_finalized(&self, hash: &Hash) -> bool;
}

pub trait Storage {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn Error>>;
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Box<dyn Error>>;
    fn contains(&self, key: &[u8]) -> Result<bool, Box<dyn Error>>;
}

pub trait Executor {
    fn execute_block(&self, block: &Block) -> Result<Hash, Box<dyn Error>>; // Returns new state root
    fn apply_transaction(&self, tx: &Transaction) -> Result<(), Box<dyn Error>>;
}

pub trait Mempool {
    fn submit(&self, tx: Transaction) -> Result<(), Box<dyn Error>>;
}
