use serde::Deserialize;
use std::path::Path;
use anyhow::Result;

#[derive(Debug, Deserialize, Clone)]
pub struct NodeConfig {
    pub network: NetworkConfig,
    pub consensus: ConsensusConfig,
    pub validator: ValidatorConfig,
    pub storage: StorageConfig,
    pub api: ApiConfig,
    pub metrics: MetricsConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NetworkConfig {
    pub chain_id: String,
    pub p2p_port: u16,
    pub rpc_port: u16,
    pub bootstrap_nodes: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConsensusConfig {
    pub block_time_ms: u64,
    #[serde(default = "default_max_validators")]
    pub max_validators: usize,
}

fn default_max_validators() -> usize {
    100
}

#[derive(Debug, Deserialize, Clone)]
pub struct ValidatorConfig {
    pub enabled: bool,
    pub commission_rate: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StorageConfig {
    pub data_dir: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiConfig {
    pub enabled: bool,
    pub address: String,
    pub cors_origins: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub address: String,
}

impl NodeConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: NodeConfig = toml::from_str(&content)?;
        Ok(config)
    }
}
