use common::types::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// INFLATION & TOKENOMICS
// ============================================================================

/// Dynamic inflation schedule with halving mechanism
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InflationSchedule {
    /// Initial block reward in smallest unit
    pub initial_reward: u128,
    /// Number of blocks between halvings (~4 years at 6s blocks)
    pub halving_interval: u64,
    /// Fee burn percentage (0-100)
    pub fee_burn_percentage: u8,
}

impl InflationSchedule {
    pub fn new(initial_reward: u128, halving_interval: u64, fee_burn_percentage: u8) -> Self {
        Self {
            initial_reward,
            halving_interval,
            fee_burn_percentage,
        }
    }

    /// Calculate block reward at given height
    pub fn calculate_reward(&self, height: u64) -> u128 {
        let halvings = height / self.halving_interval;
        // Prevent overflow by capping halvings
        if halvings >= 64 {
            return 0;
        }
        self.initial_reward >> halvings
    }

    /// Calculate how much of the fee to burn
    pub fn calculate_fee_burn(&self, total_fee: u128) -> u128 {
        (total_fee * self.fee_burn_percentage as u128) / 100
    }
}

impl Default for InflationSchedule {
    fn default() -> Self {
        Self::new(
            10_000_000_000, // 10 tokens (assuming 9 decimals)
            2_100_000,      // ~4 years
            50,             // Burn 50% of fees
        )
    }
}

// ============================================================================
// STAKING & DELEGATION
// ============================================================================

/// Delegation from token holder to validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delegation {
    pub delegator: Address,
    pub validator: Address,
    pub amount: u128,
    pub rewards_earned: u128,
    pub created_at_height: u64,
}

/// Enhanced validator metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorMetadata {
    /// Commission rate (0-100 percentage)
    pub commission_rate: u8,
    /// Total amount delegated to this validator
    pub total_delegated: u128,
    /// Number of delegators
    pub delegator_count: u32,
    /// Blocks produced by this validator
    pub blocks_produced: u64,
    /// Blocks missed (for slashing)
    pub blocks_missed: u64,
    /// Whether validator is currently active
    pub is_active: bool,
    /// Total rewards earned
    pub total_rewards: u128,
}

impl Default for ValidatorMetadata {
    fn default() -> Self {
        Self {
            commission_rate: 10, // 10% default commission
            total_delegated: 0,
            delegator_count: 0,
            blocks_produced: 0,
            blocks_missed: 0,
            is_active: true,
            total_rewards: 0,
        }
    }
}

/// Unbonding request for delayed unstaking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbondingRequest {
    pub delegator: Address,
    pub validator: Address,
    pub amount: u128,
    pub completion_height: u64,
    pub created_at_height: u64,
}

// ============================================================================
// SLASHING
// ============================================================================

/// Types of slashable offenses
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SlashingReason {
    DoubleSign,
    Downtime,
    InvalidStateTransition,
}

/// Slashing event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingEvent {
    pub validator: Address,
    pub reason: SlashingReason,
    pub amount_slashed: u128,
    pub height: u64,
}

impl SlashingReason {
    /// Get the slash percentage for this offense
    pub fn slash_percentage(&self) -> u8 {
        match self {
            SlashingReason::DoubleSign => 5,              // 5%
            SlashingReason::Downtime => 0,                // 0.1% (handled separately)
            SlashingReason::InvalidStateTransition => 10, // 10%
        }
    }
}

// ============================================================================
// TREASURY
// ============================================================================

/// Treasury for protocol development funding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Treasury {
    pub balance: u128,
    pub total_collected: u128,
    pub total_spent: u128,
}

impl Treasury {
    pub fn new() -> Self {
        Self {
            balance: 0,
            total_collected: 0,
            total_spent: 0,
        }
    }

    /// Add funds to treasury
    pub fn deposit(&mut self, amount: u128) {
        self.balance += amount;
        self.total_collected += amount;
    }

    /// Spend from treasury (requires governance approval)
    pub fn spend(&mut self, amount: u128) -> Result<(), String> {
        if self.balance < amount {
            return Err("Insufficient treasury balance".into());
        }
        self.balance -= amount;
        self.total_spent += amount;
        Ok(())
    }
}

