//! # Genesis Validation Module
//!
//! Comprehensive validation of genesis configuration before generation.

use crate::config::GenesisConfig;
use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::str::FromStr;

pub struct Validator;

impl Validator {
    /// Validate complete genesis configuration
    pub fn validate_config(config: &GenesisConfig) -> Result<()> {
        Self::validate_chain(&config.chain)?;
        Self::validate_consensus(&config.consensus)?;
        Self::validate_governance(&config.governance)?;
        Self::validate_economic(&config.economic)?;
        Self::validate_validators(&config.validators, &config.consensus.min_stake)?;
        Self::validate_accounts(&config.accounts)?;
        Self::validate_precompiles(&config.precompiles)?;
        Self::validate_cross_references(config)?;
        Ok(())
    }

    fn validate_chain(chain: &crate::config::ChainConfig) -> Result<()> {
        if chain.chain_id.is_empty() {
            return Err(anyhow!("Chain ID cannot be empty"));
        }

        if !chain
            .chain_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(anyhow!(
                "Chain ID must be alphanumeric with hyphens/underscores"
            ));
        }

        if chain.timestamp == 0 {
            return Err(anyhow!("Timestamp must be greater than 0"));
        }

        if chain.network_id == 0 {
            return Err(anyhow!("Network ID must be greater than 0"));
        }

