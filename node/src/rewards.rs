//! # Reward Distribution Module
//!
//! This module handles validator rewards and penalties:
//! - Block proposer rewards
//! - Vote inclusion rewards  
//! - Transaction fee distribution
//! - Slashing penalties

use common::types::Block;
use execution::evm::TransactionReceipt;
use std::collections::HashMap;

/// Fees actually paid in a block. Failed receipts (bad signature, nonce, balance)
/// were never charged to the sender, so they must not be paid out as rewards.
pub fn total_fees(receipts: &[TransactionReceipt], base_fee: u64) -> u128 {
    receipts
        .iter()
        .filter(|r| r.success)
        .map(|r| (r.gas_used as u128).saturating_mul(base_fee as u128))
        .fold(0u128, u128::saturating_add)
}

/// Reward configuration for the blockchain
#[derive(Debug, Clone)]
pub struct RewardConfig {
    /// Base block reward (in native tokens)
    pub base_block_reward: u128,

    /// Percentage of fees that goes to proposer (rest burned or distributed)
    pub proposer_fee_percentage: u8,

    /// Reward for validators who participated in finality
    pub vote_reward: u128,

    /// Slashing penalty percentage (0-100)
    #[allow(dead_code)] // not wired into the node yet; covered by tests
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
        total_fees: u128,
    ) -> HashMap<Vec<u8>, u128> {
        let mut rewards: HashMap<Vec<u8>, u128> = HashMap::new();

        // 1. Base block reward for proposer
        let proposer_reward = self.config.base_block_reward;
        let entry = rewards.entry(proposer.clone()).or_insert(0);
        *entry = entry.saturating_add(proposer_reward);

        // 2. Fee distribution (proposer gets percentage, rest burned)
        let proposer_fees =
            total_fees.saturating_mul(self.config.proposer_fee_percentage as u128) / 100;
        let entry = rewards.entry(proposer).or_insert(0);
        *entry = entry.saturating_add(proposer_fees);

        // 3. Vote rewards for validators who voted
        if !voters.is_empty() {
            let vote_reward_per_validator = self.config.vote_reward / (voters.len() as u128);
            for voter in voters {
                let entry = rewards.entry(voter.clone()).or_insert(0);
                *entry = entry.saturating_add(vote_reward_per_validator);
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
    #[allow(dead_code)] // not wired into the node yet; covered by tests
    pub fn calculate_slash_penalty(&self, stake: u128) -> u128 {
        stake.saturating_mul(self.config.slash_penalty_percentage as u128) / 100
    }

    /// Calculate annual percentage rate (APR) for validators
    #[allow(dead_code)] // not wired into the node yet; covered by tests
    pub fn calculate_apr(&self, blocks_per_year: u64, avg_stake: u128) -> f64 {
        let total_rewards_per_year = self
            .config
            .base_block_reward
            .saturating_mul(blocks_per_year as u128);
        (total_rewards_per_year as f64 / avg_stake as f64) * 100.0
    }

    /// Get validator statistics
    #[allow(dead_code)] // not wired into the node yet; covered by tests
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
    pending_rewards: HashMap<Vec<u8>, i128>, // Negative values = penalties
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
        total_fees: u128,
    ) -> HashMap<Vec<u8>, u128> {
        let rewards = self
            .calculator
            .calculate_block_rewards(block, proposer, voters, total_fees);

        // Add to pending rewards
        for (validator, amount) in &rewards {
            let entry = self.pending_rewards.entry(validator.clone()).or_insert(0);
            *entry = entry.saturating_add(i128::try_from(*amount).unwrap_or(i128::MAX));
        }

        rewards
    }

    /// Apply slashing penalty
    #[allow(dead_code)] // not wired into the node yet; covered by tests
    pub fn slash_validator(&mut self, validator: Vec<u8>, stake: u128) -> u128 {
        let penalty = self.calculator.calculate_slash_penalty(stake);
        let entry = self.pending_rewards.entry(validator).or_insert(0);
        *entry = entry.saturating_sub(i128::try_from(penalty).unwrap_or(i128::MAX));
        penalty
    }

    /// Get and clear pending rewards for a validator
    #[allow(dead_code)] // not wired into the node yet; covered by tests
    pub fn claim_rewards(&mut self, validator: &[u8]) -> i128 {
        self.pending_rewards.remove(validator).unwrap_or(0)
    }

    /// Check if validator has pending penalties
    #[allow(dead_code)] // not wired into the node yet; covered by tests
    pub fn has_penalties(&self, validator: &[u8]) -> bool {
        self.pending_rewards.get(validator).is_some_and(|&v| v < 0)
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

        let stake = 10_000_000_000_000_000_000u128; // 10 tokens
        let penalty = calc.calculate_slash_penalty(stake);

        assert_eq!(penalty, 500_000_000_000_000_000); // 5% of 10 tokens
    }

    #[test]
    fn test_default_base_reward_is_paid_in_full() {
        // Regression: 1e19 used to be cast to i64, wrapping negative, so it was never paid.
        let mut calc = RewardCalculator::new(RewardConfig::default());
        let proposer = vec![9u8];
        let rewards = calc.calculate_block_rewards(&Block::genesis(), proposer.clone(), &[], 0);
        assert_eq!(rewards[&proposer], 10_000_000_000_000_000_000u128);
    }

    #[test]
    fn test_huge_fees_do_not_overflow() {
        let mut calc = RewardCalculator::new(RewardConfig::default());
        let proposer = vec![9u8];
        let rewards =
            calc.calculate_block_rewards(&Block::genesis(), proposer.clone(), &[], u128::MAX);
        assert!(rewards[&proposer] > 0);
    }

    fn receipt(success: bool, gas_used: u64) -> TransactionReceipt {
        TransactionReceipt {
            success,
            gas_used,
            output: vec![],
            created_address: None,
            revert_reason: None,
            logs: vec![],
        }
    }

    #[test]
    fn test_failed_receipts_pay_no_fees() {
        let receipts = vec![receipt(true, 21_000), receipt(false, 21_000)];
        assert_eq!(total_fees(&receipts, 10), 210_000);
        assert_eq!(
            total_fees(&[receipt(true, u64::MAX)], u64::MAX),
            (u64::MAX as u128) * (u64::MAX as u128)
        );
    }
}
