//! # Consensus Module
//!
//! This module implements the blockchain consensus mechanisms including:
//! - BFT consensus (from bft module)
//! - GRANDPA-style finality gadget
//! - Slashing for malicious behavior
//! - View-change protocol for leader rotation
//!
//! ## Design
//! The consensus system uses a hybrid approach:
//! - **BFT** for block production and immediate confirmation
//! - **Finality Gadget** for irreversible finalization
//! - **Slashing** to punish validators who violate safety rules

pub mod bft;
pub mod slashing;

pub use bft::{BftEngine, BftEvent};
pub use common::consensus_types::ValidatorInfo;
use common::traits::Consensus;
use common::types::{Block, Header};
use ed25519_dalek::Verifier;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use tracing::{info, warn, debug};

// ============================================================================
// Slashing Conditions
// ============================================================================

/// Conditions that can cause a validator to be slashed (punished)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlashingCondition {
    /// Validator signed two different blocks at the same height
    DoubleSign { height: u64, validator: Vec<u8> },
    
    /// Validator produced conflicting votes for the same round
    Equivocation { height: u64, validator: Vec<u8> },
    
    /// Validator deliberately withheld blocks (proven by timeout)
    Censorship { validator: Vec<u8> },
    
    /// Validator was offline for too long
    LivenessFailure { validator: Vec<u8>, missed_blocks: u64 },
}

// ============================================================================
// Finality Gadget (GRANDPA-style)
// ============================================================================

/// Finality vote from a validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityVote {
    /// Hash of the block being voted for
    pub block_hash: [u8; 32],
    
    /// Block number (height)
    pub block_number: u64,
    
    /// Validator's public key
    pub voter: Vec<u8>,
    
    /// Ed25519 signature of (block_hash + block_number + voter)
    pub signature: Vec<u8>,
}

impl FinalityVote {
    /// Serialize vote fields for signing (excludes signature itself)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.block_hash);
        bytes.extend_from_slice(&self.block_number.to_le_bytes());
        bytes.extend_from_slice(&self.voter);
        bytes
    }
    
    /// Verify the vote's signature
    pub fn verify(&self) -> Result<bool, Box<dyn Error>> {
        use ed25519_dalek::{Signature, VerifyingKey};
        
        if self.signature.len() != 64 {
            return Ok(false);
        }
        
        let signature = Signature::from_slice(&self.signature)?;
        let verifying_key = VerifyingKey::from_bytes(self.voter.as_slice().try_into()?)?;
        let message = self.to_bytes();
        
        Ok(verifying_key.verify(&message, &signature).is_ok())
    }
}

/// GRANDPA-style finality gadget
/// 
/// This implements a finality gadget similar to Polkadot's GRANDPA,
/// which runs alongside block production and finalizes blocks in batches.
pub struct FinalityGadget {
    /// Validators participating in finality
    validators: HashMap<Vec<u8>, ValidatorInfo>,
    
    /// Total stake across all validators
    total_stake: u64,
    
    /// Votes for each block number (prevotes)
    prevotes: HashMap<u64, Vec<FinalityVote>>,
    
    /// Votes for each block number (precommits)
    precommits: HashMap<u64, Vec<FinalityVote>>,
    
    /// Finalized blocks (height -> hash)
    finalized_blocks: HashMap<u64, [u8; 32]>,
    
    /// Current round number
    current_round: u64,
    
    /// Last finalized height
    last_finalized: u64,
    
    /// Blocks that are voted but not yet finalized
    pending_finalization: HashSet<u64>,
}

impl FinalityGadget {
    /// Create a new finality gadget with the given validator set
    pub fn new(validators: Vec<ValidatorInfo>) -> Self {
        let total_stake: u64 = validators.iter().map(|v| v.stake).sum();
        let mut validator_map = HashMap::new();
        
        for v in validators {
            validator_map.insert(v.public_key.clone(), v);
        }
        
        Self {
            validators: validator_map,
            total_stake,
            prevotes: HashMap::new(),
            precommits: HashMap::new(),
            finalized_blocks: HashMap::new(),
            current_round: 0,
            last_finalized: 0,
            pending_finalization: HashSet::new(),
        }
    }
    
