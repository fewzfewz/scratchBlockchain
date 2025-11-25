
pub mod bft;

use common::traits::Consensus;
use common::types::{Block, Hash, Header};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;

// Slashing conditions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlashingCondition {
    DoubleSign { height: u64, validator: Vec<u8> },
    Equivocation { height: u64, validator: Vec<u8> },
    Censorship { validator: Vec<u8> },
}

// Validator information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub public_key: Vec<u8>,
    pub stake: u64,
    pub slashed: bool,
}

// GRANDPA-style finality vote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityVote {
    pub block_hash: [u8; 32],
    pub block_number: u64,
    pub voter: Vec<u8>,
    pub signature: Vec<u8>,
}

impl FinalityVote {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.block_hash);
        bytes.extend_from_slice(&self.block_number.to_le_bytes());
        bytes.extend_from_slice(&self.voter);
        // Do not include signature in the serialized data
        bytes
    }
}

// Finality gadget implementing GRANDPA-like finality
pub struct FinalityGadget {
    // Validators participating in finality
    validators: HashMap<Vec<u8>, ValidatorInfo>,
    // Votes for each round
    prevotes: HashMap<u64, Vec<FinalityVote>>,
    precommits: HashMap<u64, Vec<FinalityVote>>,
    // Finalized blocks
    finalized_blocks: HashMap<u64, [u8; 32]>,
    // Current round
    #[allow(dead_code)]
    current_round: u64,
}

impl FinalityGadget {
    pub fn new(validators: Vec<ValidatorInfo>) -> Self {
        let mut validator_map = HashMap::new();
        for v in validators {
            validator_map.insert(v.public_key.clone(), v);
        }

        Self {
            validators: validator_map,
            prevotes: HashMap::new(),
            precommits: HashMap::new(),
            finalized_blocks: HashMap::new(),
            current_round: 0,
        }
    }

    /// Submit a prevote for a block
    pub fn prevote(&mut self, vote: FinalityVote) -> Result<(), Box<dyn Error>> {
        // Verify the voter is a validator
        if !self.validators.contains_key(&vote.voter) {
            return Err("Voter is not a validator".into());
        }

        // Verify signature
        let vote_bytes = vote.to_bytes();
        if let Err(e) = common::crypto::verify_signature(&vote.voter, &vote_bytes, &vote.signature) {
            return Err(format!("Invalid signature: {}", e).into());
        }

        // Add to prevotes
        self.prevotes
            .entry(vote.block_number)
            .or_default()
            .push(vote);

        Ok(())
    }

    /// Submit a precommit for a block
    pub fn precommit(&mut self, vote: FinalityVote) -> Result<(), Box<dyn Error>> {
        // Verify the voter is a validator
        if !self.validators.contains_key(&vote.voter) {
            return Err("Voter is not a validator".into());
        }

        // Verify signature
        let vote_bytes = vote.to_bytes();
        if let Err(e) = common::crypto::verify_signature(&vote.voter, &vote_bytes, &vote.signature) {
            return Err(format!("Invalid signature: {}", e).into());
        }

        let block_number = vote.block_number; // Capture before move

        // Add to precommits
        self.precommits
            .entry(vote.block_number)
            .or_default()
            .push(vote);

        // Check if we can finalize
        self.try_finalize(block_number)?;

        Ok(())
    }

    /// Try to finalize a block if we have enough precommits
    fn try_finalize(&mut self, block_number: u64) -> Result<(), Box<dyn Error>> {
        let precommits = self.precommits.get(&block_number);
        if precommits.is_none() {
            return Ok(());
        }

        let precommits = precommits.unwrap();
        let total_stake: u64 = self.validators.values().map(|v| v.stake).sum();
        let threshold = (total_stake * 2) / 3; // 2/3 threshold

        // Calculate stake that has precommitted
        let mut stake_precommitted = 0u64;
        let mut block_hash: Option<Hash> = None;

        for vote in precommits {
            if let Some(validator) = self.validators.get(&vote.voter) {
                if !validator.slashed {
                    stake_precommitted += validator.stake;
                    if block_hash.is_none() {
                        block_hash = Some(vote.block_hash);
                    }
                }
            }
        }

        // If we have 2/3+ stake, finalize
        if stake_precommitted >= threshold {
            if let Some(hash) = block_hash {
                self.finalized_blocks.insert(block_number, hash);
                println!("✓ Block {} finalized with hash {:?}", block_number, hash);
            }
        }

        Ok(())
    }

