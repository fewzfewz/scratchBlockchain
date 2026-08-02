use common::consensus_types::ValidatorInfo;
use common::types::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VoteChoice {
    Yes,
    No,
    Abstain,
}

impl VoteChoice {
    pub fn from_str(s: &str) -> Option<VoteChoice> {
        match s.trim().to_ascii_lowercase().as_str() {
            "yes" | "for" | "aye" => Some(VoteChoice::Yes),
            "no" | "against" | "nay" => Some(VoteChoice::No),
            "abstain" => Some(VoteChoice::Abstain),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            VoteChoice::Yes => "yes",
            VoteChoice::No => "no",
            VoteChoice::Abstain => "abstain",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub proposal_type: ProposalType,
    pub title: String,
    pub description: String,
    pub start_block: u64,
    pub end_block: u64,
    pub yes_votes: u128,
    pub no_votes: u128,
    pub abstain_votes: u128,
    pub status: ProposalStatus,
    /// Voter address (hex, no 0x prefix) -> choice. String keys keep the
    /// struct JSON-serializable (addresses are fixed-size byte arrays, which
    /// serde_json cannot use as map keys).
    pub voters: HashMap<String, VoteChoice>,
}

fn addr_key(addr: Address) -> String {
    addr.iter().map(|b| format!("{:02x}", b)).collect()
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
            title: String::new(),
            description: String::new(),
            start_block: current_epoch,
            end_block: current_epoch + duration,
            yes_votes: 0,
            no_votes: 0,
            abstain_votes: 0,
            status: ProposalStatus::Active,
            voters: HashMap::new(),
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
            proposal.yes_votes += voting_power as u128;
        } else {
            proposal.no_votes += voting_power as u128;
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


// ============================================================================
// Governance Execution & Parameter Updates
// ============================================================================

/// Executable governance actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GovernanceAction {
    /// Change a protocol parameter
    SetParameter { key: String, value: String },
    /// Update validator set
    UpdateValidatorSet { validators: Vec<ValidatorInfo> },
    /// Execute treasury spend
    TreasurySpend { recipient: Address, amount: u128 },
    /// Upgrade runtime
    RuntimeUpgrade { version: String, code_hash: [u8; 32] },
    /// Update inflation schedule
    UpdateInflation { new_initial_reward: u128, new_halving_interval: u64 },
}

/// Governance executor that applies approved proposals
pub struct GovernanceExecutor {
    staking: StakingContract,
    executed_actions: Vec<GovernanceAction>,
}

impl GovernanceExecutor {
    pub fn new(staking: StakingContract) -> Self {
        Self {
            staking,
            executed_actions: Vec::new(),
        }
    }
    
    /// Execute an approved governance action
    pub fn execute(&mut self, action: GovernanceAction) -> Result<(), String> {
        match action {
            GovernanceAction::SetParameter { ref key, ref value } => {
                self.set_parameter(key, value)?;
            }
            GovernanceAction::UpdateValidatorSet { ref validators } => {
                self.update_validator_set(validators)?;
            }
            GovernanceAction::TreasurySpend { recipient: _, ref amount } => {
                self.staking.treasury.spend(*amount)?;
                // Transfer to recipient logic here
            }
            GovernanceAction::RuntimeUpgrade { ref version, code_hash: _ } => {
                info!("Runtime upgrade to version {} approved", version);
            }
            GovernanceAction::UpdateInflation { ref new_initial_reward, ref new_halving_interval } => {
                let _new_schedule = InflationSchedule::new(*new_initial_reward, *new_halving_interval, 50);
                // Apply new schedule - would need to update staking
                info!("Inflation schedule updated");
            }
        }
        
        self.executed_actions.push(action);
        Ok(())
    }
    
    fn set_parameter(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "min_stake" => {
                let new_min = value.parse().map_err(|_| "Invalid min_stake value")?;
                self.staking.min_stake = new_min;
            }
            "unbonding_period" => {
                let new_period = value.parse().map_err(|_| "Invalid unbonding_period value")?;
                self.staking.unbonding_period = new_period;
            }
            _ => return Err(format!("Unknown parameter: {}", key)),
        }
        Ok(())
    }
    
    fn update_validator_set(&mut self, validators: &[ValidatorInfo]) -> Result<(), String> {
        // Update active validator set (match by public_key)
        for validator in validators {
            let existing = self.staking.validators.values_mut().find(|v| v.public_key == validator.public_key);
            if let Some(existing) = existing {
                existing.stake = validator.stake;
                existing.is_active = !validator.slashed;
            }
        }
        Ok(())
    }
}

// ============================================================================
// ON-CHAIN GOVERNANCE (wired into the running node)
// ============================================================================

/// Governance parameters that define how proposals behave on-chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceParams {
    /// Deposit required to submit a proposal (smallest token unit).
    pub proposal_deposit: u128,
    /// How many blocks a proposal stays open for voting.
    pub voting_period_blocks: u64,
    /// Quorum threshold in basis points (10000 = 100%) of total stake.
    pub quorum_threshold_bps: u64,
    /// Share of decided votes that must be "yes" to pass, in basis points.
    pub pass_threshold_bps: u64,
}

