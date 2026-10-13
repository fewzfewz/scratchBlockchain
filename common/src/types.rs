use ed25519_dalek::{Signature as Ed25519Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// Type aliases for now, can be replaced with specific types later
pub type Hash = [u8; 32];
pub type Address = [u8; 20];
pub type Signature = Vec<u8>; // Changed from [u8; 64] to avoid serde limit
pub type PublicKey = [u8; 32];

/// Domain tags keep transaction and header hashes from ever colliding with each
/// other (or with any other signed message), and version the encodings.
pub const TX_DOMAIN: &[u8] = b"NEBULA_TX_V1";
pub const HEADER_DOMAIN: &[u8] = b"NEBULA_HEADER_V1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Header {
    pub parent_hash: Hash,
    pub state_root: Hash,
    pub extrinsics_root: Hash,
    pub slot: u64,
    pub epoch: u64,
    pub validator_set_id: u64,
    pub signature: Signature,
    pub gas_used: u64, // Total gas used in block
    pub base_fee: u64, // Base fee per gas (EIP-1559)
}

impl Header {
    pub fn new(parent_hash: Hash, slot: u64) -> Self {
        Self {
            parent_hash,
            state_root: [0; 32],
            extrinsics_root: [0; 32],
            slot,
            epoch: 0,
            validator_set_id: 0,
            signature: vec![],
            gas_used: 0,
            base_fee: 1_000_000_000, // 1 Gwei default
        }
    }

    /// Canonical, domain-separated header hash. Covers every field except the
    /// proposer's signature (which is computed over this hash).
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(HEADER_DOMAIN);
        hasher.update(self.parent_hash);
        hasher.update(self.state_root);
        hasher.update(self.extrinsics_root);
        hasher.update(self.slot.to_le_bytes());
        hasher.update(self.epoch.to_le_bytes());
        hasher.update(self.validator_set_id.to_le_bytes());
        hasher.update(self.gas_used.to_le_bytes());
        hasher.update(self.base_fee.to_le_bytes());
        hasher.finalize().into()
    }
}

/// Transaction structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transaction {
    pub sender: Address,
    pub nonce: u64,
    /// Ed25519 key that signed the transaction; `sender` must be derived from it.
    pub public_key: PublicKey,
    pub payload: Vec<u8>,
    pub signature: Vec<u8>,

    // Gas fields (EIP-1559 style)
    pub gas_limit: u64,
    pub max_fee_per_gas: u64,
    pub max_priority_fee_per_gas: u64,

    // Optional fields
    pub chain_id: Option<u64>, // For replay protection
    pub to: Option<Address>,   // None for contract creation
    pub value: u64,            // Amount to transfer
}

impl Transaction {
    /// Canonical transaction hash (the message that gets signed).
    ///
    /// Every variable-length field is length-prefixed and every optional field
    /// carries a presence tag, so no two distinct transactions share an encoding
    /// (e.g. `chain_id` cannot be stripped by shifting bytes into `payload`).
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(TX_DOMAIN);
        hasher.update(self.sender);
        hasher.update(self.nonce.to_le_bytes());
        hasher.update(self.public_key);
        hasher.update((self.payload.len() as u64).to_le_bytes());
        hasher.update(&self.payload);
        hasher.update(self.gas_limit.to_le_bytes());
        hasher.update(self.max_fee_per_gas.to_le_bytes());
        hasher.update(self.max_priority_fee_per_gas.to_le_bytes());
        match self.chain_id {
            Some(chain_id) => {
                hasher.update([1u8]);
                hasher.update(chain_id.to_le_bytes());
            }
            None => hasher.update([0u8]),
        }
        match self.to {
            Some(to) => {
                hasher.update([1u8]);
                hasher.update(to);
            }
            None => hasher.update([0u8]),
        }
        hasher.update(self.value.to_le_bytes());
        hasher.finalize().into()
    }

    /// Verify transaction signature using the public key derived from sender address
    pub fn verify(&self, public_key: &PublicKey) -> bool {
        if self.signature.len() != 64 {
            return false;
        }

        let Ok(verifying_key) = VerifyingKey::from_bytes(public_key) else {
            return false;
        };

        let Ok(signature) = Ed25519Signature::from_slice(&self.signature) else {
            return false;
        };

        let message = self.hash();
        verifying_key.verify(&message, &signature).is_ok()
    }

    /// Set `public_key` and `sender` from `key`, then sign `self.hash()`.
    pub fn sign_with(&mut self, key: &ed25519_dalek::SigningKey) {
        use ed25519_dalek::Signer;
        self.public_key = key.verifying_key().to_bytes();
        self.sender = crate::crypto::derive_address_from_public_key(&self.public_key);
        self.signature = key.sign(&self.hash()).to_bytes().to_vec();
    }

    /// Create a simple transfer transaction with default gas values
    pub fn simple_transfer(
        sender: Address,
        to: Address,
        value: u64,
        nonce: u64,
        chain_id: u64,
    ) -> Self {
        Self {
            sender,
            nonce,
            public_key: [0; 32],
            payload: vec![],
            signature: vec![],
            gas_limit: 21_000,
            max_fee_per_gas: 1_000_000_000,        // 1 Gwei
            max_priority_fee_per_gas: 100_000_000, // 0.1 Gwei
            chain_id: Some(chain_id),
            to: Some(to),
            value,
        }
    }

    /// Create a test transaction with minimal gas values
    pub fn test_transaction(sender: Address, nonce: u64) -> Self {
        Self {
            sender,
            nonce,
            public_key: [0; 32],
            payload: vec![],
            signature: vec![0; 64],
            gas_limit: 21_000,
            max_fee_per_gas: 10_000_000_000,         // 10 Gwei
            max_priority_fee_per_gas: 2_000_000_000, // 2 Gwei
            chain_id: Some(1),
            to: Some([0; 20]),
            value: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    pub header: Header,
    pub extrinsics: Vec<Transaction>,
}

impl Block {
    pub fn new(header: Header, extrinsics: Vec<Transaction>) -> Self {
        Self { header, extrinsics }
    }

    pub fn hash(&self) -> Hash {
        self.header.hash()
    }

    pub fn genesis() -> Self {
        Self {
            header: Header::new([0; 32], 0),
            extrinsics: vec![],
        }
    }
}

// State Management Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Account {
    pub nonce: u64,
    pub balance: u128,
}