impl Default for Treasury {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    pub address: Address,
    pub public_key: Vec<u8>,
    pub stake: u64,
    pub is_active: bool,
    pub last_active_epoch: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingContract {
    validators: HashMap<Address, Validator>,
    validator_metadata: HashMap<Address, ValidatorMetadata>,
    delegations: Vec<Delegation>,
    unbonding_requests: Vec<UnbondingRequest>,
    slashing_events: Vec<SlashingEvent>,
    min_stake: u128,
    total_stake: u128,
    unbonding_period: u64, // blocks
    max_validators: usize,
    treasury: Treasury,
    inflation_schedule: InflationSchedule,
}

impl StakingContract {
    pub fn new(min_stake: u128) -> Self {
        Self {
            validators: HashMap::new(),
            validator_metadata: HashMap::new(),
            delegations: Vec::new(),
            unbonding_requests: Vec::new(),
            slashing_events: Vec::new(),
            min_stake,
            total_stake: 0,
            unbonding_period: 100_800, // ~7 days at 6s blocks
            max_validators: 100,
            treasury: Treasury::default(),
            inflation_schedule: InflationSchedule::default(),
        }
    }

    /// Register a new validator
    pub fn register_validator(&mut self, address: Address, public_key: Vec<u8>, stake: u128, commission_rate: u8) -> Result<(), String> {
        if stake < self.min_stake {
            return Err(format!("Stake {} is below minimum {}", stake, self.min_stake));
        }

        if commission_rate > 100 {
            return Err("Commission rate must be 0-100".into());
        }

        if self.validators.contains_key(&address) {
            return Err("Validator already registered".into());
        }

        if self.validators.len() >= self.max_validators {
            return Err(format!("Maximum validators ({}) reached", self.max_validators));
        }

        let validator = Validator {
            address,
            public_key,
            stake: stake as u64, // Convert for compatibility
            is_active: true,
            last_active_epoch: 0,
        };

        let metadata = ValidatorMetadata {
            commission_rate,
            ..Default::default()
        };

        self.validators.insert(address, validator);
        self.validator_metadata.insert(address, metadata);
        self.total_stake += stake;

        Ok(())
    }

    /// Delegate stake to a validator
    pub fn delegate(&mut self, delegator: Address, validator: Address, amount: u128, current_height: u64) -> Result<(), String> {
        if !self.validators.contains_key(&validator) {
            return Err("Validator not found".into());
        }

        let delegation = Delegation {
            delegator,
            validator,
            amount,
            rewards_earned: 0,
            created_at_height: current_height,
        };

        self.delegations.push(delegation);

        // Update validator metadata
        if let Some(metadata) = self.validator_metadata.get_mut(&validator) {
            metadata.total_delegated += amount;
            metadata.delegator_count += 1;
        }

        self.total_stake += amount;
        Ok(())
    }

    /// Request to undelegate (starts unbonding period)
    pub fn undelegate(&mut self, delegator: Address, validator: Address, amount: u128, current_height: u64) -> Result<(), String> {
        // Find and reduce delegation
        let mut found = false;
        let mut remaining_amount = amount;

        self.delegations.retain_mut(|d| {
            if d.delegator == delegator && d.validator == validator && remaining_amount > 0 {
                if d.amount >= remaining_amount {
                    d.amount -= remaining_amount;
                    remaining_amount = 0;
                    found = true;
                    d.amount > 0 // Keep if there's still stake
                } else {
                    remaining_amount -= d.amount;
                    found = true;
                    false // Remove this delegation
                }
            } else {
                true // Keep other delegations
            }
        });

        if !found || remaining_amount > 0 {
            return Err("Insufficient delegated amount".into());
        }

        // Create unbonding request
        let unbonding = UnbondingRequest {
            delegator,
            validator,
            amount,
            completion_height: current_height + self.unbonding_period,
            created_at_height: current_height,
        };

        self.unbonding_requests.push(unbonding);

        // Update validator metadata
        if let Some(metadata) = self.validator_metadata.get_mut(&validator) {
            metadata.total_delegated = metadata.total_delegated.saturating_sub(amount);
            metadata.delegator_count = metadata.delegator_count.saturating_sub(1);
        }

        self.total_stake = self.total_stake.saturating_sub(amount);
        Ok(())
    }

    /// Process completed unbonding requests
    pub fn process_unbonding(&mut self, current_height: u64) -> Vec<(Address, u128)> {
        let mut completed = Vec::new();

        self.unbonding_requests.retain(|req| {
            if current_height >= req.completion_height {
                completed.push((req.delegator, req.amount));
                false // Remove from queue
            } else {
                true // Keep in queue
            }
        });

        completed
    }