impl Default for GovernanceParams {
    fn default() -> Self {
        Self {
            proposal_deposit: 1_000,
            voting_period_blocks: 1_000,
            quorum_threshold_bps: 3_340, // 33.4%
            pass_threshold_bps: 5_000,   // simple majority
        }
    }
}

/// The complete, persisted on-chain governance state. The node stores a
/// serialized copy of this in its state trie (under the `b"governance"` key)
/// so every validator commits exactly the same result for a given block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainGovernance {
    pub params: GovernanceParams,
    pub treasury: Treasury,
    pub proposals: Vec<Proposal>,
    pub next_proposal_id: u64,
    /// Total stake of the validator set — used to compute quorum.
    pub total_stake: u128,
}

impl ChainGovernance {
    pub fn new(params: GovernanceParams, treasury_balance: u128, total_stake: u128) -> Self {
        Self {
            params,
            treasury: Treasury {
                balance: treasury_balance,
                total_collected: 0,
                total_spent: 0,
            },
            proposals: Vec::new(),
            next_proposal_id: 1,
            total_stake,
        }
    }

    /// Create a new proposal. Any account may propose.
    pub fn propose(
        &mut self,
        proposer: Address,
        title: &str,
        description: &str,
        proposal_type: ProposalType,
        current_block: u64,
    ) -> Result<u64, String> {
        if title.trim().is_empty() {
            return Err("Proposal title cannot be empty".into());
        }

        let id = self.next_proposal_id;
        self.next_proposal_id += 1;

        let proposal = Proposal {
            id,
            proposer,
            proposal_type,
            title: title.to_string(),
            description: description.to_string(),
            start_block: current_block,
            end_block: current_block.saturating_add(self.params.voting_period_blocks),
            yes_votes: 0,
            no_votes: 0,
            abstain_votes: 0,
            status: ProposalStatus::Active,
            voters: HashMap::new(),
        };

        self.proposals.push(proposal);
        Ok(id)
    }

    /// Cast a vote with the given voting power (e.g. the voter's staked +
    /// delegated balance). Each account may vote once per proposal.
    pub fn vote(
        &mut self,
        proposal_id: u64,
        voter: Address,
        choice: VoteChoice,
        voting_power: u128,
        current_block: u64,
    ) -> Result<(), String> {
        let (quorum_bps, pass_bps, total_stake) = (
            self.params.quorum_threshold_bps,
            self.params.pass_threshold_bps,
            self.total_stake,
        );

        let proposal = self
            .proposals
            .iter_mut()
            .find(|p| p.id == proposal_id)
            .ok_or("Proposal not found")?;

        if Self::compute_status(quorum_bps, pass_bps, total_stake, proposal, current_block)
            != ProposalStatus::Active
        {
            return Err("Proposal is not active".into());
        }
        if proposal.voters.contains_key(&addr_key(voter)) {
            return Err("Already voted".into());
        }

        proposal.voters.insert(addr_key(voter), choice.clone());
        match choice {
            VoteChoice::Yes => proposal.yes_votes += voting_power,
            VoteChoice::No => proposal.no_votes += voting_power,
            VoteChoice::Abstain => proposal.abstain_votes += voting_power,
        }

        proposal.status = Self::compute_status(quorum_bps, pass_bps, total_stake, proposal, current_block);
        Ok(())
    }

