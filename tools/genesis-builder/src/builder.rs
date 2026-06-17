//! # Genesis Builder Module
//!
//! Builds the final genesis.json from validated configuration.

use crate::config::GenesisConfig;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Output genesis structure for the blockchain node
#[derive(Debug, Serialize, Deserialize)]
pub struct GenesisOutput {
    pub chain_id: String,
    pub timestamp: u64,
    pub initial_height: u64,
    pub network_id: u64,
    pub consensus_params: ConsensusParams,
    pub governance_params: GovernanceParams,
    pub economic_params: EconomicParams,
    pub validators: Vec<ValidatorOutput>,
    pub accounts: Vec<AccountOutput>,
    pub precompiles: Vec<PrecompileOutput>,
    pub app_state: AppState,
}

/// Consensus parameters for the node
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsensusParams {
    pub block_time_ms: u64,
    pub max_validators: usize,
    pub min_stake: u128,
    pub unbonding_period: u64,
    pub max_validators_per_epoch: usize,
}

/// Governance parameters
#[derive(Debug, Serialize, Deserialize)]
pub struct GovernanceParams {
    pub proposal_deposit: u128,
    pub voting_period_blocks: u64,
    pub quorum_threshold: f64,
    pub approval_threshold: f64,
    pub max_proposals: usize,
}

/// Economic parameters
#[derive(Debug, Serialize, Deserialize)]
pub struct EconomicParams {
    pub initial_block_reward: u128,
    pub halving_interval: u64,
    pub fee_burn_percentage: u8,
    pub treasury_address: [u8; 20],
}

/// Validator output format
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidatorOutput {
    pub address: [u8; 20],
    pub stake: u128,
    pub commission_rate: f64,
    pub public_key: String,
    pub description: Option<String>,
    pub website: Option<String>,
}

/// Account output format
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountOutput {
    pub address: [u8; 20],
    pub balance: u128,
    pub nonce: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<HashMap<String, String>>,
}

/// Precompiled contract output
#[derive(Debug, Serialize, Deserialize)]
pub struct PrecompileOutput {
    pub address: [u8; 20],
    pub contract_type: String,
    pub enabled: bool,
}

/// Application state summary
#[derive(Debug, Serialize, Deserialize)]
pub struct AppState {
    pub total_supply: u128,
    pub total_stake: u128,
    pub version: String,
}

pub struct GenesisBuilder;

/// Parse hex address string to byte array
pub fn parse_address(addr_str: &str) -> Result<[u8; 20]> {
    let addr_str = addr_str.strip_prefix("0x").unwrap_or(addr_str);
    let bytes = hex::decode(addr_str)
        .map_err(|e| anyhow!("Invalid address hex: {}", e))?;
    
    if bytes.len() != 20 {
        anyhow::bail!("Address must be 20 bytes, got {}", bytes.len());
    }
    
    let mut arr = [0u8; 20];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Parse treasury address
pub fn parse_treasury_address(addr_str: &str) -> Result<[u8; 20]> {
    parse_address(addr_str)
}

impl GenesisBuilder {
    /// Build genesis output from configuration
    pub fn build(config: GenesisConfig) -> Result<GenesisOutput> {
        // Build validators
        let validators = config.validators.iter()
            .map(|v| {
                Ok(ValidatorOutput {
                    address: parse_address(&v.address)?,
                    stake: v.stake.parse()
                        .map_err(|_| anyhow!("Invalid stake for {}: {}", v.address, v.stake))?,
                    commission_rate: v.commission_rate.parse()
                        .unwrap_or(0.05),
                    public_key: v.public_key.clone(),
                    description: v.description.clone(),
                    website: v.website.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        // Build accounts
        let accounts = config.accounts.iter()
            .map(|a| {
                Ok(AccountOutput {
                    address: parse_address(&a.address)?,
                    balance: a.balance.parse()
                        .map_err(|_| anyhow!("Invalid balance for {}: {}", a.address, a.balance))?,
                    nonce: 0,
                    code: a.code.clone(),
                    storage: a.storage.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        // Build precompiles
        let precompiles = config.precompiles.iter()
            .map(|p| {
                Ok(PrecompileOutput {
                    address: parse_address(&p.address)?,
                    contract_type: p.contract_type.clone(),
                    enabled: p.enabled,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        // Calculate totals
        let total_supply: u128 = accounts.iter().map(|a| a.balance).sum();
        let total_stake: u128 = validators.iter().map(|v| v.stake).sum();

        // Parse economic params
        let initial_block_reward: u128 = config.economic.initial_block_reward.parse()
            .map_err(|_| anyhow!("Invalid initial_block_reward: {}", config.economic.initial_block_reward))?;
        
        let treasury_address = parse_treasury_address(&config.economic.treasury_address)?;

        Ok(GenesisOutput {
            chain_id: config.chain.chain_id,
            timestamp: config.chain.timestamp,
            initial_height: config.chain.initial_height,
            network_id: config.chain.network_id,
            consensus_params: ConsensusParams {
                block_time_ms: config.consensus.block_time_ms,
                max_validators: config.consensus.max_validators,
                min_stake: config.consensus.min_stake.parse()
                    .map_err(|_| anyhow!("Invalid min_stake: {}", config.consensus.min_stake))?,
                unbonding_period: config.consensus.unbonding_period,
                max_validators_per_epoch: config.consensus.max_validators_per_epoch,
            },
            governance_params: GovernanceParams {
                proposal_deposit: config.governance.proposal_deposit.parse()
                    .map_err(|_| anyhow!("Invalid proposal_deposit: {}", config.governance.proposal_deposit))?,
                voting_period_blocks: config.governance.voting_period_blocks,
                quorum_threshold: config.governance.quorum_threshold.parse()
                    .unwrap_or(0.334),
                approval_threshold: config.governance.approval_threshold.parse()
                    .unwrap_or(0.5),
                max_proposals: config.governance.max_proposals,
            },
            economic_params: EconomicParams {
                initial_block_reward,
                halving_interval: config.economic.halving_interval,
                fee_burn_percentage: config.economic.fee_burn_percentage,
                treasury_address,
            },
            validators,
            accounts,
            precompiles,
            app_state: AppState {
                total_supply,
                total_stake,
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        })
    }

    /// Convert genesis output to pretty JSON
    pub fn to_json(genesis: &GenesisOutput) -> Result<String> {
        Ok(serde_json::to_string_pretty(genesis)?)
    }
    
    /// Validate built genesis (sanity checks)
    pub fn validate_built(genesis: &GenesisOutput) -> Result<()> {
        if genesis.chain_id.is_empty() {
            return Err(anyhow!("Chain ID is empty"));
        }
        
        if genesis.validators.is_empty() {
            return Err(anyhow!("No validators in genesis"));
        }
        
        if genesis.accounts.is_empty() {
            return Err(anyhow!("No accounts in genesis"));
        }
        
        if genesis.app_state.total_supply == 0 {
            return Err(anyhow!("Total supply is zero"));
        }
        
        // Check validator addresses are unique
        let mut addresses = std::collections::HashSet::new();
        for validator in &genesis.validators {
            if !addresses.insert(validator.address) {
                return Err(anyhow!("Duplicate validator address: {:?}", validator.address));
            }
        }
        
        // Check account addresses are unique
        let mut addresses = std::collections::HashSet::new();
        for account in &genesis.accounts {
            if !addresses.insert(account.address) {
                return Err(anyhow!("Duplicate account address: {:?}", account.address));
            }
        }
        
        Ok(())
    }
}