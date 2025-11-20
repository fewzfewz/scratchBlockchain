use serde::{Deserialize, Serialize};

// Type aliases for now, can be replaced with specific types later
pub type Hash = [u8; 32];
pub type Address = [u8; 20];
pub type Signature = Vec<u8>; // Changed from [u8; 64] to avoid serde limit

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub parent_hash: Hash,
    pub state_root: Hash,
    pub extrinsics_root: Hash,
    pub slot: u64,
    pub epoch: u64,
    pub validator_set_id: u64,
    pub signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub sender: Address,
    pub nonce: u64,
    pub payload: Vec<u8>,
    pub signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: Header,
    pub extrinsics: Vec<Transaction>,
}
