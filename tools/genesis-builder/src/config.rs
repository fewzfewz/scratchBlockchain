use serde::{Deserialize, Serialize};
use common::types::Address;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub chain: ChainConfig,
    pub consensus: ConsensusConfig,
    pub governance: GovernanceConfig,
    #[serde(default)]
    pub validators: Vec<ValidatorConfig>,
    #[serde(default)]
    pub accounts: Vec<AccountConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_id: String,
    pub timestamp: u64,
    #[serde(default)]
    pub initial_height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    pub block_time_ms: u64,
    pub max_validators: usize,
    pub min_stake: String, // Parse as u128
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceConfig {
    pub proposal_deposit: String, // Parse as u128
    pub voting_period_blocks: u64,
    pub quorum_threshold: String, // Stored as string, parsed to f64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    pub address: String,
    pub stake: String, // Parse as u128
    pub commission_rate: String, // Stored as string, parsed to f64
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub address: String,
    pub balance: String, // Parse as u128
}

impl GenesisConfig {
    pub fn from_toml(content: &str) -> anyhow::Result<Self> {
        Ok(toml::from_str(content)?)
    }
}
