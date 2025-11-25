use common::types::{Address, Transaction};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod ethereum_bridge;
pub mod relayer;
pub mod token_registry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainMessage {
    pub source_chain: String,
    pub dest_chain: String,
    pub nonce: u64,
    pub sender: Address,
    pub recipient: Address,
    pub amount: u64,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeContract {
    pub chain_id: String,
    pub locked_assets: HashMap<Address, u64>, // User -> Amount
    pub processed_nonces: HashMap<String, u64>, // SourceChain -> LastNonce
    pub relayers: Vec<Vec<u8>>, // Public keys of authorized relayers
}

impl BridgeContract {
    pub fn new(chain_id: String, relayers: Vec<Vec<u8>>) -> Self {
        Self {
            chain_id,
            locked_assets: HashMap::new(),
            processed_nonces: HashMap::new(),
            relayers,
        }
    }

    /// Lock assets to be sent to another chain
    pub fn lock_assets(
        &mut self,
        sender: Address,
        dest_chain: String,
        recipient: Address,
        amount: u64,
    ) -> Result<CrossChainMessage, String> {
        // In a real system, we would transfer funds from sender to bridge account here.
        // For this simulation, we just track the locked amount.
        
        let current_locked = self.locked_assets.entry(sender).or_insert(0);
        *current_locked += amount;

        let _nonce = self.processed_nonces.get(&dest_chain).unwrap_or(&0) + 1; // This logic is slightly wrong for outbound, but ok for MVP
        // Actually, nonce should be per sender or global outbound nonce.
        // Let's use a simple global nonce for now.
        let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        let message = CrossChainMessage {
            source_chain: self.chain_id.clone(),
            dest_chain,
            nonce,
            sender,
            recipient,
            amount,
            payload: vec![],
        };

        Ok(message)
    }

    /// Unlock assets (Mint/Release) based on proof from another chain
    pub fn unlock_assets(
        &mut self,
        message: CrossChainMessage,
        _relayer_sig: Vec<u8>,
        relayer_pubkey: Vec<u8>,
    ) -> Result<(), String> {
        if message.dest_chain != self.chain_id {
            return Err("Wrong destination chain".into());
        }

        // Verify relayer is authorized
        if !self.relayers.contains(&relayer_pubkey) {
            return Err("Unauthorized relayer".into());
        }

        // Verify signature (Mocked for now)
        // verify_signature(&message, &relayer_sig, &relayer_pubkey)?;

        // Check replay
        let last_nonce = self.processed_nonces.entry(message.source_chain.clone()).or_insert(0);
        if message.nonce <= *last_nonce {
            return Err("Message already processed".into());
        }
        *last_nonce = message.nonce;

        // Release assets
        // In real system: Mint wrapped tokens or release native tokens
        // Here we just log it
        println!("Releasing {} to {:?}", message.amount, message.recipient);

        Ok(())
    }
}
