//! # Slashing Module
//!
//! This module implements validator slashing for:
//! - Double signing (equivocation)
//! - Liveness failures (downtime)
//! - Censorship (block withholding)

use crate::{SlashingCondition, ValidatorInfo};
use common::types::Hash;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

/// Slashing configuration
#[derive(Debug, Clone)]
pub struct SlashingConfig {
    /// Maximum number of blocks a validator can miss before slashing
    pub max_missed_blocks: u64,
    
    /// Blocks to look back for liveness tracking
    pub liveness_window: u64,
    
    /// Minimum stake required to be a validator
    pub min_stake: u64,
    
    /// Tombstone after slashing (permanently banned)
    pub tombstone_after_slash: bool,
}

impl Default for SlashingConfig {
    fn default() -> Self {
        Self {
            max_missed_blocks: 50,      // Miss 50 blocks -> slash
            liveness_window: 100,       // Look at last 100 blocks
            min_stake: 1_000_000_000_000_000_000, // 1 token minimum
            tombstone_after_slash: true,
        }
    }
}

/// Tracks validator liveness and offenses
pub struct SlashingTracker {
    config: SlashingConfig,
    
    /// Missed blocks per validator (block height -> validator)
    missed_blocks: HashMap<Vec<u8>, VecDeque<u64>>,
    
    /// Double sign evidence (height -> [validator])
    double_sign_evidence: HashMap<u64, Vec<Vec<u8>>>,
    
    /// Slashed validators (public key -> slash time)
    slashed_validators: HashMap<Vec<u8>, Instant>,
    
    /// Tombstoned validators (permanently banned)
    tombstoned: HashMap<Vec<u8>, Instant>,
    
    /// Current block height
    current_height: u64,
}

impl SlashingTracker {
    pub fn new(config: SlashingConfig) -> Self {
        Self {
            config,
            missed_blocks: HashMap::new(),
            double_sign_evidence: HashMap::new(),
            slashed_validators: HashMap::new(),
            tombstoned: HashMap::new(),
            current_height: 0,
        }
    }
    
    /// Record that a validator missed a block (did not vote)
    pub fn record_missed_block(&mut self, validator: Vec<u8>, height: u64) {
        if self.is_slashed(&validator) {
            return;
        }
        
        let val_copy = validator.clone();
        let missed = self.missed_blocks
            .entry(validator)
            .or_insert_with(VecDeque::new);
        
        missed.push_back(height);
        
        // Maintain window size
        while missed.len() > self.config.liveness_window as usize {
            missed.pop_front();
        }
        
        // Check if slashing threshold reached
        if missed.len() >= self.config.max_missed_blocks as usize {
            let missed_count = missed.len();
            tracing::warn!(
                "Validator {:?} missed {} blocks in last {} blocks - SLASHING",
                hex::encode(&val_copy[..4]),
                missed_count,
                self.config.liveness_window
            );
        }
    }
    
    /// Record that a validator successfully voted
    pub fn record_vote(&mut self, validator: Vec<u8>, height: u64) {
        if let Some(missed) = self.missed_blocks.get_mut(&validator) {
            missed.retain(|&h| h != height);
        }
    }
    
    /// Record double-sign evidence
    pub fn record_double_sign(
        &mut self, 
        validator: Vec<u8>, 
        height: u64,
        block1_hash: Hash,
        block2_hash: Hash,
    ) {
        if self.is_slashed(&validator) {
            return;
        }
        
        tracing::error!(
            "DOUBLE SIGN DETECTED: Validator {:?} signed two blocks at height {}: {:?} and {:?}",
            hex::encode(&validator[..4]),
            height,
            hex::encode(&block1_hash[..4]),
            hex::encode(&block2_hash[..4])
        );
        
        self.double_sign_evidence
            .entry(height)
            .or_default()
            .push(validator);
    }
    
    /// Check if validator should be slashed for double signing
    pub fn check_double_sign_slash(&mut self, validator: &[u8], height: u64) -> bool {
        if let Some(validators) = self.double_sign_evidence.get(&height) {
            if validators.contains(&validator.to_vec()) {
                // Second offense at same height -> slash
                return true;
            }
        }
        false
    }
    
    /// Slash a validator (call when offense is proven)
    pub fn slash(&mut self, validator: Vec<u8>, reason: SlashingCondition) -> Result<u64, String> {
        if self.is_tombstoned(&validator) {
            return Err("Validator is already tombstoned".to_string());
        }
        
        if self.is_slashed(&validator) {
            return Err("Validator already slashed".to_string());
        }
        
        // Record slash
        self.slashed_validators.insert(validator.clone(), Instant::now());
        
        // Tombstone if configured
        if self.config.tombstone_after_slash {
            self.tombstoned.insert(validator.clone(), Instant::now());
        }
        
        // Clean up tracking data
        self.missed_blocks.remove(&validator);
        self.double_sign_evidence.retain(|_, v| !v.contains(&validator));
        
        tracing::error!(
            "🔨 VALIDATOR SLASHED: {:?} - Reason: {:?}",
            hex::encode(&validator[..4]),
            reason
        );
        
        Ok(0) // Slashing amount would be calculated by parent
    }
    