impl Account {
    pub fn new(balance: u128) -> Self {
        Self { nonce: 0, balance }
    }
}

// Genesis Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccount {
    pub address: Address,
    pub balance: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisValidator {
    pub address: Address, // [u8; 20]
    pub stake: u128,
    pub commission_rate: f64,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub chain_id: String,
    pub timestamp: u64,
    pub accounts: Vec<GenesisAccount>,
    pub validators: Vec<GenesisValidator>,
}

impl GenesisConfig {
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: GenesisConfig = serde_json::from_str(&content)?;
        Ok(config)
    }
}

impl Default for GenesisConfig {
    fn default() -> Self {
        Self {
            chain_id: "modular-blockchain".to_string(),
            timestamp: 0,
            accounts: vec![
                GenesisAccount {
                    address: [1u8; 20],
                    balance: 100_000_000_000_000,
                },
                GenesisAccount {
                    address: [2u8; 20],
                    balance: 50_000_000_000_000,
                },
            ],
            validators: vec![],
        }
    }
}

// Transaction Receipt Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    Success,
    Failed(String), // Error message
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionReceipt {
    pub tx_hash: Hash,
    pub block_hash: Hash,
    pub block_height: u64,
    pub transaction_index: u32,
    pub gas_used: u64,
    pub cumulative_gas_used: u64, // Total gas used up to this tx in the block
    pub status: ExecutionStatus,
    pub from: Address,
    pub to: Option<Address>,
    pub contract_address: Option<Address>, // If contract creation
}

impl TransactionReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tx_hash: Hash,
        block_hash: Hash,
        block_height: u64,
        transaction_index: u32,
        gas_used: u64,
        cumulative_gas_used: u64,
        status: ExecutionStatus,
        from: Address,
        to: Option<Address>,
    ) -> Self {
        Self {
            tx_hash,
            block_hash,
            block_height,
            transaction_index,
            gas_used,
            cumulative_gas_used,
            status,
            from,
            to,
            contract_address: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_hash_commits_to_fee_fields() {
        let header = Header::new([7; 32], 3);
        let mut other = header.clone();
        other.base_fee += 1;
        assert_ne!(header.hash(), other.hash());
        let mut other = header.clone();
        other.gas_used += 1;
        assert_ne!(header.hash(), other.hash());
        // The proposer signature is over the hash, so it must not be part of it.
        let mut signed = header.clone();
        signed.signature = vec![1; 64];
        assert_eq!(header.hash(), signed.hash());
    }

    #[test]
    fn tx_hash_distinguishes_optional_fields() {
        let mut a = Transaction::test_transaction([1; 20], 0);
        a.to = None;
        a.payload = [9u8; 20].to_vec();
        let mut b = a.clone();
        b.to = Some([9; 20]);
        b.payload.clear();
        assert_ne!(a.hash(), b.hash());
    }

    /// Fixed vectors shared with the JS clients (frontend `txHash`, SDK
    /// `Wallet.signTransaction`). If this changes, every client must change too.
    #[test]
    fn tx_hash_matches_client_vectors() {
        let tx = Transaction {
            sender: [0x11; 20],
            nonce: 7,
            public_key: [0x22; 32],
            payload: vec![1, 2, 3],
            signature: vec![],
            gas_limit: 21_000,
            max_fee_per_gas: 1_000_000_000,
            max_priority_fee_per_gas: 100_000_000,
            chain_id: Some(1),
            to: Some([0x33; 20]),
            value: 5,
        };
        assert_eq!(
            hex::encode(tx.hash()),
            "3413644368542c163b3a62867bd5c0743479b00d5cb16a3a0629b2891d5c70b9"
        );
        let bare = Transaction {
            chain_id: None,
            to: None,
            payload: vec![],
            ..tx
        };
        assert_eq!(
            hex::encode(bare.hash()),
            "56ce5bd73e11baa36f745bdbc607a09e9d89c302d8a6676ea3c0731deea2f9cd"
        );
    }
}
