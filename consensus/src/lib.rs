use common::traits::Consensus;
use common::types::{Block, Hash, Header};
use std::collections::HashSet;
use std::error::Error;

pub struct SimpleConsensus {
    validators: HashSet<Vec<u8>>,
}

impl SimpleConsensus {
    pub fn new(validators: Vec<Vec<u8>>) -> Self {
        let mut set = HashSet::new();
        for v in validators {
            set.insert(v);
        }
        Self { validators: set }
    }
}

impl Consensus for SimpleConsensus {
    fn verify_header(&self, header: &Header) -> Result<(), Box<dyn Error>> {
        // In a real implementation, we would:
        // 1. Serialize the header (excluding the signature).
        // 2. Recover the public key from the signature or look it up based on validator_set_id.
        // 3. Verify the signature against the serialized header.

        // For this MVP, we just check if the signature is not empty.
        if header.signature.is_empty() {
            return Err("Header signature is empty".into());
        }

        // Mock check: ensure we have validators
        if self.validators.is_empty() {
            return Err("No validators configured".into());
        }

        Ok(())
    }

    fn verify_block(&self, block: &Block) -> Result<(), Box<dyn Error>> {
        self.verify_header(&block.header)?;
        Ok(())
    }

    fn is_finalized(&self, _hash: &Hash) -> bool {
        true
    }
}

pub fn init() {
    println!("Consensus initialized (use SimpleConsensus::new)");
}