    /// Submit a prevote for a block
    pub fn prevote(&mut self, vote: FinalityVote) -> Result<(), Box<dyn Error>> {
        // Verify the voter is a validator
        let validator = self.validators
            .get(&vote.voter)
            .ok_or("Voter is not a validator")?;
        
        if validator.slashed {
            return Err("Validator has been slashed".into());
        }
        
        // Verify signature
        if !vote.verify()? {
            return Err("Invalid vote signature".into());
        }
        
        // Check for equivocation (voting for different blocks at same height)
        let mut equivocating: Option<Vec<u8>> = None;
        if let Some(existing_votes) = self.prevotes.get(&vote.block_number) {
            for existing in existing_votes {
                if existing.voter == vote.voter && existing.block_hash != vote.block_hash {
                    warn!("Equivocation detected: validator voted for different blocks at height {}", 
                          vote.block_number);
                    equivocating = Some(vote.voter.clone());
                    break;
                }
            }
        }
        if let Some(voter) = equivocating {
            let _ = self.slash(&voter);
        }
        
        // Add to prevotes
        let bn = vote.block_number;
        self.prevotes
            .entry(bn)
            .or_default()
            .push(vote);
        
        debug!("Prevote recorded for height {}", bn);
        Ok(())
    }
    
    /// Submit a precommit for a block
    pub fn precommit(&mut self, vote: FinalityVote) -> Result<(), Box<dyn Error>> {
        // Verify the voter is a validator
        let validator = self.validators
            .get(&vote.voter)
            .ok_or("Voter is not a validator")?;
        
        if validator.slashed {
            return Err("Validator has been slashed".into());
        }
        
        // Verify signature
        if !vote.verify()? {
            return Err("Invalid vote signature".into());
        }
        
        // Check for equivocation
        let mut equivocating: Option<Vec<u8>> = None;
        if let Some(existing_votes) = self.precommits.get(&vote.block_number) {
            for existing in existing_votes {
                if existing.voter == vote.voter && existing.block_hash != vote.block_hash {
                    warn!("Precommit equivocation detected for height {}", vote.block_number);
                    equivocating = Some(vote.voter.clone());
                    break;
                }
            }
        }
        if let Some(voter) = equivocating {
            let _ = self.slash(&voter);
        }
        
        // Add to precommits
        let block_number = vote.block_number;
        self.precommits
            .entry(block_number)
            .or_default()
            .push(vote);
        
        // Try to finalize
        self.try_finalize(block_number)?;
        
        Ok(())
    }
    
