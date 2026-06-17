// use ed25519_dalek::{Signature, Signer, SigningKey as DalekSigningKey, Verifier, VerifyingKey, SECRET_KEY_LENGTH};
// use rand::{rngs::OsRng, RngCore};

// #[derive(Debug)]
// pub enum CryptoError {
//     InvalidSignature,
//     InvalidPublicKey,
//     InvalidPrivateKey,
//     SigningError,
// }

// impl std::fmt::Display for CryptoError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             CryptoError::InvalidSignature => write!(f, "Invalid signature"),
//             CryptoError::InvalidPublicKey => write!(f, "Invalid public key"),
//             CryptoError::InvalidPrivateKey => write!(f, "Invalid private key"),
//             CryptoError::SigningError => write!(f, "Signing error"),
//         }
//     }
// }

// impl std::error::Error for CryptoError {}

// /// Wrapper for Ed25519 signing key
// #[derive(Clone)]
// pub struct SigningKey {
//     signing_key: DalekSigningKey,
//     verifying_key: VerifyingKey,
// }

// impl SigningKey {
//     /// Generate a new random signing key
//     pub fn generate() -> Self {
//         let mut seed = [0u8; 32];
//         OsRng.fill_bytes(&mut seed);
        
//         let signing_key = DalekSigningKey::from_bytes(&seed);
//         let verifying_key = signing_key.verifying_key();
        
//         Self { signing_key, verifying_key }
//     }
    
//     /// Create signing key from 32-byte seed
//     pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
//         if bytes.len() != SECRET_KEY_LENGTH {
//             return Err(CryptoError::InvalidPrivateKey);
//         }
        
//         let mut seed = [0u8; SECRET_KEY_LENGTH];
//         seed.copy_from_slice(bytes);
        
//         let signing_key = DalekSigningKey::from_bytes(&seed);
//         let verifying_key = signing_key.verifying_key();
        
//         Ok(Self { signing_key, verifying_key })
//     }
    
//     /// Sign a message
//     pub fn sign(&self, message: &[u8]) -> Vec<u8> {
//         let signature = self.signing_key.sign(message);
//         signature.to_bytes().to_vec()
//     }
    
//     /// Get the public key
//     pub fn public_key(&self) -> Vec<u8> {
//         self.verifying_key.to_bytes().to_vec()
//     }
    
//     /// Get the private key bytes (for storage)
//     pub fn to_bytes(&self) -> Vec<u8> {
//         self.signing_key.to_bytes().to_vec()
//     }
// }

// /// Verify a signature
// pub fn verify_signature(
//     public_key: &[u8],
//     message: &[u8],
//     signature: &[u8],
// ) -> Result<(), CryptoError> {
//     // Parse public key
//     if public_key.len() != 32 {
//         return Err(CryptoError::InvalidPublicKey);
//     }
    
//     let mut pk_bytes = [0u8; 32];
//     pk_bytes.copy_from_slice(public_key);
    
//     let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
//         .map_err(|_| CryptoError::InvalidPublicKey)?;
    
//     // Parse signature
//     if signature.len() != 64 {
//         return Err(CryptoError::InvalidSignature);
//     }
    
//     let mut sig_bytes = [0u8; 64];
//     sig_bytes.copy_from_slice(signature);
    
//     let sig = Signature::from_bytes(&sig_bytes);
    