    /// Check if a block is finalized
    pub fn is_finalized(&self, block_number: u64) -> bool {
        self.finalized_blocks.contains_key(&block_number)
    }

    /// Get finalized block hash
    pub fn get_finalized_hash(&self, block_number: u64) -> Option<[u8; 32]> {
        self.finalized_blocks.get(&block_number).copied()
    }

    /// Slash a validator
    pub fn slash(&mut self, validator_pubkey: &[u8]) -> Result<(), Box<dyn Error>> {
        if let Some(validator) = self.validators.get_mut(validator_pubkey) {
            validator.slashed = true;
            validator.stake = 0; // Confiscate stake
            println!("⚠ Validator slashed: {:?}", validator_pubkey);
            Ok(())
        } else {
            Err("Validator not found".into())
        }
    }
}

// Enhanced consensus with slashing and finality
pub struct EnhancedConsensus {
    validators: HashSet<Vec<u8>>,
    finality_gadget: FinalityGadget,
    // Track seen blocks to detect double-signing
    seen_blocks: HashMap<u64, Vec<[u8; 32]>>,
    // Slashing events
    slashing_events: Vec<SlashingCondition>,
}

impl EnhancedConsensus {
    pub fn new(validator_infos: Vec<ValidatorInfo>) -> Self {
        let validators: HashSet<Vec<u8>> = validator_infos
            .iter()
            .map(|v| v.public_key.clone())
            .collect();

        let finality_gadget = FinalityGadget::new(validator_infos);

        Self {
            validators,
            finality_gadget,
            seen_blocks: HashMap::new(),
            slashing_events: Vec::new(),
        }
    }

    /// Detect and handle slashing conditions
    pub fn check_slashing_conditions(
        &mut self,
        header: &Header,
    ) -> Result<(), Box<dyn Error>> {
        let height = header.slot; // Use slot as height

        // Check for double-signing
        let blocks_at_height = self.seen_blocks.entry(height).or_default();

        // If we've seen a different block at this height from the same validator, slash
        if !blocks_at_height.is_empty() {
            let current_hash = header.parent_hash; // Use parent_hash as identifier
            for existing_hash in blocks_at_height.iter() {
                if existing_hash != &current_hash {
                    // Double-sign detected!
                    let condition = SlashingCondition::DoubleSign {
                        height,
                        validator: header.signature.clone(), // Simplified - should be pubkey
                    };
                    self.slashing_events.push(condition);
                    self.finality_gadget.slash(&header.signature)?;
                    return Err("Double-sign detected".into());
                }
            }
        }

        blocks_at_height.push(header.parent_hash);
        Ok(())
    }

    /// Get slashing events
    pub fn get_slashing_events(&self) -> &[SlashingCondition] {
        &self.slashing_events
    }

    /// Submit a finality vote
    pub fn submit_prevote(&mut self, vote: FinalityVote) -> Result<(), Box<dyn Error>> {
        self.finality_gadget.prevote(vote)
    }

    pub fn submit_precommit(&mut self, vote: FinalityVote) -> Result<(), Box<dyn Error>> {
        self.finality_gadget.precommit(vote)
    }

    /// Check if block is finalized
    pub fn is_block_finalized(&self, block_number: u64) -> bool {
        self.finality_gadget.is_finalized(block_number)
    }
}

impl Consensus for EnhancedConsensus {
    fn verify_header(&self, header: &Header) -> Result<(), Box<dyn Error>> {
        // Verify signature is present
        if header.signature.is_empty() {
            return Err("Header signature is empty".into());
        }

        // Verify signature length (ed25519 signatures are 64 bytes)
        if header.signature.len() != 64 {
            return Err("Invalid signature length".into());
        }

        // Check if we have validators
        if self.validators.is_empty() {
            return Err("No validators configured".into());
        }

        // In a real implementation, we would verify against the specific validator's key
        // For this MVP, we'll check if the signature is valid for *any* known validator
        // This is a simplification; normally we'd look up the validator by ID/slot
        
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};
        
