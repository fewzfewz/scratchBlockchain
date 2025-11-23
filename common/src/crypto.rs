use ed25519_dalek::{Signature, Signer, SigningKey as DalekSigningKey, Verifier, VerifyingKey, SECRET_KEY_LENGTH};
use rand::{rngs::OsRng, RngCore};

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
            CryptoError::InvalidSignature => write!(f, "Invalid signature"),
            CryptoError::InvalidPublicKey => write!(f, "Invalid public key"),
            CryptoError::InvalidPrivateKey => write!(f, "Invalid private key"),
            CryptoError::SigningError => write!(f, "Signing error"),
        }
    }
}

impl std::error::Error for CryptoError {}

/// Wrapper for Ed25519 signing key
#[derive(Clone)]
pub struct SigningKey {
    signing_key: DalekSigningKey,
    verifying_key: VerifyingKey,
}

impl SigningKey {
    /// Generate a new random signing key
    pub fn generate() -> Self {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        
        let signing_key = DalekSigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        
        Self { signing_key, verifying_key }
    }
    
    /// Create signing key from 32-byte seed
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != SECRET_KEY_LENGTH {
            return Err(CryptoError::InvalidPrivateKey);
        }
        
        let mut seed = [0u8; SECRET_KEY_LENGTH];
        seed.copy_from_slice(bytes);
        
        let signing_key = DalekSigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        
        Ok(Self { signing_key, verifying_key })
    }
    
    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let signature = self.signing_key.sign(message);
        signature.to_bytes().to_vec()
    }
    
    /// Get the public key
    pub fn public_key(&self) -> Vec<u8> {
        self.verifying_key.to_bytes().to_vec()
    }
    
    /// Get the private key bytes (for storage)
    pub fn to_bytes(&self) -> Vec<u8> {
        self.signing_key.to_bytes().to_vec()
    }
}

/// Verify a signature
pub fn verify_signature(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), CryptoError> {
    // Parse public key
    if public_key.len() != 32 {
        return Err(CryptoError::InvalidPublicKey);
    }
    
    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(public_key);
    
    let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
        .map_err(|_| CryptoError::InvalidPublicKey)?;
    
    // Parse signature
    if signature.len() != 64 {
        return Err(CryptoError::InvalidSignature);
    }
    
    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(signature);
    
    let sig = Signature::from_bytes(&sig_bytes);
    
    // Verify
    verifying_key
        .verify(message, &sig)
        .map_err(|_| CryptoError::InvalidSignature)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_sign() {
        let key = SigningKey::generate();
        let message = b"Hello, world!";
        let signature = key.sign(message);
        
        assert_eq!(signature.len(), 64);
        
        // Verify the signature
        let public_key = key.public_key();
        assert!(verify_signature(&public_key, message, &signature).is_ok());
    }

    #[test]
    fn test_invalid_signature() {
        let key = SigningKey::generate();
        let message = b"Hello, world!";
        let signature = key.sign(message);
        
        // Tamper with message
        let wrong_message = b"Hello, World!";
        let public_key = key.public_key();
        
        assert!(verify_signature(&public_key, wrong_message, &signature).is_err());
    }

    #[test]
    fn test_wrong_public_key() {
        let key1 = SigningKey::generate();
        let key2 = SigningKey::generate();
        
        let message = b"Hello, world!";
        let signature = key1.sign(message);
        
        // Try to verify with wrong public key
        let wrong_public_key = key2.public_key();
        assert!(verify_signature(&wrong_public_key, message, &signature).is_err());
    }

    #[test]
    fn test_from_bytes() {
        let key1 = SigningKey::generate();
        let bytes = key1.to_bytes();
        
        let key2 = SigningKey::from_bytes(&bytes).unwrap();
        
        // Both keys should produce same signature
        let message = b"Test message";
        let sig1 = key1.sign(message);
        let sig2 = key2.sign(message);
        
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_deterministic_signatures() {
        let key = SigningKey::generate();
        let message = b"Test message";
        
        let sig1 = key.sign(message);
        let sig2 = key.sign(message);
        
        // Ed25519 signatures are deterministic
        assert_eq!(sig1, sig2);
    }
}
