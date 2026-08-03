//! # Genesis Configuration Module
//!
//! This module defines the configuration structures for blockchain genesis.
//! Configuration is loaded from TOML files and validated before genesis generation.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Complete genesis configuration loaded from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub chain: ChainConfig,
    pub consensus: ConsensusConfig,
    pub governance: GovernanceConfig,
    pub economic: EconomicConfig,
    #[serde(default)]
    pub validators: Vec<ValidatorConfig>,
    #[serde(default)]
    pub accounts: Vec<AccountConfig>,
    #[serde(default)]
    pub precompiles: Vec<PrecompileConfig>,
}

/// Chain-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_id: String,
    pub timestamp: u64,
    #[serde(default = "default_initial_height")]
    pub initial_height: u64,
    #[serde(default = "default_network_id")]
    pub network_id: u64,
    #[serde(default = "default_chain_name")]
    pub name: String,
}

/// Consensus parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    pub block_time_ms: u64,
    pub max_validators: usize,
    pub min_stake: String, // Parse as u128
    #[serde(default = "default_unbonding_period")]
    pub unbonding_period: u64, // blocks
    #[serde(default = "default_max_validators_per_epoch")]
    pub max_validators_per_epoch: usize,
}

/// Governance parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceConfig {
    pub proposal_deposit: String, // Parse as u128
    pub voting_period_blocks: u64,
    pub quorum_threshold: String, // Stored as string, parsed to f64
    #[serde(default = "default_approval_threshold")]
    pub approval_threshold: String,
    #[serde(default = "default_max_proposals")]
    pub max_proposals: usize,
}

/// Economic parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicConfig {
    #[serde(default = "default_initial_reward")]
    pub initial_block_reward: String, // Parse as u128
    #[serde(default = "default_halving_interval")]
    pub halving_interval: u64,
    #[serde(default = "default_fee_burn_percentage")]
    pub fee_burn_percentage: u8,
    #[serde(default = "default_treasury_address")]
    pub treasury_address: String,
}

/// Validator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    pub address: String,
    pub stake: String,           // Parse as u128
    pub commission_rate: String, // Stored as string, parsed to f64
    pub public_key: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
}

/// Account configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub address: String,
    pub balance: String, // Parse as u128
    #[serde(default)]
    pub code: Option<String>, // Contract code (hex)
    #[serde(default)]
    pub storage: Option<std::collections::HashMap<String, String>>,
}

/// Precompiled contract configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecompileConfig {
    pub address: String,
    pub contract_type: String,
    pub enabled: bool,
}

// Default value functions
fn default_initial_height() -> u64 {
    0
}
fn default_network_id() -> u64 {
    1
}
fn default_chain_name() -> String {
    "Modular Blockchain".to_string()
}
fn default_unbonding_period() -> u64 {
    100_800
} // ~7 days at 6s blocks
fn default_max_validators_per_epoch() -> usize {
    100
}
fn default_approval_threshold() -> String {
    "0.5".to_string()
}
fn default_max_proposals() -> usize {
    100
}
fn default_initial_reward() -> String {
    "10000000000000000000".to_string()
} // 10 tokens
fn default_halving_interval() -> u64 {
    2_100_000
} // ~4 years
fn default_fee_burn_percentage() -> u8 {
    50
}
fn default_treasury_address() -> String {
    "0x0000000000000000000000000000000000000000".to_string()
}

impl GenesisConfig {
    /// Load genesis configuration from TOML string
    pub fn from_toml(content: &str) -> Result<Self> {
        let config: GenesisConfig = toml::from_str(content)?;
        Ok(config)
    }

    /// Load from file
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_toml(&content)
    }

    /// Generate example TOML configuration
    pub fn example_toml() -> String {
        r#"# Genesis Configuration Example
[chain]
chain_id = "mainnet-1"
timestamp = 1700000000
initial_height = 0
network_id = 1
name = "Modular Blockchain"

[consensus]
block_time_ms = 3000
max_validators = 100
min_stake = "1000000000000000000"  # 1 token
unbonding_period = 100800          # ~7 days
max_validators_per_epoch = 100

[governance]
proposal_deposit = "1000000000000000000"  # 1 token
voting_period_blocks = 50400              # ~3.5 days
quorum_threshold = "0.334"                # 33.4%
approval_threshold = "0.5"                # 50%
max_proposals = 100

[economic]
initial_block_reward = "10000000000000000000"  # 10 tokens
halving_interval = 2100000                     # ~4 years
fee_burn_percentage = 50
treasury_address = "0x0000000000000000000000000000000000000000"

# Validators
[[validators]]
address = "0x1111111111111111111111111111111111111111"
stake = "10000000000000000000"  # 10 tokens
commission_rate = "0.05"         # 5%
public_key = "0x..."

# Accounts
[[accounts]]
address = "0x2222222222222222222222222222222222222222"
balance = "100000000000000000000"  # 100 tokens

# Precompiled contracts (optional)
[[precompiles]]
address = "0x0000000000000000000000000000000000000001"
contract_type = "ecrecover"
enabled = true
"#
        .to_string()
    }
}