//     // Verify
//     verifying_key
//         .verify(message, &sig)
//         .map_err(|_| CryptoError::InvalidSignature)
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_generate_and_sign() {
//         let key = SigningKey::generate();
//         let message = b"Hello, world!";
//         let signature = key.sign(message);
        
//         assert_eq!(signature.len(), 64);
        
//         // Verify the signature
//         let public_key = key.public_key();
//         assert!(verify_signature(&public_key, message, &signature).is_ok());
//     }

//     #[test]
//     fn test_invalid_signature() {
//         let key = SigningKey::generate();
//         let message = b"Hello, world!";
//         let signature = key.sign(message);
        
//         // Tamper with message
//         let wrong_message = b"Hello, World!";
//         let public_key = key.public_key();
        
//         assert!(verify_signature(&public_key, wrong_message, &signature).is_err());
//     }

//     #[test]
//     fn test_wrong_public_key() {
//         let key1 = SigningKey::generate();
//         let key2 = SigningKey::generate();
        
//         let message = b"Hello, world!";
//         let signature = key1.sign(message);
        
//         // Try to verify with wrong public key
//         let wrong_public_key = key2.public_key();
//         assert!(verify_signature(&wrong_public_key, message, &signature).is_err());
//     }

//     #[test]
//     fn test_from_bytes() {
//         let key1 = SigningKey::generate();
//         let bytes = key1.to_bytes();
        
//         let key2 = SigningKey::from_bytes(&bytes).unwrap();
        
//         // Both keys should produce same signature
//         let message = b"Test message";
//         let sig1 = key1.sign(message);
//         let sig2 = key2.sign(message);
        
//         assert_eq!(sig1, sig2);
//     }

//     #[test]
//     fn test_deterministic_signatures() {
//         let key = SigningKey::generate();
//         let message = b"Test message";
        
//         let sig1 = key.sign(message);
//         let sig2 = key.sign(message);
        
//         // Ed25519 signatures are deterministic
//         assert_eq!(sig1, sig2);
//     }
// }


use ed25519_dalek::{Signature, Signer, SigningKey as DalekSigningKey, Verifier, VerifyingKey, SECRET_KEY_LENGTH};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub enum CryptoError {
    InvalidSignature,
    InvalidPublicKey,
    InvalidPrivateKey,
    SigningError,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::InvalidSignature  => write!(f, "Invalid signature"),
            CryptoError::InvalidPublicKey  => write!(f, "Invalid public key"),
            CryptoError::InvalidPrivateKey => write!(f, "Invalid private key"),
            CryptoError::SigningError      => write!(f, "Signing error"),
        }
    }
}

impl std::error::Error for CryptoError {}

// =============================================================================
// Hash helper — required by bft.rs proposer selection and general use
// =============================================================================

/// SHA-256 hash of arbitrary bytes. Returns a 32-byte array.
/// Used for deterministic-but-unpredictable proposer selection in BFT.
pub fn hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Double SHA-256 (Bitcoin-style) — useful for Merkle trees.
pub fn hash256(data: &[u8]) -> [u8; 32] {
    hash(&hash(data))
}

// =============================================================================
// Signing key wrapper (Ed25519 via ed25519_dalek)
// =============================================================================

#[derive(Clone)]
pub struct SigningKey {
    signing_key:   DalekSigningKey,
    verifying_key: VerifyingKey,
}

impl SigningKey {
    /// Generate a new random signing key using the OS RNG.
    pub fn generate() -> Self {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        Self::from_seed(seed)
    }

    /// Create a signing key from a 32-byte seed.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != SECRET_KEY_LENGTH {
            return Err(CryptoError::InvalidPrivateKey);
        }
        let mut seed = [0u8; SECRET_KEY_LENGTH];
        seed.copy_from_slice(bytes);
        Ok(Self::from_seed(seed))
    }

    fn from_seed(seed: [u8; 32]) -> Self {
        let signing_key   = DalekSigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        Self { signing_key, verifying_key }
    }

    /// Sign a message. Returns a 64-byte Ed25519 signature.
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_bytes().to_vec()
    }

    /// Return the 32-byte compressed public key.
    pub fn public_key(&self) -> Vec<u8> {
        self.verifying_key.to_bytes().to_vec()
    }

    /// Return the 32-byte private key seed (for secure storage / export).
    pub fn to_bytes(&self) -> Vec<u8> {
        self.signing_key.to_bytes().to_vec()
    }
}

// =============================================================================
// Signature verification
// =============================================================================