    /// Slash a validator for misbehavior
    pub fn slash(&mut self, validator: Address, reason: SlashingReason, current_height: u64) -> Result<u128, String> {
        let val = self.validators.get_mut(&validator).ok_or("Validator not found")?;
        
        let slash_percentage = if reason == SlashingReason::Downtime {
            // Special case: 0.1% for downtime
            1 // 0.1% = 1/1000
        } else {
            reason.slash_percentage() as u128 * 10 // Convert to per-1000
        };

        let validator_stake = val.stake as u128;
        let slashed_amount = if reason == SlashingReason::Downtime {
            validator_stake / 1000 // 0.1%
        } else {
            (validator_stake * slash_percentage) / 1000
        };

        val.stake = (validator_stake.saturating_sub(slashed_amount)) as u64;
        self.total_stake = self.total_stake.saturating_sub(slashed_amount);

        // Deactivate if below minimum
        if (val.stake as u128) < self.min_stake {
            val.is_active = false;
            if let Some(metadata) = self.validator_metadata.get_mut(&validator) {
                metadata.is_active = false;
            }
        }

        // Record slashing event
        let event = SlashingEvent {
            validator,
            reason,
            amount_slashed: slashed_amount,
            height: current_height,
        };
        self.slashing_events.push(event);

        // Send slashed funds to treasury
        self.treasury.deposit(slashed_amount);

        Ok(slashed_amount)
    }

    /// Distribute block rewards to validator and delegators
    pub fn distribute_rewards(&mut self, validator: Address, block_reward: u128, fees: u128, _current_height: u64) -> Result<(), String> {
        let metadata = self.validator_metadata.get_mut(&validator).ok_or("Validator not found")?;
        
        // Calculate total reward (block reward + fees after burn)
        let fee_burn = self.inflation_schedule.calculate_fee_burn(fees);
        let fee_to_distribute = fees - fee_burn;
        let total_reward = block_reward + fee_to_distribute;

        // Treasury gets 10% of block rewards
        let treasury_share = block_reward / 10;
        self.treasury.deposit(treasury_share);
        let remaining_reward = total_reward - treasury_share;

        // Get validator's self-stake and total delegated
        let validator_self_stake = self.validators.get(&validator).map(|v| v.stake as u128).unwrap_or(0);
        let total_delegated = metadata.total_delegated;
        let total_stake = validator_self_stake + total_delegated;

        if total_stake == 0 {
            return Ok(()); // No stake, no rewards
        }

        // Calculate validator's share (self-stake + commission on delegated rewards)
        let validator_stake_reward = (remaining_reward * validator_self_stake) / total_stake;
        let delegated_reward = (remaining_reward * total_delegated) / total_stake;
        let commission = (delegated_reward * metadata.commission_rate as u128) / 100;
        let validator_total_reward = validator_stake_reward + commission;

        // Update validator metadata
        metadata.total_rewards += validator_total_reward;
        metadata.blocks_produced += 1;

        // Distribute to delegators (proportionally, minus commission)
        let delegator_reward_pool = delegated_reward - commission;
        for delegation in self.delegations.iter_mut() {
            if delegation.validator == validator {
                let delegator_share = (delegator_reward_pool * delegation.amount) / total_delegated;
                delegation.rewards_earned += delegator_share;
            }
        }

        Ok(())
    }

    /// Record a missed block (for downtime slashing)
    pub fn record_missed_block(&mut self, validator: Address) -> Result<(), String> {
        // Check if we need to slash before borrowing
        let should_slash = {
            let metadata = self.validator_metadata.get(&validator).ok_or("Validator not found")?;
            metadata.blocks_missed + 1 >= 100
        };

        // Update blocks_missed
        if let Some(metadata) = self.validator_metadata.get_mut(&validator) {
            metadata.blocks_missed += 1;
        }

        // Slash if threshold reached
        if should_slash {
            self.slash(validator, SlashingReason::Downtime, 0)?;
            if let Some(metadata) = self.validator_metadata.get_mut(&validator) {
                metadata.blocks_missed = 0; // Reset counter
            }
        }

        Ok(())
    }