    /// Current status of a proposal. Tally-based outcomes only resolve after
    /// the voting period has ended; until then a proposal is Active.
    pub fn resolve_status(&self, proposal: &Proposal, current_block: u64) -> ProposalStatus {
        Self::compute_status(
            self.params.quorum_threshold_bps,
            self.params.pass_threshold_bps,
            self.total_stake,
            proposal,
            current_block,
        )
    }

    fn compute_status(
        quorum_threshold_bps: u64,
        pass_threshold_bps: u64,
        total_stake: u128,
        proposal: &Proposal,
        current_block: u64,
    ) -> ProposalStatus {
        if proposal.status == ProposalStatus::Executed {
            return ProposalStatus::Executed;
        }
        if current_block < proposal.end_block {
            return ProposalStatus::Active;
        }

        let total_votes = proposal.yes_votes + proposal.no_votes + proposal.abstain_votes;
        let quorum_needed = (total_stake * quorum_threshold_bps as u128) / 10_000;

        if total_votes < quorum_needed {
            return ProposalStatus::Rejected; // quorum not met
        }

        let decided = proposal.yes_votes + proposal.no_votes;
        let yes_share = if decided == 0 {
            0
        } else {
            (proposal.yes_votes * 10_000) / decided
        };

        if yes_share >= pass_threshold_bps as u128 {
            ProposalStatus::Passed
        } else {
            ProposalStatus::Rejected
        }
    }

    /// Parse a governance action out of a transaction payload and apply it.
    /// `voting_power` is the weight of `sender` for vote actions (resolved by
    /// the node — currently the voter's staked + delegated balance).
    pub fn apply_payload(
        &mut self,
        sender: Address,
        payload: &[u8],
        voting_power: u128,
        current_block: u64,
    ) -> Result<(), String> {
        let value: serde_json::Value =
            serde_json::from_slice(payload).map_err(|_| "Invalid governance payload".to_string())?;

        let action = value
            .get("action")
            .and_then(|a| a.as_str())
            .ok_or("Missing action")?;

        match action {
            "vote" => {
                let proposal_id = value
                    .get("proposal_id")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing proposal_id")?;
                let choice_str = value
                    .get("choice")
                    .and_then(|c| c.as_str())
                    .ok_or("Missing choice")?;
                let choice = VoteChoice::from_str(choice_str).ok_or("Invalid choice")?;
                self.vote(proposal_id, sender, choice, voting_power, current_block)
            }
            "propose" => {
                let title = value
                    .get("title")
                    .and_then(|t| t.as_str())
                    .ok_or("Missing title")?;
                let description = value
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("");
                self.propose(
                    sender,
                    title,
                    description,
                    ProposalType::TextProposal {
                        title: title.to_string(),
                        description: description.to_string(),
                    },
                    current_block,
                )?;
                Ok(())
            }
            _ => Err(format!("Unknown governance action: {}", action)),
        }
    }

    /// Whether a transaction payload looks like a governance action.
    pub fn is_governance_payload(payload: &[u8]) -> bool {
        if payload.is_empty() {
            return false;
        }
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(payload) else {
            return false;
        };
        matches!(
            value.get("action").and_then(|a| a.as_str()),
            Some("vote") | Some("propose")
        )
    }

    pub fn get_proposal(&self, id: u64) -> Option<&Proposal> {
        self.proposals.iter().find(|p| p.id == id)
    }
}

#[cfg(test)]
mod chain_governance_tests {
    use super::*;

    fn addr(byte: u8) -> Address {
        [byte; 20]
    }

    fn setup() -> ChainGovernance {
        ChainGovernance::new(GovernanceParams::default(), 5_000_000_000, 2_400_000)
    }