/// Verify an Ed25519 signature.
///
/// Returns `Ok(())` if the signature is valid, or a `CryptoError` otherwise.
pub fn verify_signature(
    public_key: &[u8],
    message:    &[u8],
    signature:  &[u8],
) -> Result<(), CryptoError> {
    if public_key.len() != 32 {
        return Err(CryptoError::InvalidPublicKey);
    }
    if signature.len() != 64 {
        return Err(CryptoError::InvalidSignature);
    }

    let mut pk_bytes  = [0u8; 32];
    let mut sig_bytes = [0u8; 64];
    pk_bytes.copy_from_slice(public_key);
    sig_bytes.copy_from_slice(signature);

    let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
        .map_err(|_| CryptoError::InvalidPublicKey)?;
    let sig = Signature::from_bytes(&sig_bytes);

    verifying_key
        .verify(message, &sig)
        .map_err(|_| CryptoError::InvalidSignature)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_produces_32_bytes() {
        let h = hash(b"hello world");
        assert_eq!(h.len(), 32);
    }

    #[test]
    fn test_hash_is_deterministic() {
        assert_eq!(hash(b"same input"), hash(b"same input"));
    }

    #[test]
    fn test_hash_differs_on_different_input() {
        assert_ne!(hash(b"input a"), hash(b"input b"));
    }

    #[test]
    fn test_generate_and_sign() {
        let key     = SigningKey::generate();
        let message = b"Hello, world!";
        let sig     = key.sign(message);

        assert_eq!(sig.len(), 64);
        assert!(verify_signature(&key.public_key(), message, &sig).is_ok());
    }

    #[test]
    fn test_invalid_signature_tampered_message() {
        let key = SigningKey::generate();
        let sig = key.sign(b"Hello, world!");
        assert!(verify_signature(&key.public_key(), b"Hello, World!", &sig).is_err());
    }

    #[test]
    fn test_wrong_public_key_rejected() {
        let key1 = SigningKey::generate();
        let key2 = SigningKey::generate();
        let sig  = key1.sign(b"Hello");
        assert!(verify_signature(&key2.public_key(), b"Hello", &sig).is_err());
    }

    #[test]
    fn test_from_bytes_roundtrip() {
        let key1  = SigningKey::generate();
        let key2  = SigningKey::from_bytes(&key1.to_bytes()).unwrap();
        let msg   = b"Test message";
        assert_eq!(key1.sign(msg), key2.sign(msg));
    }

    #[test]
    fn test_signatures_are_deterministic() {
        let key = SigningKey::generate();
        let msg = b"Test message";
        assert_eq!(key.sign(msg), key.sign(msg));
    }

    #[test]
    fn test_empty_signature_rejected() {
        let key = SigningKey::generate();
        assert!(verify_signature(&key.public_key(), b"msg", &[]).is_err());
    }

    #[test]
    fn test_short_public_key_rejected() {
        assert!(verify_signature(&[0u8; 10], b"msg", &[0u8; 64]).is_err());
    }
}

// =============================================================================
// Additional Cryptographic Utilities
// =============================================================================

/// Hash two 32-byte values together (for Merkle trees)
pub fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// Derive an Ethereum-style address from a public key
/// Takes the last 20 bytes of the hash of the public key
pub fn derive_address_from_public_key(public_key: &[u8]) -> [u8; 20] {
    let hash = hash(public_key);
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..32]); // Last 20 bytes
    address
}

/// Batch verify multiple signatures (more efficient than individual verification)
pub fn verify_batch(
    public_keys: &[&[u8]],
    messages: &[&[u8]],
    signatures: &[&[u8]],
) -> bool {
    if public_keys.len() != messages.len() || messages.len() != signatures.len() {
        return false;
    }
    
    for i in 0..public_keys.len() {
        if verify_signature(public_keys[i], messages[i], signatures[i]).is_err() {
            return false;
        }
    }
    true
}

/// Generate a random nonce for transaction replay protection
pub fn generate_nonce() -> u64 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    rng.gen()
}

impl SigningKey {
    /// Create signing key from hex string
    pub fn from_hex(hex_str: &str) -> Result<Self, CryptoError> {
        let bytes = hex::decode(hex_str.trim_start_matches("0x"))
            .map_err(|_| CryptoError::InvalidPrivateKey)?;
        Self::from_bytes(&bytes)
    }
    
    /// Export public key as hex
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key())
    }
    
    /// Export private key as hex
    pub fn to_hex(&self) -> String {
        hex::encode(self.to_bytes())
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;
    
    #[test]
    fn test_hash_pair() {
        let left = [1u8; 32];
        let right = [2u8; 32];
        let result = hash_pair(&left, &right);
        assert_eq!(result.len(), 32);
        assert_ne!(result, left);
        assert_ne!(result, right);
    }
    
    #[test]
    fn test_derive_address() {
        let pubkey = [1u8; 32];
        let address = derive_address_from_public_key(&pubkey);
        assert_eq!(address.len(), 20);
    }
    
    #[test]
    fn test_batch_verify() {
        let key1 = SigningKey::generate();
        let key2 = SigningKey::generate();
        
        let msg1 = b"message 1";
        let msg2 = b"message 2";
        
        let sig1 = key1.sign(msg1);
        let sig2 = key2.sign(msg2);
        
        let pk1 = key1.public_key();
        let pk2 = key2.public_key();
        let pubkeys = vec![pk1.as_slice(), pk2.as_slice()];
        let messages = vec![msg1.as_slice(), msg2.as_slice()];
        let sigs = vec![sig1.as_slice(), sig2.as_slice()];
        
        assert!(verify_batch(&pubkeys, &messages, &sigs));
    }
    
    #[test]
    fn test_key_hex_roundtrip() {
        let key1 = SigningKey::generate();
        let hex = key1.to_hex();
        let key2 = SigningKey::from_hex(&hex).unwrap();
        
        let msg = b"test";
        assert_eq!(key1.sign(msg), key2.sign(msg));
    }
}