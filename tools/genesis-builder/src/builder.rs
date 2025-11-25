use crate::config::GenesisConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GenesisOutput {
    pub chain_id: String,
    pub timestamp: u64,
    pub initial_height: u64,
    pub consensus_params: ConsensusParams,
    pub governance_params: GovernanceParams,
    pub validators: Vec<ValidatorOutput>,
    pub accounts: Vec<AccountOutput>,
    pub app_state: AppState,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsensusParams {
    pub block_time_ms: u64,
    pub max_validators: usize,
    pub min_stake: u128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GovernanceParams {
    pub proposal_deposit: u128,
    pub voting_period_blocks: u64,
    pub quorum_threshold: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidatorOutput {
    pub address: String,
    pub stake: u128,
    pub commission_rate: f64,
    pub public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountOutput {
    pub address: String,
    pub balance: u128,
    pub nonce: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppState {
    pub total_supply: u128,
    pub total_stake: u128,
}

pub struct GenesisBuilder;

impl GenesisBuilder {
    pub fn build(config: GenesisConfig) -> Result<GenesisOutput> {
        let validators = config.validators.iter().map(|v| {
            Ok(ValidatorOutput {
                address: v.address.clone(),
                stake: v.stake.parse()?,
                commission_rate: v.commission_rate.parse().unwrap_or(0.0),
                public_key: format!("pubkey_{}", v.address), // Placeholder
            })
        }).collect::<Result<Vec<_>>>()?;

        let accounts = config.accounts.iter().map(|a| {
            Ok(AccountOutput {
                address: a.address.clone(),
                balance: a.balance.parse()?,
                nonce: 0,
            })
        }).collect::<Result<Vec<_>>>()?;

        let total_supply: u128 = accounts.iter().map(|a| a.balance).sum();
        let total_stake: u128 = validators.iter().map(|v| v.stake).sum();

        Ok(GenesisOutput {
            chain_id: config.chain.chain_id,
            timestamp: config.chain.timestamp,
            initial_height: config.chain.initial_height,
            consensus_params: ConsensusParams {
                block_time_ms: config.consensus.block_time_ms,
                max_validators: config.consensus.max_validators,
                min_stake: config.consensus.min_stake.parse()?,
            },
            governance_params: GovernanceParams {
                proposal_deposit: config.governance.proposal_deposit.parse()?,
                voting_period_blocks: config.governance.voting_period_blocks,
                quorum_threshold: config.governance.quorum_threshold.parse().unwrap_or(0.334),
            },
            validators,
            accounts,
            app_state: AppState {
                total_supply,
                total_stake,
            },
        })
    }

    pub fn to_json(genesis: &GenesisOutput) -> Result<String> {
        Ok(serde_json::to_string_pretty(genesis)?)
    }
}