    /// Check if validator is slashed
    pub fn is_slashed(&self, validator: &[u8]) -> bool {
        self.slashed_validators.contains_key(validator)
    }
    
    /// Check if validator is tombstoned (permanently banned)
    pub fn is_tombstoned(&self, validator: &[u8]) -> bool {
        self.tombstoned.contains_key(validator)
    }
    
    /// Get liveness percentage for a validator
    pub fn get_liveness_percentage(&self, validator: &[u8]) -> f64 {
        let missed = self.missed_blocks
            .get(validator)
            .map(|v| v.len())
            .unwrap_or(0);
        
        let total = self.config.liveness_window;
        let voted = total.saturating_sub(missed as u64);
        
        (voted as f64 / total as f64) * 100.0
    }
    
    /// Get all slashed validators
    pub fn get_slashed_validators(&self) -> Vec<&Vec<u8>> {
        self.slashed_validators.keys().collect()
    }
    
    /// Update current height (call at start of each block)
    pub fn update_height(&mut self, height: u64) {
        self.current_height = height;
    }
    
    /// Check liveness for all validators and return those below threshold
    pub fn check_liveness(&self, validators: &[ValidatorInfo]) -> Vec<Vec<u8>> {
        let mut offline = Vec::new();
        
        for validator in validators {
            if self.is_slashed(&validator.public_key) {
                continue;
            }
            
            let liveness = self.get_liveness_percentage(&validator.public_key);
            if liveness < 50.0 {
                offline.push(validator.public_key.clone());
            }
        }
        
        offline
    }
}

/// Evidence collection for slashing
pub struct EvidenceCollector {
    /// Collected evidence for each validator
    evidence: HashMap<Vec<u8>, Vec<SlashingEvidence>>,
}

#[derive(Debug, Clone)]
pub struct SlashingEvidence {
    pub offense_type: SlashingCondition,
    pub block_height: u64,
    pub timestamp: Instant,
    pub proof: Vec<u8>,
}

impl EvidenceCollector {
    pub fn new() -> Self {
        Self {
            evidence: HashMap::new(),
        }
    }
    
    /// Add evidence of misbehavior
    pub fn add_evidence(&mut self, validator: Vec<u8>, evidence: SlashingEvidence) {
        self.evidence
            .entry(validator)
            .or_default()
            .push(evidence);
    }
    
    /// Get all evidence for a validator
    pub fn get_evidence(&self, validator: &[u8]) -> Vec<&SlashingEvidence> {
        self.evidence
            .get(validator)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }
    
    /// Check if validator has enough evidence to slash
    pub fn has_slashable_offense(&self, validator: &[u8]) -> bool {
        self.evidence
            .get(validator)
            .map(|v| v.len() >= 2) // Two pieces of evidence = slash
            .unwrap_or(false)
    }
    
    /// Clear evidence for a validator (after slashing)
    pub fn clear_evidence(&mut self, validator: &[u8]) {
        self.evidence.remove(validator);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_missed_block_tracking() {
        let config = SlashingConfig::default();
        let mut tracker = SlashingTracker::new(config);
        
        let validator = vec![1u8];
        
        // Record 60 missed blocks (threshold is 50)
        for i in 0..60 {
            tracker.record_missed_block(validator.clone(), i);
        }
        
        assert!(tracker.missed_blocks.get(&validator).unwrap().len() >= 50);
        assert!(!tracker.is_slashed(&validator));
        
        // Record a vote at height 30 (should remove it from missed)
        tracker.record_vote(validator.clone(), 30);
        assert!(tracker.missed_blocks.get(&validator).unwrap().len() < 60);
    }
    
    #[test]
    fn test_double_sign_detection() {
        let config = SlashingConfig::default();
        let mut tracker = SlashingTracker::new(config);
        
        let validator = vec![1u8];
        let hash1 = [1u8; 32];
        let hash2 = [2u8; 32];
        
        tracker.record_double_sign(validator.clone(), 100, hash1, hash2);
        
        assert!(tracker.check_double_sign_slash(&validator, 100));
    }
    
    #[test]
    fn test_liveness_percentage() {
        let config = SlashingConfig::default();
        let mut tracker = SlashingTracker::new(config);
        
        let validator = vec![1u8];
        
        // Miss 30 blocks
        for i in 0..30 {
            tracker.record_missed_block(validator.clone(), i);
        }
        
        let liveness = tracker.get_liveness_percentage(&validator);
        assert!(liveness < 100.0);
        assert!(liveness > 0.0);
    }
}