    /// Try to finalize a block if we have enough precommits
    fn try_finalize(&mut self, block_number: u64) -> Result<(), Box<dyn Error>> {
        let precommits = match self.precommits.get(&block_number) {
            Some(votes) => votes,
            None => return Ok(()),
        };
        
        // FIX: Properly count votes per block hash (find hash with 2/3+ stake)
        let threshold = (self.total_stake * 2) / 3; // 2/3 threshold (strictly greater)
        
        // Count stake per block hash
        let mut stake_by_hash: HashMap<[u8; 32], u64> = HashMap::new();
        
        for vote in precommits {
            if let Some(validator) = self.validators.get(&vote.voter) {
                if !validator.slashed {
                    *stake_by_hash.entry(vote.block_hash).or_insert(0) += validator.stake;
                }
            }
        }
        
        // Find the hash with the most stake (must exceed threshold)
        let mut best_hash: Option<[u8; 32]> = None;
        let mut best_stake = 0u64;
        
        for (hash, stake) in stake_by_hash {
            if stake > best_stake {
                best_stake = stake;
                best_hash = Some(hash);
            }
        }
        
        // Check if we have enough stake to finalize
        if best_stake > threshold {
            if let Some(hash) = best_hash {
                // Check if this block can be finalized (parent must be finalized)
                if block_number == self.last_finalized + 1 || 
                   self.finalized_blocks.contains_key(&(block_number - 1)) {
                    
                    self.finalized_blocks.insert(block_number, hash);
                    self.last_finalized = block_number;
                    self.pending_finalization.remove(&block_number);
                    
                    info!("✓ Block {} finalized with hash {:?} (stake: {}/{})", 
                          block_number, hex::encode(&hash[..4]), best_stake, self.total_stake);
                } else {
                    // This block is not contiguous - can't finalize yet
                    self.pending_finalization.insert(block_number);
                    debug!("Block {} pending finalization (parent not finalized)", block_number);
                }
            }
        }
        
        // Try to finalize any pending blocks that are now contiguous
        let mut to_finalize: Vec<u64> = self.pending_finalization
            .iter()
            .filter(|h| **h == self.last_finalized + 1)
            .copied()
            .collect();
        
        while let Some(height) = to_finalize.pop() {
            if let Some(votes) = self.precommits.get(&height) {
                // Re-check threshold
                let mut stake = 0u64;
                for vote in votes {
                    if let Some(validator) = self.validators.get(&vote.voter) {
                        if !validator.slashed {
                            stake += validator.stake;
                        }
                    }
                }
                
                if stake > threshold {
                    if let Some(vote) = votes.first() {
                        self.finalized_blocks.insert(height, vote.block_hash);
                        self.last_finalized = height;
                        self.pending_finalization.remove(&height);
                        
                        // Check next height
                        if self.pending_finalization.contains(&(height + 1)) {
                            to_finalize.push(height + 1);
                        }
                    }
                }
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
    
    /// Get the last finalized block height
    pub fn last_finalized_height(&self) -> u64 {
        self.last_finalized
    }
    
    /// Slash a validator for misbehavior
    pub fn slash(&mut self, validator_pubkey: &[u8]) -> Result<(), Box<dyn Error>> {
        if let Some(validator) = self.validators.get_mut(validator_pubkey) {
            if !validator.slashed {
                validator.slashed = true;
                let slashed_stake = validator.stake;
                validator.stake = 0; // Confiscate stake
                
                info!("⚠ Validator slashed: {:?}, stake confiscated: {}", 
                      hex::encode(&validator_pubkey[..4]), slashed_stake);
                return Ok(());
            }
        }
        Err("Validator not found".into())
    }
    
    /// Get the current round
    pub fn current_round(&self) -> u64 {
        self.current_round
    }
    
    /// Advance to next round
    pub fn next_round(&mut self) {
        self.current_round += 1;
        debug!("Finality gadget advanced to round {}", self.current_round);
    }
}

// ============================================================================
// Enhanced Consensus with Slashing and Finality
// ============================================================================

/// Enhanced consensus that combines BFT, finality, and slashing
pub struct EnhancedConsensus {
    /// Set of active validators (public keys)
    validators: HashSet<Vec<u8>>,
    
    /// Validator info with stakes
    validator_info: Vec<ValidatorInfo>,
    
    /// Finality gadget for irreversible finalization
    finality_gadget: FinalityGadget,
    
    /// Track seen blocks to detect double-signing
    seen_blocks: HashMap<u64, Vec<[u8; 32]>>,
    
    /// Slashing events that have occurred
    slashing_events: Vec<SlashingCondition>,
    
    /// Validator liveness tracking
    validator_liveness: HashMap<Vec<u8>, u64>, // validator -> last seen height
}

impl EnhancedConsensus {
    /// Create a new enhanced consensus instance
    pub fn new(validator_infos: Vec<ValidatorInfo>) -> Self {
        let validators: HashSet<Vec<u8>> = validator_infos
            .iter()
            .map(|v| v.public_key.clone())
            .collect();
        
        let finality_gadget = FinalityGadget::new(validator_infos.clone());
        
        Self {
            validator_info: validator_infos,
            validators,
            finality_gadget,
            seen_blocks: HashMap::new(),
            slashing_events: Vec::new(),
            validator_liveness: HashMap::new(),
        }
    }
    
    /// Detect and handle slashing conditions
    pub fn check_slashing_conditions(
        &mut self,
        header: &Header,
        validator_pubkey: &[u8],
    ) -> Result<(), Box<dyn Error>> {
        let height = header.slot;
        
        // Update liveness
        self.validator_liveness
            .insert(validator_pubkey.to_vec(), height);
        
        // Check for double-signing (two different blocks at same height)
        let blocks_at_height = self.seen_blocks.entry(height).or_default();
        let current_hash = header.parent_hash;
        
        for existing_hash in blocks_at_height.iter() {
            if existing_hash != &current_hash {
                // Double-sign detected!
                warn!("Double-sign detected at height {} by validator", height);
                
                let condition = SlashingCondition::DoubleSign {
                    height,
                    validator: validator_pubkey.to_vec(),
                };
                self.slashing_events.push(condition);
                self.finality_gadget.slash(validator_pubkey)?;
                
                return Err("Double-sign detected - validator slashed".into());
            }
        }
        
        blocks_at_height.push(current_hash);
        Ok(())
    }
    
    /// Get all slashing events
    pub fn get_slashing_events(&self) -> &[SlashingCondition] {
        &self.slashing_events
    }
    
    /// Submit a prevote to the finality gadget
    pub fn submit_prevote(&mut self, vote: FinalityVote) -> Result<(), Box<dyn Error>> {
        self.finality_gadget.prevote(vote)
    }
    
    /// Submit a precommit to the finality gadget
    pub fn submit_precommit(&mut self, vote: FinalityVote) -> Result<(), Box<dyn Error>> {
        self.finality_gadget.precommit(vote)
    }
    
    /// Check if a block is finalized
    pub fn is_block_finalized(&self, block_number: u64) -> bool {
        self.finality_gadget.is_finalized(block_number)
    }
    
    /// Get finalized block hash
    pub fn get_finalized_hash(&self, block_number: u64) -> Option<[u8; 32]> {
        self.finality_gadget.get_finalized_hash(block_number)
    }
    
    /// Get validator set
    pub fn get_validators(&self) -> &HashSet<Vec<u8>> {
        &self.validators
    }
    
    /// Get validator info
    pub fn get_validator_info(&self) -> &[ValidatorInfo] {
        &self.validator_info
    }
    
    /// Check if a public key is a validator
    pub fn is_validator(&self, pubkey: &[u8]) -> bool {
        self.validators.contains(pubkey)
    }
    
    /// Get liveness status of validators
    pub fn get_liveness(&self, current_height: u64, max_missed: u64) -> Vec<&Vec<u8>> {
        let mut offline = Vec::new();
        
        for (validator, last_seen) in &self.validator_liveness {
            if current_height.saturating_sub(*last_seen) > max_missed {
                offline.push(validator);
            }
        }
        
        offline
    }
    
    /// Advance the finality gadget to the next round
    pub fn next_round(&mut self) {
        self.finality_gadget.next_round();
    }
}

// ============================================================================
// Consensus Trait Implementations
// ============================================================================

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
        
        // Verify the signature against known validators
        use ed25519_dalek::{Signature, VerifyingKey};
        
        let signature = Signature::from_slice(&header.signature)?;
        let message = header.hash();
        
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
        
        // Additional block validation could go here
        // - Verify block timestamp is reasonable
        // - Verify block gas limit
        // - Verify transactions root matches
        
        Ok(())
    }
    
    fn is_finalized(&self, hash: &[u8; 32]) -> bool {
        // Check if any finalized block has this hash
        for (_height, finalized_hash) in &self.finality_gadget.finalized_blocks {
            if finalized_hash == hash {
                return true;
            }
        }
        false
    }
}

// ============================================================================
// Simple Consensus (for testing/backward compatibility)
// ============================================================================

/// Simple consensus that skips signature verification (for testing only)
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
        