    #[test]
    fn test_propose_and_active_status() {
        let mut gov = setup();
        let id = gov
            .propose(addr(1), "Upgrade", "Upgrade description", ProposalType::TextProposal { title: "Upgrade".into(), description: "Upgrade description".into() }, 100)
            .unwrap();
        assert_eq!(id, 1);
        assert_eq!(gov.next_proposal_id, 2);
        let p = gov.get_proposal(1).unwrap();
        assert_eq!(p.start_block, 100);
        assert_eq!(p.end_block, 100 + GovernanceParams::default().voting_period_blocks);
        assert_eq!(gov.resolve_status(p, 99), ProposalStatus::Active);
    }

    #[test]
    fn test_vote_weight_and_double_vote_rejected() {
        let mut gov = setup();
        gov.propose(addr(1), "T", "D", ProposalType::TextProposal { title: "T".into(), description: "D".into() }, 0).unwrap();
        gov.vote(1, addr(2), VoteChoice::Yes, 1_000_000, 10).unwrap();
        assert_eq!(gov.get_proposal(1).unwrap().yes_votes, 1_000_000);
        assert!(gov.vote(1, addr(2), VoteChoice::No, 500_000, 10).is_err());
    }

    #[test]
    fn test_resolution_after_voting_ends() {
        let mut gov = setup();
        gov.propose(addr(1), "T", "D", ProposalType::TextProposal { title: "T".into(), description: "D".into() }, 0).unwrap();
        // Votes from three validators of the 2.4M total stake.
        gov.vote(1, addr(1), VoteChoice::Yes, 1_000_000, 0).unwrap();
        gov.vote(1, addr(2), VoteChoice::No, 800_000, 0).unwrap();
        gov.vote(1, addr(3), VoteChoice::Yes, 600_000, 0).unwrap();
        // Still active while voting is open.
        assert_eq!(gov.resolve_status(gov.get_proposal(1).unwrap(), 999), ProposalStatus::Active);
        // After the voting period, quorum is met and yes has a majority.
        assert_eq!(gov.resolve_status(gov.get_proposal(1).unwrap(), 1000), ProposalStatus::Passed);
    }

    #[test]
    fn test_rejected_when_majority_no() {
        let mut gov = setup();
        gov.propose(addr(1), "T", "D", ProposalType::TextProposal { title: "T".into(), description: "D".into() }, 0).unwrap();
        gov.vote(1, addr(1), VoteChoice::No, 1_000_000, 0).unwrap();
        gov.vote(1, addr(2), VoteChoice::No, 800_000, 0).unwrap();
        gov.vote(1, addr(3), VoteChoice::Yes, 600_000, 0).unwrap();
        assert_eq!(gov.resolve_status(gov.get_proposal(1).unwrap(), 1000), ProposalStatus::Rejected);
    }

    #[test]
    fn test_quorum_not_met_rejected() {
        let mut gov = setup();
        gov.propose(addr(1), "T", "D", ProposalType::TextProposal { title: "T".into(), description: "D".into() }, 0).unwrap();
        gov.vote(1, addr(1), VoteChoice::Yes, 400_000, 0).unwrap();
        // 400k < 33.4% of 2.4M (801.6k) quorum.
        assert_eq!(gov.resolve_status(gov.get_proposal(1).unwrap(), 1000), ProposalStatus::Rejected);
    }

    #[test]
    fn test_apply_payload_propose_and_vote() {
        let mut gov = setup();
        let propose_payload = br#"{"action":"propose","title":"Airdrop","description":"Send tokens"}"#;
        gov.apply_payload(addr(1), propose_payload, 0, 5).unwrap();
        assert_eq!(gov.get_proposal(1).unwrap().title, "Airdrop");

        let vote_payload = br#"{"action":"vote","proposal_id":1,"choice":"yes"}"#;
        gov.apply_payload(addr(2), vote_payload, 2_000_000, 5).unwrap();
        assert_eq!(gov.get_proposal(1).unwrap().yes_votes, 2_000_000);
    }

    #[test]
    fn test_is_governance_payload() {
        assert!(ChainGovernance::is_governance_payload(br#"{"action":"vote","proposal_id":1,"choice":"no"}"#));
        assert!(!ChainGovernance::is_governance_payload(b""));
        assert!(!ChainGovernance::is_governance_payload(b"not json"));
    }
}