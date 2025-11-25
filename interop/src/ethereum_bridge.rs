use common::types::{Address, Transaction};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Bridge message for cross-chain communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMessage {
    /// Unique message ID
    pub id: u64,
    /// Source chain ID
    pub source_chain: u32,
    /// Destination chain ID
    pub dest_chain: u32,
    /// Sender address on source chain
    pub sender: Address,
    /// Recipient address on destination chain
    pub recipient: Address,
    /// Token address (or native token if zero)
    pub token: Address,
    /// Amount to transfer
    pub amount: u128,
    /// Nonce for replay protection
    pub nonce: u64,
    /// Relayer signatures
    pub signatures: Vec<Vec<u8>>,
}

/// Ethereum bridge contract state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumBridge {
    /// Locked tokens on this chain
    locked_tokens: HashMap<Address, HashMap<Address, u128>>, // token -> user -> amount
    /// Processed message IDs to prevent replay
    processed_messages: HashMap<u64, bool>,
    /// Authorized relayers
    relayers: Vec<Address>,
    /// Required signatures for message validation
    required_signatures: usize,
    /// Next message ID
    next_message_id: u64,
    /// Ethereum chain ID
    eth_chain_id: u32,
    /// This chain ID
    chain_id: u32,
}

impl EthereumBridge {
    pub fn new(chain_id: u32, eth_chain_id: u32, relayers: Vec<Address>, required_signatures: usize) -> Self {
        Self {
            locked_tokens: HashMap::new(),
            processed_messages: HashMap::new(),
            relayers,
            required_signatures,
            next_message_id: 1,
            eth_chain_id,
            chain_id,
        }
    }

    /// Lock tokens to send to Ethereum
    pub fn lock_tokens(
        &mut self,
        user: Address,
        token: Address,
        amount: u128,
        eth_recipient: Address,
    ) -> Result<BridgeMessage, String> {
        if amount == 0 {
            return Err("Amount must be greater than 0".into());
        }

        // Record locked tokens
        let user_balance = self.locked_tokens
            .entry(token)
            .or_default()
            .entry(user)
            .or_insert(0);
        *user_balance += amount;

        // Create bridge message
        let message = BridgeMessage {
            id: self.next_message_id,
            source_chain: self.chain_id,
            dest_chain: self.eth_chain_id,
            sender: user,
            recipient: eth_recipient,
            token,
            amount,
            nonce: self.next_message_id,
            signatures: Vec::new(),
        };

        self.next_message_id += 1;

        Ok(message)
    }

    /// Unlock tokens received from Ethereum
    pub fn unlock_tokens(&mut self, message: BridgeMessage) -> Result<(), String> {
        // Verify message hasn't been processed
        if self.processed_messages.contains_key(&message.id) {
            return Err("Message already processed".into());
        }

        // Verify source chain
        if message.source_chain != self.eth_chain_id {
            return Err("Invalid source chain".into());
        }

        // Verify destination chain
        if message.dest_chain != self.chain_id {
            return Err("Invalid destination chain".into());
        }

        // Verify signatures
        if !self.verify_signatures(&message) {
            return Err("Invalid signatures".into());
        }

        // Mark as processed
        self.processed_messages.insert(message.id, true);

        // Unlock tokens (in real implementation, this would transfer tokens)
        let user_balance = self.locked_tokens
            .entry(message.token)
            .or_default()
            .entry(message.recipient)
            .or_insert(0);
        
        if *user_balance < message.amount {
            // If not enough locked, this is a new deposit from Ethereum
            *user_balance = message.amount;
        } else {
            *user_balance -= message.amount;
        }

        Ok(())
    }

    /// Verify relayer signatures on a message
    fn verify_signatures(&self, message: &BridgeMessage) -> bool {
        if message.signatures.len() < self.required_signatures {
            return false;
        }

        // In real implementation, verify cryptographic signatures
        // For now, just check we have enough signatures
        message.signatures.len() >= self.required_signatures
    }

    /// Add a relayer
    pub fn add_relayer(&mut self, relayer: Address) -> Result<(), String> {
        if self.relayers.contains(&relayer) {
            return Err("Relayer already exists".into());
        }
        self.relayers.push(relayer);
        Ok(())
    }

    /// Remove a relayer
    pub fn remove_relayer(&mut self, relayer: Address) -> Result<(), String> {
        if let Some(pos) = self.relayers.iter().position(|r| *r == relayer) {
            self.relayers.remove(pos);
            Ok(())
        } else {
            Err("Relayer not found".into())
        }
    }

    /// Get locked balance for a user
    pub fn get_locked_balance(&self, token: &Address, user: &Address) -> u128 {
        self.locked_tokens
            .get(token)
            .and_then(|users| users.get(user))
            .copied()
            .unwrap_or(0)
    }

    /// Check if message has been processed
    pub fn is_processed(&self, message_id: u64) -> bool {
        self.processed_messages.contains_key(&message_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_tokens() {
        let relayers = vec![[1u8; 20], [2u8; 20], [3u8; 20]];
        let mut bridge = EthereumBridge::new(1, 2, relayers, 2);

        let user = [10u8; 20];
        let token = [20u8; 20];
        let eth_recipient = [30u8; 20];

        let message = bridge.lock_tokens(user, token, 1000, eth_recipient).unwrap();

        assert_eq!(message.amount, 1000);
        assert_eq!(message.sender, user);
        assert_eq!(message.recipient, eth_recipient);
        assert_eq!(bridge.get_locked_balance(&token, &user), 1000);
    }

    #[test]
    fn test_unlock_tokens() {
        let relayers = vec![[1u8; 20], [2u8; 20], [3u8; 20]];
        let mut bridge = EthereumBridge::new(1, 2, relayers, 2);

        let mut message = BridgeMessage {
            id: 1,
            source_chain: 2,
            dest_chain: 1,
            sender: [10u8; 20],
            recipient: [20u8; 20],
            token: [30u8; 20],
            amount: 500,
            nonce: 1,
            signatures: vec![vec![1u8; 64], vec![2u8; 64]], // Mock signatures
        };

        let result = bridge.unlock_tokens(message.clone());
        assert!(result.is_ok());
        assert!(bridge.is_processed(1));

        // Try to process again - should fail
        let result = bridge.unlock_tokens(message);
        assert!(result.is_err());
    }

    #[test]
    fn test_relayer_management() {
        let relayers = vec![[1u8; 20]];
        let mut bridge = EthereumBridge::new(1, 2, relayers, 1);

        let new_relayer = [2u8; 20];
        bridge.add_relayer(new_relayer).unwrap();
        assert_eq!(bridge.relayers.len(), 2);

        bridge.remove_relayer(new_relayer).unwrap();
        assert_eq!(bridge.relayers.len(), 1);
    }
}
