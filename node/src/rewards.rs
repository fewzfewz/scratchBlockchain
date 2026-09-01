//! # Reward Distribution Module
//!
//! This module handles validator rewards and penalties:
//! - Block proposer rewards
//! - Vote inclusion rewards  
//! - Transaction fee distribution
//! - Slashing penalties

use common::types::{Block, Transaction};
use std::collections::HashMap;

/// Reward configuration for the blockchain
#[derive(Debug, Clone)]
pub struct RewardConfig {
    /// Base block reward (in native tokens)
    pub base_block_reward: u64,

    /// Percentage of fees that goes to proposer (rest burned or distributed)
    pub proposer_fee_percentage: u8,

    /// Reward for validators who participated in finality
    pub vote_reward: u64,

    /// Slashing penalty percentage (0-100)
    pub slash_penalty_percentage: u8,
}

impl Default for RewardConfig {
    fn default() -> Self {
        Self {
            base_block_reward: 10_000_000_000_000_000_000, // 10 tokens (assuming 10^18 decimals)
            proposer_fee_percentage: 20,                   // 20% of fees to proposer
            vote_reward: 1_000_000_000_000_000_000,        // 1 token per vote
            slash_penalty_percentage: 5,                   // 5% stake slashed
        }
    }
}

/// Reward calculator for blocks and votes
pub struct RewardCalculator {
    config: RewardConfig,
    total_blocks_produced: HashMap<Vec<u8>, u64>,
    total_votes_cast: HashMap<Vec<u8>, u64>,
}

impl RewardCalculator {
    pub fn new(config: RewardConfig) -> Self {
        Self {
            config,
            total_blocks_produced: HashMap::new(),
            total_votes_cast: HashMap::new(),
        }
    }

    /// Calculate rewards for a block and distribute to participants
    ///
    /// # Returns
    /// Map of validator public key -> reward amount
    pub fn calculate_block_rewards(
        &mut self,
        block: &Block,
        proposer: Vec<u8>,
        voters: &[Vec<u8>],
        total_fees: u64,
    ) -> HashMap<Vec<u8>, i64> {
        let mut rewards = HashMap::new();

        // 1. Base block reward for proposer
        let proposer_reward = self.config.base_block_reward;
        *rewards.entry(proposer.clone()).or_insert(0) += proposer_reward as i64;

        // 2. Fee distribution (proposer gets percentage, rest burned)
        let proposer_fees = (total_fees * self.config.proposer_fee_percentage as u64) / 100;
        *rewards.entry(proposer).or_insert(0) += proposer_fees as i64;

        // 3. Vote rewards for validators who voted
        if !voters.is_empty() {
            let vote_reward_per_validator = self.config.vote_reward / (voters.len() as u64);
            for voter in voters {
                *rewards.entry(voter.clone()).or_insert(0) += vote_reward_per_validator as i64;
                *self.total_votes_cast.entry(voter.clone()).or_insert(0) += 1;
            }
        }

        // Track proposer stats
        *self
            .total_blocks_produced
            .entry(block.header.validator_set_id.to_le_bytes().to_vec())
            .or_insert(0) += 1;

        rewards
    }

    /// Calculate slashing penalty for malicious behavior
    pub fn calculate_slash_penalty(&self, stake: u64) -> u64 {
        (stake * self.config.slash_penalty_percentage as u64) / 100
    }

    /// Calculate annual percentage rate (APR) for validators
    pub fn calculate_apr(&self, blocks_per_year: u64, avg_stake: u64) -> f64 {
        let total_rewards_per_year = self.config.base_block_reward * blocks_per_year;
        (total_rewards_per_year as f64 / avg_stake as f64) * 100.0
    }

    /// Get validator statistics
    pub fn get_validator_stats(&self) -> Vec<(Vec<u8>, u64, u64)> {
        let mut stats = Vec::new();
        let all_keys: std::collections::HashSet<_> = self
            .total_blocks_produced
            .keys()
            .chain(self.total_votes_cast.keys())
            .collect();

        for key in all_keys {
            let blocks = *self.total_blocks_produced.get(key).unwrap_or(&0);
            let votes = *self.total_votes_cast.get(key).unwrap_or(&0);
            stats.push((key.clone(), blocks, votes));
        }

        stats.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by blocks produced
        stats
    }
}

/// Reward distribution manager that integrates with state
pub struct RewardManager {
    calculator: RewardCalculator,
    pending_rewards: HashMap<Vec<u8>, i64>, // Negative values = penalties
}

impl RewardManager {
    pub fn new(config: RewardConfig) -> Self {
        Self {
            calculator: RewardCalculator::new(config),
            pending_rewards: HashMap::new(),
        }
    }

    /// Process block rewards and update state
    pub fn process_block(
        &mut self,
        block: &Block,
        proposer: Vec<u8>,
        voters: &[Vec<u8>],
        total_fees: u64,
    ) -> HashMap<Vec<u8>, i64> {
        let rewards = self
            .calculator
            .calculate_block_rewards(block, proposer, voters, total_fees);

        // Add to pending rewards
        for (validator, amount) in &rewards {
            *self.pending_rewards.entry(validator.clone()).or_insert(0) += amount;
        }

        rewards
    }

    /// Apply slashing penalty
    pub fn slash_validator(&mut self, validator: Vec<u8>, stake: u64) -> u64 {
        let penalty = self.calculator.calculate_slash_penalty(stake);
        *self.pending_rewards.entry(validator).or_insert(0) -= penalty as i64;
        penalty
    }

    /// Get and clear pending rewards for a validator
    pub fn claim_rewards(&mut self, validator: &[u8]) -> i64 {
        self.pending_rewards.remove(validator).unwrap_or(0)
    }

    /// Check if validator has pending penalties
    pub fn has_penalties(&self, validator: &[u8]) -> bool {
        self.pending_rewards
            .get(validator)
            .map_or(false, |&v| v < 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reward_calculation() {
        let config = RewardConfig::default();
        let mut calc = RewardCalculator::new(config);

        let proposer = vec![1u8];
        let voters = vec![vec![1u8], vec![2u8], vec![3u8]];
        let block = Block::genesis();
        let total_fees = 1000;

        let rewards = calc.calculate_block_rewards(&block, proposer.clone(), &voters, total_fees);

        // Proposer should get base reward + fees
        assert!(rewards.get(&proposer).unwrap() > &0);

        // Each voter should get vote reward
        for voter in &voters {
            assert!(rewards.get(voter).unwrap() > &0);
        }
    }

    #[test]
    fn test_slashing_penalty() {
        let config = RewardConfig::default();
        let calc = RewardCalculator::new(config);

        let stake = 10_000_000_000_000_000_000u64; // 10 tokens
        let penalty = calc.calculate_slash_penalty(stake);

        assert_eq!(penalty, 500_000_000_000_000_000); // 5% of 10 tokens
    }
}