        // Simplified: just check that signature exists
        Ok(())
    }
    
    fn verify_block(&self, block: &Block) -> Result<(), Box<dyn Error>> {
        self.verify_header(&block.header)
    }
    
    fn is_finalized(&self, _hash: &[u8; 32]) -> bool {
        true // Simple consensus finalizes everything immediately
    }
}

// ============================================================================
// View-Change Protocol for Leader Rotation
// ============================================================================

/// Message for initiating a view change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewChangeMessage {
    /// New view number being proposed
    pub view_number: u64,
    
    /// Sender's public key
    pub sender: Vec<u8>,
    
    /// Signature of (view_number + sender)
    pub signature: Vec<u8>,
}

impl ViewChangeMessage {
    /// Verify the message signature
    pub fn verify(&self) -> Result<bool, Box<dyn Error>> {
        use ed25519_dalek::{Signature, VerifyingKey};
        
        if self.signature.len() != 64 {
            return Ok(false);
        }
        
        let signature = Signature::from_slice(&self.signature)?;
        let verifying_key = VerifyingKey::from_bytes(self.sender.as_slice().try_into()?)?;
        
        let mut message = Vec::new();
        message.extend_from_slice(&self.view_number.to_le_bytes());
        message.extend_from_slice(&self.sender);
        
        Ok(verifying_key.verify(&message, &signature).is_ok())
    }
}

