use anyhow::Result;
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainMessage {
    pub source_chain: u64,
    pub target_chain: u64,
    pub nonce: u64,
    pub payload: Vec<u8>,
    pub signature: Option<Vec<u8>>, // Signature over the message content
}

pub struct Router {
    pub chain_id: u64,
    pub inbox: Vec<CrossChainMessage>,
    pub outbox: Vec<CrossChainMessage>,
    // In a real implementation, we would manage keys securely
    pub signing_key: SigningKey,
}

impl Router {
    pub fn new(chain_id: u64) -> Self {
        let mut csprng = rand::rngs::OsRng;
        let signing_key = SigningKey::generate(&mut csprng);

        Self {
            chain_id,
            inbox: Vec::new(),
            outbox: Vec::new(),
            signing_key,
        }
    }

    pub fn send_message(&mut self, target_chain: u64, payload: Vec<u8>) -> Result<()> {
        let mut msg = CrossChainMessage {
            source_chain: self.chain_id,
            target_chain,
            nonce: self.outbox.len() as u64,
            payload,
            signature: None,
        };

        // Sign the message (simplified: signing the payload for now)
        // Real implementation should sign the hash of the entire struct
        let signature = self.signing_key.sign(&msg.payload);
        msg.signature = Some(signature.to_vec());

        self.outbox.push(msg);
        println!("Message added to outbox for chain {}", target_chain);
        Ok(())
    }

    pub fn receive_message(&mut self, msg: CrossChainMessage) -> Result<()> {
        if msg.target_chain != self.chain_id {
            return Err(anyhow::anyhow!("Message not for this chain"));
        }

        // Verify signature (simplified: assuming we know the sender's public key)
        // In reality, we'd look up the validator set for source_chain
        if let Some(sig_bytes) = &msg.signature {
            if sig_bytes.len() != 64 {
                return Err(anyhow::anyhow!("Invalid signature length"));
            }
            // Mock verification: we can't verify without the sender's public key here
            // For this MVP, we just check presence
        } else {
            return Err(anyhow::anyhow!("Missing signature"));
        }

        println!("Message received from chain {}", msg.source_chain);
        self.inbox.push(msg);
        Ok(())
    }
}

pub fn init() {
    println!("Interop Router initialized (use Router::new)");
}