        Ok(())
    }

    fn validate_consensus(consensus: &crate::config::ConsensusConfig) -> Result<()> {
        if consensus.block_time_ms < 500 {
            return Err(anyhow!("Block time must be at least 500ms"));
        }

        if consensus.max_validators == 0 {
            return Err(anyhow!("Max validators must be greater than 0"));
        }

        if consensus.max_validators > 1000 {
            return Err(anyhow!("Max validators cannot exceed 1000"));
        }

        let min_stake: u128 = consensus
            .min_stake
            .parse()
            .map_err(|_| anyhow!("Invalid min_stake format: {}", consensus.min_stake))?;

        if min_stake == 0 {
            return Err(anyhow!("Min stake must be greater than 0"));
        }

        if consensus.unbonding_period < 100 {
            return Err(anyhow!("Unbonding period must be at least 100 blocks"));
        }

        Ok(())
    }

    fn validate_governance(governance: &crate::config::GovernanceConfig) -> Result<()> {
        let proposal_deposit: u128 = governance.proposal_deposit.parse().map_err(|_| {
            anyhow!(
                "Invalid proposal_deposit format: {}",
                governance.proposal_deposit
            )
        })?;

        if proposal_deposit == 0 {
            return Err(anyhow!("Proposal deposit must be greater than 0"));
        }

        if governance.voting_period_blocks < 100 {
            return Err(anyhow!("Voting period must be at least 100 blocks"));
        }

        let quorum: f64 = governance.quorum_threshold.parse().map_err(|_| {
            anyhow!(
                "Invalid quorum threshold format: {}",
                governance.quorum_threshold
            )
        })?;

        if !(0.0..=1.0).contains(&quorum) {
            return Err(anyhow!("Quorum threshold must be between 0.0 and 1.0"));
        }

        let approval: f64 = governance.approval_threshold.parse().map_err(|_| {
            anyhow!(
                "Invalid approval threshold format: {}",
                governance.approval_threshold
            )
        })?;

        if !(0.0..=1.0).contains(&approval) {
            return Err(anyhow!("Approval threshold must be between 0.0 and 1.0"));
        }

        if approval <= quorum {
            return Err(anyhow!(
                "Approval threshold ({}) must be greater than quorum threshold ({})",
                approval,
                quorum
            ));
        }

        Ok(())
    }

    fn validate_economic(economic: &crate::config::EconomicConfig) -> Result<()> {
        let initial_reward: u128 = economic.initial_block_reward.parse().map_err(|_| {
            anyhow!(
                "Invalid initial_block_reward format: {}",
                economic.initial_block_reward
            )
        })?;

        if initial_reward == 0 {
            return Err(anyhow!("Initial block reward must be greater than 0"));
        }

        if economic.halving_interval == 0 {
            return Err(anyhow!("Halving interval must be greater than 0"));
        }

        if economic.fee_burn_percentage > 100 {
            return Err(anyhow!("Fee burn percentage must be between 0 and 100"));
        }

        // Validate treasury address format
        let treasury = economic.treasury_address.trim_start_matches("0x");
        if treasury.len() != 40 && treasury.len() != 0 {
            return Err(anyhow!("Treasury address must be 20 bytes (40 hex chars)"));
        }

        Ok(())
    }

    fn validate_validators(
        validators: &[crate::config::ValidatorConfig],
        min_stake_str: &str,
    ) -> Result<()> {
        if validators.is_empty() {
            return Err(anyhow!("At least one validator is required"));
        }

        let min_stake: u128 = min_stake_str
            .parse()
            .map_err(|_| anyhow!("Invalid min_stake format: {}", min_stake_str))?;

        let mut addresses = HashSet::new();
        let mut public_keys = HashSet::new();
        let mut total_stake: u128 = 0;

        for validator in validators {
            // Check unique addresses
            if !addresses.insert(&validator.address) {
                return Err(anyhow!(
                    "Duplicate validator address: {}",
                    validator.address
                ));
            }

            // Check unique public keys
            if !public_keys.insert(&validator.public_key) {
                return Err(anyhow!(
                    "Duplicate validator public key: {}",
                    validator.public_key
                ));
            }

            // Validate address format
            let addr_clean = validator.address.trim_start_matches("0x");
            if addr_clean.len() != 40 {
                return Err(anyhow!(
                    "Invalid address format for {}: must be 20 bytes (40 hex chars)",
                    validator.address
                ));
            }

            if !addr_clean.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(anyhow!("Invalid address hex format: {}", validator.address));
            }

            // Parse stake
            let stake: u128 = validator.stake.parse().map_err(|_| {
                anyhow!(
                    "Invalid stake format for {}: {}",
                    validator.address,
                    validator.stake
                )
            })?;

            // Check min stake
            if stake < min_stake {
                return Err(anyhow!(
                    "Validator {} stake ({}) is below minimum ({})",
                    validator.address,
                    stake,
                    min_stake
                ));
            }

            // Validate commission rate
            let commission: f64 = validator.commission_rate.parse().map_err(|_| {
                anyhow!(
                    "Invalid commission rate for {}: {}",
                    validator.address,
                    validator.commission_rate
                )
            })?;

            if !(0.0..=1.0).contains(&commission) {
                return Err(anyhow!(
                    "Commission rate must be between 0.0 and 1.0 for {}",
                    validator.address
                ));
            }

            // Validate public key format
            let pk_clean = validator.public_key.trim_start_matches("0x");
            if pk_clean.len() != 64 && pk_clean.len() != 66 {
                return Err(anyhow!(
                    "Invalid public key format for {}: must be 32 or 33 bytes",
                    validator.address
                ));
            }

            total_stake = total_stake
                .checked_add(stake)
                .ok_or_else(|| anyhow!("Total stake overflow"))?;
        }

        if total_stake == 0 {
            return Err(anyhow!("Total validator stake must be greater than 0"));
        }

        Ok(())
    }

    fn validate_accounts(accounts: &[crate::config::AccountConfig]) -> Result<()> {
        if accounts.is_empty() {
            return Err(anyhow!("At least one account is required"));
        }

        let mut addresses = HashSet::new();
        let mut total_supply: u128 = 0;

        for account in accounts {
            // Check unique addresses
            if !addresses.insert(&account.address) {
                return Err(anyhow!("Duplicate account address: {}", account.address));
            }

            // Validate address format
            let addr_clean = account.address.trim_start_matches("0x");
            if addr_clean.len() != 40 {
                return Err(anyhow!(
                    "Invalid address format for {}: must be 20 bytes (40 hex chars)",
                    account.address
                ));
            }

            if !addr_clean.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(anyhow!("Invalid address hex format: {}", account.address));
            }

            // Parse balance
            let balance: u128 = account.balance.parse().map_err(|_| {
                anyhow!(
                    "Invalid balance format for {}: {}",
                    account.address,
                    account.balance
                )
            })?;

            if balance == 0 {
                return Err(anyhow!(
                    "Account {} balance must be greater than 0",
                    account.address
                ));
            }

            // Validate code if present
            if let Some(code) = &account.code {
                let code_clean = code.trim_start_matches("0x");
                if code_clean.is_empty() {
                    return Err(anyhow!("Account {} has empty code", account.address));
                }

                if !code_clean.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(anyhow!("Account {} has invalid code hex", account.address));
                }
            }

            total_supply = total_supply
                .checked_add(balance)
                .ok_or_else(|| anyhow!("Total supply overflow"))?;
        }

        if total_supply == 0 {
            return Err(anyhow!("Total supply must be greater than 0"));
        }

        Ok(())
    }

    fn validate_precompiles(precompiles: &[crate::config::PrecompileConfig]) -> Result<()> {
        let valid_types = vec!["ecrecover", "sha256", "ripemd160", "identity", "blake2f"];

        for precompile in precompiles {
            // Validate address
            let addr_clean = precompile.address.trim_start_matches("0x");
            if addr_clean.len() != 40 {
                return Err(anyhow!(
                    "Invalid precompile address: {}",
                    precompile.address
                ));
            }

            // Validate contract type
            if !valid_types.contains(&precompile.contract_type.as_str()) {
                return Err(anyhow!(
                    "Invalid precompile type: {}. Valid: {:?}",
                    precompile.contract_type,
                    valid_types
                ));
            }
        }

        Ok(())
    }

    fn validate_cross_references(config: &GenesisConfig) -> Result<()> {
        // Check that validator addresses are unique from account addresses
        let validator_addresses: HashSet<_> =
            config.validators.iter().map(|v| &v.address).collect();

        for account in &config.accounts {
            if validator_addresses.contains(&account.address) {
                return Err(anyhow!("Account address {} is also a validator. Validators must have separate accounts.", account.address));
            }
        }

        // Check that treasury address is a valid account if not zero
        let treasury = config.economic.treasury_address.trim_start_matches("0x");
        if treasury != "0000000000000000000000000000000000000000" {
            let treasury_addr = format!("0x{}", treasury);
            if !config.accounts.iter().any(|a| a.address == treasury_addr) {
                return Err(anyhow!(
                    "Treasury address {} not found in accounts",
                    treasury_addr
                ));
            }
        }

        Ok(())
    }

    /// Print validation summary
    pub fn print_summary(config: &GenesisConfig) -> Result<()> {
        println!("\n📋 Genesis Configuration Summary");
        println!("=================================");
        println!("Chain ID:           {}", config.chain.chain_id);
        println!("Network ID:         {}", config.chain.network_id);
        println!("Block time:         {} ms", config.consensus.block_time_ms);
        println!("Max validators:     {}", config.consensus.max_validators);
        println!("Min stake:          {}", config.consensus.min_stake);
        println!("Validators:         {}", config.validators.len());
        println!("Accounts:           {}", config.accounts.len());
        println!("Precompiles:        {}", config.precompiles.len());

        let total_stake: u128 = config
            .validators
            .iter()
            .filter_map(|v| v.stake.parse::<u128>().ok())
            .sum();
        let total_supply: u128 = config
            .accounts
            .iter()
            .filter_map(|a| a.balance.parse::<u128>().ok())
            .sum();

        println!("Total stake:        {}", total_stake);
        println!("Total supply:       {}", total_supply);
        println!(
            "Initial reward:     {}",
            config.economic.initial_block_reward
        );
        println!("Halving interval:   {}", config.economic.halving_interval);
        println!(
            "Fee burn:           {}%",
            config.economic.fee_burn_percentage
        );

        Ok(())
    }
}