        let signature = Signature::from_slice(&header.signature)?;
        
        // Reconstruct the message that was signed
        // In a real system, this would be the serialized header minus the signature
        // We use the header hash which includes all fields except the signature
        let message = header.hash();
        
        // Try to find a validator that signed this
        let mut verified = false;
        for validator_pubkey in &self.validators {
            if let Ok(verifying_key) = VerifyingKey::from_bytes(validator_pubkey.as_slice().try_into()?) {
                if verifying_key.verify(&message, &signature).is_ok() {
                    verified = true;
                    break;
                }
            }
        }

        if !verified {
            return Err("Invalid signature or unknown validator".into());
        }

        Ok(())
    }

    fn verify_block(&self, block: &Block) -> Result<(), Box<dyn Error>> {
        self.verify_header(&block.header)?;
        Ok(())
    }

    fn is_finalized(&self, hash: &[u8; 32]) -> bool {
        // Check if any finalized block has this hash
        self.finality_gadget
            .finalized_blocks
            .values()
            .any(|h| h == hash)
    }
}

// Keep the simple consensus for backward compatibility
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
        if header.signature.is_empty() {
            return Err("Header signature is empty".into());
        }

        if self.validators.is_empty() {
            return Err("No validators configured".into());
        }

        Ok(())
    }

    fn verify_block(&self, block: &Block) -> Result<(), Box<dyn Error>> {
        self.verify_header(&block.header)?;
        Ok(())
    }

    fn is_finalized(&self, _hash: &[u8; 32]) -> bool {
        true
    }
}

