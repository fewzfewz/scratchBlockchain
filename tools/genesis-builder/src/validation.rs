use crate::config::GenesisConfig;
use anyhow::{anyhow, Result};
use std::collections::HashSet;

pub struct Validator;

impl Validator {
    pub fn validate_config(config: &GenesisConfig) -> Result<()> {
        Self::validate_chain(&config.chain)?;
        Self::validate_consensus(&config.consensus)?;
        Self::validate_governance(&config.governance)?;
        Self::validate_validators(&config.validators, &config.consensus.min_stake)?;
        Self::validate_accounts(&config.accounts)?;
        Ok(())
    }

    fn validate_chain(chain: &crate::config::ChainConfig) -> Result<()> {
        if chain.chain_id.is_empty() {
            return Err(anyhow!("Chain ID cannot be empty"));
        }
        
        if !chain.chain_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(anyhow!("Chain ID must be alphanumeric with hyphens/underscores"));
        }
        
        if chain.timestamp == 0 {
            return Err(anyhow!("Timestamp must be greater than 0"));
        }
        
        Ok(())
    }

    fn validate_consensus(consensus: &crate::config::ConsensusConfig) -> Result<()> {
        if consensus.block_time_ms == 0 {
            return Err(anyhow!("Block time must be greater than 0"));
        }
        
        if consensus.max_validators == 0 {
            return Err(anyhow!("Max validators must be greater than 0"));
        }
        
        let min_stake: u128 = consensus.min_stake.parse()
            .map_err(|_| anyhow!("Invalid min_stake format"))?;
        
        if min_stake == 0 {
            return Err(anyhow!("Min stake must be greater than 0"));
        }
        
        Ok(())
    }

    fn validate_governance(governance: &crate::config::GovernanceConfig) -> Result<()> {
        let proposal_deposit: u128 = governance.proposal_deposit.parse()
            .map_err(|_| anyhow!("Invalid proposal_deposit format"))?;
        
        if proposal_deposit == 0 {
            return Err(anyhow!("Proposal deposit must be greater than 0"));
        }
        
        if governance.voting_period_blocks == 0 {
            return Err(anyhow!("Voting period must be greater than 0"));
        }
        
        let quorum: f64 = governance.quorum_threshold.parse()
            .map_err(|_| anyhow!("Invalid quorum threshold format"))?;
        
        if !(0.0..=1.0).contains(&quorum) {
            return Err(anyhow!("Quorum threshold must be between 0.0 and 1.0"));
        }
        
        Ok(())
    }

    fn validate_validators(validators: &[crate::config::ValidatorConfig], min_stake_str: &str) -> Result<()> {
        if validators.is_empty() {
            return Err(anyhow!("At least one validator is required"));
        }
        
        let min_stake: u128 = min_stake_str.parse()
            .map_err(|_| anyhow!("Invalid min_stake format"))?;
        
        let mut addresses = HashSet::new();
        let mut total_stake: u128 = 0;
        
        for validator in validators {
            // Check unique addresses
            if !addresses.insert(&validator.address) {
                return Err(anyhow!("Duplicate validator address: {}", validator.address));
            }
            
            // Validate address format (basic check)
            if !validator.address.starts_with("0x") || validator.address.len() != 42 {
                return Err(anyhow!("Invalid address format: {}", validator.address));
            }
            
            // Parse stake
            let stake: u128 = validator.stake.parse()
                .map_err(|_| anyhow!("Invalid stake for {}", validator.address))?;
            
            // Check min stake
            if stake < min_stake {
                return Err(anyhow!(
                    "Validator {} stake ({}) is below minimum ({})",
                    validator.address, stake, min_stake
                ));
            }
            
            // Validate commission rate
            let commission: f64 = validator.commission_rate.parse()
                .map_err(|_| anyhow!("Invalid commission rate for {}", validator.address))?;
            
            if !(0.0..=1.0).contains(&commission) {
                return Err(anyhow!("Commission rate must be between 0.0 and 1.0 for {}", validator.address));
            }
            
            total_stake = total_stake.checked_add(stake)
                .ok_or_else(|| anyhow!("Total stake overflow"))?;
        }
        
        if total_stake == 0 {
            return Err(anyhow!("Total validator stake must be greater than 0"));
        }
        
        Ok(())
    }

    fn validate_accounts(accounts: &[crate::config::AccountConfig]) -> Result<()> {
        let mut addresses = HashSet::new();
        let mut total_supply: u128 = 0;
        
        for account in accounts {
            // Check unique addresses
            if !addresses.insert(&account.address) {
                return Err(anyhow!("Duplicate account address: {}", account.address));
            }
            
            // Validate address format
            if !account.address.starts_with("0x") || account.address.len() != 42 {
                return Err(anyhow!("Invalid address format: {}", account.address));
            }
            
            // Parse balance
            let balance: u128 = account.balance.parse()
                .map_err(|_| anyhow!("Invalid balance for {}", account.address))?;
            
            // Check balance
            if balance == 0 {
                return Err(anyhow!("Account {} balance must be greater than 0", account.address));
            }
            
            total_supply = total_supply.checked_add(balance)
                .ok_or_else(|| anyhow!("Total supply overflow"))?;
        }
        
        Ok(())
    }
}