/// View-change protocol for leader rotation and fault tolerance
pub struct ViewChange {
    /// Current view number
    current_view: u64,
    
    /// List of validators (ordered for round-robin selection)
    validators: Vec<ValidatorInfo>,
    
    /// Votes for view changes (view_number -> list of votes)
    view_change_votes: HashMap<u64, Vec<ViewChangeMessage>>,
    
    /// Threshold for view change (2/3 + 1 of validators)
    view_change_threshold: usize,
    
    /// Whether a view change is in progress
    view_change_in_progress: bool,
}

impl ViewChange {
    /// Create a new view-change protocol instance
    pub fn new(validators: Vec<ValidatorInfo>) -> Self {
        let threshold = (validators.len() * 2) / 3 + 1; // 2/3 + 1 for BFT
        Self {
            current_view: 0,
            validators,
            view_change_votes: HashMap::new(),
            view_change_threshold: threshold,
            view_change_in_progress: false,
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
    /// Returns Ok(true) if view change occurred, Ok(false) if waiting for more votes
    pub fn submit_view_change(&mut self, msg: ViewChangeMessage) -> Result<bool, Box<dyn Error>> {
        // Verify sender is a validator
        if !self.validators.iter().any(|v| v.public_key == msg.sender) {
            return Err("Sender is not a validator".into());
        }
        
        // Verify signature
        if !msg.verify()? {
            return Err("Invalid view-change signature".into());
        }
        
        let view = msg.view_number;
        
        // Must be for a future view
        if view <= self.current_view {
            debug!("View change for past view {} ignored (current: {})", view, self.current_view);
            return Ok(false);
        }
        
        // Add vote
        let votes = self.view_change_votes
            .entry(view)
            .or_default();
        
        // Check for duplicate votes from same sender
        if votes.iter().any(|v| v.sender == msg.sender) {
            return Err("Duplicate view-change vote".into());
        }
        
        votes.push(msg);
        
        // Check if we have enough votes to change view
        if votes.len() >= self.view_change_threshold {
            self.current_view = view;
            self.view_change_in_progress = false;
            info!("✓ View changed to {}", view);
            return Ok(true);
        }
        
        self.view_change_in_progress = true;
        Ok(false)
    }
    
    /// Trigger view change (e.g., due to timeout or leader failure)
    pub fn trigger_view_change(&mut self) -> u64 {
        let new_view = self.current_view + 1;
        self.current_view = new_view;
        self.view_change_in_progress = true;
        
        info!("View change triggered: now at view {}", new_view);
        new_view
    }
    
    /// Check if a view change is in progress
    pub fn is_view_change_in_progress(&self) -> bool {
        self.view_change_in_progress
    }
    
    /// Get the number of votes for a view change
    pub fn get_vote_count(&self, view: u64) -> usize {
        self.view_change_votes.get(&view).map(|v| v.len()).unwrap_or(0)
    }
    
    /// Reset view change state (after successful round)
    pub fn reset(&mut self) {
        self.view_change_votes.clear();
        self.view_change_in_progress = false;
    }
}

// ============================================================================
// Module Initialization
// ============================================================================

pub fn init() {
    println!("Consensus module initialized");
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use common::crypto::SigningKey;
    
    fn create_test_validator(key: &SigningKey, stake: u64) -> ValidatorInfo {
        ValidatorInfo {
            public_key: key.public_key(),
            stake,
            slashed: false,
        }
    }
    
    #[test]
    fn test_finality_gadget_creation() {
        let key = SigningKey::generate();
        let validators = vec![
            create_test_validator(&key, 100),
        ];
        
        let gadget = FinalityGadget::new(validators);
        assert_eq!(gadget.current_round(), 0);
        assert_eq!(gadget.last_finalized_height(), 0);
    }
    
    #[test]
    fn test_slashing() {
        let key = SigningKey::generate();
        let validators = vec![
            create_test_validator(&key, 100),
        ];
        
        let mut gadget = FinalityGadget::new(validators);
        assert!(gadget.slash(&key.public_key()).is_ok());
        
        // Verify validator was slashed by trying to find it (it won't be in validators map)
        // In our implementation, the validator entry remains but with slashed=true and stake=0
        // We can't directly access it from the outside, but the slash method succeeded
    }
    
    #[test]
    fn test_view_change_leader_rotation() {
        let keys: Vec<SigningKey> = (0..4).map(|_| SigningKey::generate()).collect();
        let validators: Vec<ValidatorInfo> = keys.iter()
            .enumerate()
            .map(|(i, key)| ValidatorInfo {
                public_key: key.public_key(),
                stake: 100,
                slashed: false,
            })
            .collect();
        
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
        let keys: Vec<SigningKey> = (0..4).map(|_| SigningKey::generate()).collect();
        let validators: Vec<ValidatorInfo> = keys.iter()
            .map(|key| ValidatorInfo {
                public_key: key.public_key(),
                stake: 100,
                slashed: false,
            })
            .collect();
        
        let mut vc = ViewChange::new(validators);
        assert_eq!(vc.current_view(), 0);
        
        // Create signed view-change messages for view 1
        let mut messages = Vec::new();
        for key in &keys {
            let mut message = ViewChangeMessage {
                view_number: 1,
                sender: key.public_key(),
                signature: vec![],
            };
            
            // Sign the message
            let mut msg_bytes = Vec::new();
            msg_bytes.extend_from_slice(&message.view_number.to_le_bytes());
            msg_bytes.extend_from_slice(&message.sender);
            message.signature = key.sign(&msg_bytes);
            
            messages.push(message);
        }
        
        // Need 3 votes (2/3 of 4 = 2.66, rounded up to 3)
        assert!(!vc.submit_view_change(messages[0].clone()).unwrap());
        assert!(!vc.submit_view_change(messages[1].clone()).unwrap());
        // Third vote should trigger view change
        assert!(vc.submit_view_change(messages[2].clone()).unwrap());
        assert_eq!(vc.current_view(), 1);
    }
}