pub fn init() {
    println!("Consensus initialized (use SimpleConsensus or EnhancedConsensus)");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finality_gadget_creation() {
        let validators = vec![
            ValidatorInfo {
                public_key: vec![1, 2, 3],
                stake: 100,
                slashed: false,
            },
            ValidatorInfo {
                public_key: vec![4, 5, 6],
                stake: 100,
                slashed: false,
            },
        ];

        let gadget = FinalityGadget::new(validators);
        assert_eq!(gadget.validators.len(), 2);
        assert_eq!(gadget.current_round, 0);
    }

    #[test]
    fn test_slashing() {
        let validators = vec![ValidatorInfo {
            public_key: vec![1, 2, 3],
            stake: 100,
            slashed: false,
        }];

        let mut gadget = FinalityGadget::new(validators);
        assert!(gadget.slash(&vec![1, 2, 3]).is_ok());

        let validator = gadget.validators.get(&vec![1, 2, 3]).unwrap();
        assert!(validator.slashed);
        assert_eq!(validator.stake, 0);
    }

    #[test]
    fn test_double_sign_detection() {
        let validators = vec![ValidatorInfo {
            public_key: vec![1, 2, 3],
            stake: 100,
            slashed: false,
        }];

        let mut consensus = EnhancedConsensus::new(validators);

        let header1 = Header {
            parent_hash: [0; 32],
            state_root: [1; 32],
            extrinsics_root: [0; 32],
            slot: 1,
            epoch: 0,
            validator_set_id: 0,
            signature: vec![1, 2, 3],
            gas_used: 0,
            base_fee: 1_000_000_000,
        };

        let header2 = Header {
            parent_hash: [1; 32], // Different parent hash
            state_root: [2; 32],
            extrinsics_root: [0; 32],
            slot: 1, // Same slot
            epoch: 0,
            validator_set_id: 0,
            signature: vec![1, 2, 3],
            gas_used: 0,
            base_fee: 1_000_000_000,
        };

        // First block should be fine
        assert!(consensus.check_slashing_conditions(&header1).is_ok());

        // Second block at same slot should trigger slashing
        assert!(consensus.check_slashing_conditions(&header2).is_err());
        assert_eq!(consensus.slashing_events.len(), 1);
    }

    #[test]
    fn test_finality_threshold() {
        use common::crypto::SigningKey;
        
        // Generate keys
        let key1 = SigningKey::generate();
        let key2 = SigningKey::generate();
        let key3 = SigningKey::generate();
        
        let validators = vec![
            ValidatorInfo {
                public_key: key1.public_key(),
                stake: 100,
                slashed: false,
            },
            ValidatorInfo {
                public_key: key2.public_key(),
                stake: 100,
                slashed: false,
            },
            ValidatorInfo {
                public_key: key3.public_key(),
                stake: 100,
                slashed: false,
            },
        ];

        let mut gadget = FinalityGadget::new(validators);

        let block_hash: [u8; 32] = [1; 32];

        // Submit precommits from 2 out of 3 validators (2/3 threshold)
        let mut vote1 = FinalityVote {
            block_hash: block_hash.clone(),
            block_number: 1,
            voter: key1.public_key(),
            signature: vec![],
        };
        let vote_bytes1 = vote1.to_bytes();
        vote1.signature = key1.sign(&vote_bytes1);

        let mut vote2 = FinalityVote {
            block_hash: block_hash.clone(),
            block_number: 1,
            voter: key2.public_key(),
            signature: vec![],
        };
        let vote_bytes2 = vote2.to_bytes();
        vote2.signature = key2.sign(&vote_bytes2);

        assert!(gadget.precommit(vote1).is_ok());
        assert!(gadget.precommit(vote2).is_ok());

        // Block should now be finalized
        assert!(gadget.is_finalized(1));
        assert_eq!(gadget.get_finalized_hash(1), Some(block_hash));
    }

    #[test]
    fn test_real_signature_verification() {
        use ed25519_dalek::{Signer, SigningKey};
        use rand::rngs::OsRng;
        use rand::RngCore;

        // Generate a keypair
        let mut csprng = OsRng;
        let mut secret_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();
        let pubkey_bytes = verifying_key.to_bytes().to_vec();

        // Setup consensus with this validator
        let validators = vec![ValidatorInfo {
            public_key: pubkey_bytes.clone(),
            stake: 100,
            slashed: false,
        }];
        let consensus = EnhancedConsensus::new(validators);

        // Create a header
        let parent_hash = [0u8; 32];
        let slot = 1u64;
        
        let mut header = Header {
            parent_hash: [0; 32],
            state_root: [0; 32],
            extrinsics_root: [0; 32],
            slot: 1,
            epoch: 0,
            validator_set_id: 0,
            signature: vec![],
            gas_used: 0,
            base_fee: 1_000_000_000,
        };
        // Sign the message (header hash)
        let message = header.hash();
        let signature = signing_key.sign(&message);
        header.signature = signature.to_vec();

        // Verify should succeed
        assert!(consensus.verify_header(&header).is_ok());

        // Verify with wrong signature should fail
        let mut bad_header = header.clone();
        bad_header.signature = vec![0; 64]; // Invalid signature
        assert!(consensus.verify_header(&bad_header).is_err());

        // Verify with wrong message (e.g. different slot) should fail
        let mut bad_slot_header = header.clone();
        bad_slot_header.slot = 2;
        // Signature is for slot 1, so this should fail
        assert!(consensus.verify_header(&bad_slot_header).is_err());
    }
}

// View-Change Protocol for leader rotation and fault tolerance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewChangeMessage {
    pub view_number: u64,
    pub sender: Vec<u8>,
    pub signature: Vec<u8>,
}

pub struct ViewChange {
    current_view: u64,
    validators: Vec<ValidatorInfo>,
    view_change_votes: HashMap<u64, Vec<ViewChangeMessage>>,
    view_change_threshold: usize,
}

impl ViewChange {
    pub fn new(validators: Vec<ValidatorInfo>) -> Self {
        let threshold = (validators.len() * 2) / 3 + 1; // 2/3 + 1 for BFT
        Self {
            current_view: 0,
            validators,
            view_change_votes: HashMap::new(),
            view_change_threshold: threshold,
        }
    }

    /// Get current view number
    pub fn current_view(&self) -> u64 {
        self.current_view
    }

    /// Get leader for current view (round-robin)
    pub fn get_leader(&self) -> Option<&ValidatorInfo> {
        if self.validators.is_empty() {
            return None;
        }
        let leader_index = (self.current_view as usize) % self.validators.len();
        self.validators.get(leader_index)
    }