    /// Get active validators sorted by total stake (self + delegated)
    pub fn get_active_validators(&self) -> Vec<(Validator, ValidatorMetadata)> {
        let mut active: Vec<(Validator, ValidatorMetadata)> = self.validators
            .iter()
            .filter_map(|(addr, val)| {
                if val.is_active {
                    self.validator_metadata.get(addr).map(|meta| (val.clone(), meta.clone()))
                } else {
                    None
                }
            })
            .collect();
        
        // Sort by total stake (self + delegated) descending
        active.sort_by(|a, b| {
            let a_total = a.0.stake as u128 + a.1.total_delegated;
            let b_total = b.0.stake as u128 + b.1.total_delegated;
            b_total.cmp(&a_total)
        });
        
        active
    }

    /// Get total staked amount
    pub fn total_stake(&self) -> u128 {
        self.total_stake
    }

    /// Get delegations for a specific delegator
    pub fn get_delegations(&self, delegator: &Address) -> Vec<&Delegation> {
        self.delegations.iter().filter(|d| d.delegator == *delegator).collect()
    }

    /// Get treasury balance
    pub fn treasury_balance(&self) -> u128 {
        self.treasury.balance
    }

    /// Calculate block reward for current height
    pub fn calculate_block_reward(&self, height: u64) -> u128 {
        self.inflation_schedule.calculate_reward(height)
    }
}


// Governance Structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalType {
    ParameterChange { key: String, value: String },
    SoftwareUpgrade { version: String, hash: String },
    TextProposal { title: String, description: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub proposal_type: ProposalType,
    pub start_epoch: u64,
    pub end_epoch: u64,
    pub yes_votes: u64,
    pub no_votes: u64,
    pub status: ProposalStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Executed,
}

pub struct Governance {
    proposals: HashMap<u64, Proposal>,
    votes: HashMap<u64, HashMap<Address, bool>>, // ProposalID -> Voter -> Yes/No
    next_proposal_id: u64,
    staking: StakingContract,
}

impl Governance {
    pub fn new(staking: StakingContract) -> Self {
        Self {
            proposals: HashMap::new(),
            votes: HashMap::new(),
            next_proposal_id: 1,
            staking,
        }
    }

    pub fn create_proposal(
        &mut self,
        proposer: Address,
        proposal_type: ProposalType,
        current_epoch: u64,
        duration: u64,
    ) -> Result<u64, String> {
        // Check if proposer is a validator (simplified check)
        if !self.staking.validators.contains_key(&proposer) {
            return Err("Only validators can propose".into());
        }

        let id = self.next_proposal_id;
        self.next_proposal_id += 1;

        let proposal = Proposal {
            id,
            proposer,
            proposal_type,
            start_epoch: current_epoch,
            end_epoch: current_epoch + duration,
            yes_votes: 0,
            no_votes: 0,
            status: ProposalStatus::Active,
        };

        self.proposals.insert(id, proposal);
        Ok(id)
    }

    pub fn vote(&mut self, proposal_id: u64, voter: Address, vote: bool) -> Result<(), String> {
        let proposal = self.proposals.get_mut(&proposal_id).ok_or("Proposal not found")?;

        if proposal.status != ProposalStatus::Active {
            return Err("Proposal is not active".into());
        }

        // Check if voter is a validator
        let validator = self.staking.validators.get(&voter).ok_or("Only validators can vote")?;
        let voting_power = validator.stake;

        // Record vote
        let proposal_votes = self.votes.entry(proposal_id).or_default();
        if proposal_votes.contains_key(&voter) {
            return Err("Already voted".into());
        }

        proposal_votes.insert(voter, vote);

        if vote {
            proposal.yes_votes += voting_power;
        } else {
            proposal.no_votes += voting_power;
        }

        Ok(())
    }

    pub fn tally_votes(&mut self, proposal_id: u64) -> Result<ProposalStatus, String> {
        let proposal = self.proposals.get_mut(&proposal_id).ok_or("Proposal not found")?;

        // Simple majority check
        // In real system, check quorum and threshold
        let total_votes = proposal.yes_votes + proposal.no_votes;
        if total_votes == 0 {
            return Ok(ProposalStatus::Active); // No votes yet
        }

        if proposal.yes_votes > proposal.no_votes {
            proposal.status = ProposalStatus::Passed;
        } else {
            proposal.status = ProposalStatus::Rejected;
        }

        Ok(proposal.status.clone())
    }
}