    /// Get leader for a specific view
    pub fn get_leader_for_view(&self, view: u64) -> Option<&ValidatorInfo> {
        if self.validators.is_empty() {
            return None;
        }
        let leader_index = (view as usize) % self.validators.len();
        self.validators.get(leader_index)
    }

    /// Submit a view-change vote
    pub fn submit_view_change(&mut self, msg: ViewChangeMessage) -> Result<bool, Box<dyn Error>> {
        // Verify sender is a validator
        if !self.validators.iter().any(|v| v.public_key == msg.sender) {
            return Err("Sender is not a validator".into());
        }

        // Verify signature length
        if msg.signature.len() != 64 {
            return Err("Invalid signature length".into());
        }

        let view = msg.view_number;
        
        // Add vote
        self.view_change_votes
            .entry(view)
            .or_default()
            .push(msg);

        // Check if we have enough votes to change view
        let votes = self.view_change_votes.get(&view).unwrap().len();
        if votes >= self.view_change_threshold {
            self.current_view = view;
            Ok(true) // View changed
        } else {
            Ok(false) // Not enough votes yet
        }
    }

    /// Trigger view change (e.g., due to timeout or leader failure)
    pub fn trigger_view_change(&mut self) -> u64 {
        self.current_view += 1;
        self.current_view
    }
}

#[cfg(test)]
mod view_change_tests {
    use super::*;

    fn create_test_validators(count: usize) -> Vec<ValidatorInfo> {
        (0..count)
            .map(|i| ValidatorInfo {
                public_key: vec![i as u8; 32],
                stake: 100,
                slashed: false,
            })
            .collect()
    }

    #[test]
    fn test_view_change_leader_rotation() {
        let validators = create_test_validators(4);
        let vc = ViewChange::new(validators.clone());

        // View 0 leader should be validator 0
        assert_eq!(vc.get_leader().unwrap().public_key, validators[0].public_key);

        // View 1 leader should be validator 1
        assert_eq!(
            vc.get_leader_for_view(1).unwrap().public_key,
            validators[1].public_key
        );

        // View 4 should wrap around to validator 0
        assert_eq!(
            vc.get_leader_for_view(4).unwrap().public_key,
            validators[0].public_key
        );
    }

    #[test]
    fn test_view_change_voting() {
        let validators = create_test_validators(4);
        let mut vc = ViewChange::new(validators.clone());

        assert_eq!(vc.current_view(), 0);

        // Submit view change votes for view 1
        // Need 3 votes (2/3 of 4 = 2.66, rounded up to 3)
        let msg1 = ViewChangeMessage {
            view_number: 1,
            sender: validators[0].public_key.clone(),
            signature: vec![0; 64],
        };
        assert!(!vc.submit_view_change(msg1).unwrap());

        let msg2 = ViewChangeMessage {
            view_number: 1,
            sender: validators[1].public_key.clone(),
            signature: vec![0; 64],
        };
        assert!(!vc.submit_view_change(msg2).unwrap());

        let msg3 = ViewChangeMessage {
            view_number: 1,
            sender: validators[2].public_key.clone(),
            signature: vec![0; 64],
        };
        // Third vote should trigger view change
        assert!(vc.submit_view_change(msg3).unwrap());
        assert_eq!(vc.current_view(), 1);
    }

    #[test]
    fn test_trigger_view_change() {
        let validators = create_test_validators(3);
        let mut vc = ViewChange::new(validators);

        assert_eq!(vc.current_view(), 0);
        
        let new_view = vc.trigger_view_change();
        assert_eq!(new_view, 1);
        assert_eq!(vc.current_view(), 1);
    }

    #[test]
    fn test_invalid_sender() {
        let validators = create_test_validators(3);
        let mut vc = ViewChange::new(validators);

        let invalid_msg = ViewChangeMessage {
            view_number: 1,
            sender: vec![99; 32], // Not a validator
            signature: vec![0; 64],
        };

        assert!(vc.submit_view_change(invalid_msg).is_err());
    }
}
