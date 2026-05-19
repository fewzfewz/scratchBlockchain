//! # Node Configuration Module
//! 
//! This module defines all configuration structures for the blockchain node.
//! Configuration is loaded from TOML files and supports environment variable
//! overrides for sensitive values (like private keys).
//! 
//! ## Configuration Priority (highest to lowest)
//! 1. Environment variables (e.g., `NODE_RPC_PORT=8545`)
//! 2. Configuration file values
//! 3. Default values

use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;

/// Main node configuration structure
/// 
/// Contains all configuration sections for running a blockchain node.
/// Supports loading from TOML files with environment variable overrides.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NodeConfig {
    /// Network configuration (P2P, RPC, bootstrap)
    pub network: NetworkConfig,
    
    /// Consensus parameters (block time, validator limits)
    pub consensus: ConsensusConfig,
    
    /// Validator-specific configuration
    pub validator: ValidatorConfig,
    
    /// Storage paths and settings
    pub storage: StorageConfig,
    
    /// API server configuration
    pub api: ApiConfig,
    
    /// Metrics and monitoring configuration
    pub metrics: MetricsConfig,
    
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
    
    /// Security configuration (rate limits, max sizes)
    #[serde(default)]
    pub security: SecurityConfig,
}

/// Network-layer configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NetworkConfig {
    /// Chain identifier (prevents cross-chain replay attacks)
    pub chain_id: String,
    
    /// P2P listening port (default: 9000)
    #[serde(default = "default_p2p_port")]
    pub p2p_port: u16,
    
    /// RPC listening port (default: 9933)
    #[serde(default = "default_rpc_port")]
    pub rpc_port: u16,
    
    /// WebSocket listening port (default: 9944)
    #[serde(default = "default_ws_port")]
    pub ws_port: u16,
    
    /// Bootstrap nodes (multiaddresses for initial peer discovery)
    #[serde(default)]
    pub bootstrap_nodes: Vec<String>,
    
    /// Maximum number of peers to maintain (default: 50)
    #[serde(default = "default_max_peers")]
    pub max_peers: usize,
    
    /// Peer connection timeout in seconds (default: 10)
    #[serde(default = "default_peer_timeout_secs")]
    pub peer_timeout_secs: u64,
}

/// Consensus configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConsensusConfig {
    /// Target block time in milliseconds (default: 3000)
    #[serde(default = "default_block_time_ms")]
    pub block_time_ms: u64,
    
    /// Maximum number of validators in the active set
    #[serde(default = "default_max_validators")]
    pub max_validators: usize,
    
    /// Minimum stake required to become a validator (default: 1000)
    #[serde(default = "default_min_validator_stake")]
    pub min_validator_stake: u64,
    
    /// Number of blocks between epoch changes (validator set updates)
    #[serde(default = "default_epoch_length")]
    pub epoch_length: u64,
    
    /// Maximum number of rounds before an unresponsive validator is slashed
    #[serde(default = "default_max_rounds_per_height")]
    pub max_rounds_per_height: u64,
}

/// Validator-specific configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ValidatorConfig {
    /// Whether this node participates as a validator
    #[serde(default)]
    pub enabled: bool,
    
    /// Validator commission rate (e.g., "0.05" for 5%)
    #[serde(default)]
    pub commission_rate: Option<String>,
    
    /// Path to validator key file (overrides data_dir)
    #[serde(default)]
    pub key_file: Option<String>,
}

/// Storage configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StorageConfig {
    /// Base directory for all blockchain data
    pub data_dir: String,
    
    /// Maximum database cache size in MB (default: 512)
    #[serde(default = "default_db_cache_mb")]
    pub db_cache_mb: usize,
    
    /// Enable database compression (default: true)
    #[serde(default = "default_true")]
    pub db_compression: bool,
    
    /// Pruning mode: "archive" (keep all), "full" (keep recent), "minimal" (keep state only)
    #[serde(default = "default_pruning_mode")]
    pub pruning_mode: String,
}

/// API server configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiConfig {
    /// Enable API server (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// API listen address (default: "127.0.0.1")
    #[serde(default = "default_api_address")]
    pub address: String,
    
    /// CORS allowed origins for web clients
    #[serde(default)]
    pub cors_origins: Vec<String>,
    
    /// API rate limit (requests per minute, default: 1000)
    #[serde(default = "default_rate_limit")]
    pub rate_limit: u32,
}

/// Metrics configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MetricsConfig {
    /// Enable Prometheus metrics endpoint (default: false)
    #[serde(default)]
    pub enabled: bool,
    
    /// Metrics listen address (default: "127.0.0.1:9090")
    #[serde(default = "default_metrics_address")]
    pub address: String,
}

/// Logging configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    /// Log level: "trace", "debug", "info", "warn", "error"
    #[serde(default = "default_log_level")]
    pub level: String,
    
    /// Log format: "json" or "pretty" (default: "pretty")
    #[serde(default = "default_log_format")]
    pub format: String,
    
    /// Log file path (if empty, logs to stdout)
    #[serde(default)]
    pub file_path: String,
}

/// Security configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SecurityConfig {
    /// Maximum transaction size in bytes (default: 1MB)
    #[serde(default = "default_max_tx_size_bytes")]
    pub max_tx_size_bytes: usize,
    
    /// Maximum block size in bytes (default: 10MB)
    #[serde(default = "default_max_block_size_bytes")]
    pub max_block_size_bytes: usize,
    
    /// Maximum transactions per block (default: 5000)
    #[serde(default = "default_max_tx_per_block")]
    pub max_tx_per_block: usize,
    
    /// Enable DOS protection (rate limiting per peer)
    #[serde(default = "default_true")]
    pub dos_protection_enabled: bool,
    
    /// Maximum gas per block (default: 30 million)
    #[serde(default = "default_max_gas_per_block")]
    pub max_gas_per_block: u64,
}

// ============================================================================
// Default Value Functions
// ============================================================================

fn default_p2p_port() -> u16 { 9000 }
fn default_rpc_port() -> u16 { 9933 }
fn default_ws_port() -> u16 { 9944 }
fn default_max_peers() -> usize { 50 }
fn default_peer_timeout_secs() -> u64 { 10 }
fn default_block_time_ms() -> u64 { 3000 }
fn default_max_validators() -> usize { 100 }
fn default_min_validator_stake() -> u64 { 1000 }
fn default_epoch_length() -> u64 { 100 }
fn default_max_rounds_per_height() -> u64 { 1000 }
fn default_db_cache_mb() -> usize { 512 }
fn default_true() -> bool { true }
fn default_pruning_mode() -> String { "full".to_string() }
fn default_api_address() -> String { "127.0.0.1".to_string() }
fn default_rate_limit() -> u32 { 1000 }
fn default_metrics_address() -> String { "127.0.0.1:9090".to_string() }
fn default_log_level() -> String { "info".to_string() }
fn default_log_format() -> String { "pretty".to_string() }
fn default_max_tx_size_bytes() -> usize { 1024 * 1024 }  // 1 MB
fn default_max_block_size_bytes() -> usize { 10 * 1024 * 1024 }  // 10 MB
fn default_max_tx_per_block() -> usize { 5000 }
fn default_max_gas_per_block() -> u64 { 30_000_000 }

impl NodeConfig {
    /// Load configuration from a TOML file with environment variable overrides
    /// 
    /// Environment variables take precedence over file values.
    /// Format: NODE_{SECTION}_{FIELD} (e.g., NODE_NETWORK_P2P_PORT=1234)
    /// 
    /// # Example
    /// ```ignore
    /// let config = NodeConfig::load("config.toml")?;
    /// ```
    pub fn load(path: &Path) -> Result<Self, anyhow::Error> {
        let content = std::fs::read_to_string(path)?;
        let mut config: NodeConfig = toml::from_str(&content)?;
        
        // Apply environment variable overrides
        config.apply_env_overrides();
        
        // Validate configuration
        config.validate()?;
        
        Ok(config)
    }
    
    /// Apply environment variable overrides to configuration
    /// 
    /// Environment variables use the pattern: NODE_{SECTION}_{FIELD}
    /// Example: NODE_NETWORK_P2P_PORT=1234 overrides network.p2p_port
    fn apply_env_overrides(&mut self) {
        // Network overrides
        if let Ok(val) = env::var("NODE_NETWORK_P2P_PORT") {
            if let Ok(port) = val.parse() {
                self.network.p2p_port = port;
            }
        }
        
        if let Ok(val) = env::var("NODE_NETWORK_RPC_PORT") {
            if let Ok(port) = val.parse() {
                self.network.rpc_port = port;
            }
        }
        
        if let Ok(val) = env::var("NODE_NETWORK_MAX_PEERS") {
            if let Ok(max) = val.parse() {
                self.network.max_peers = max;
            }
        }
        
        // Storage overrides
        if let Ok(val) = env::var("NODE_STORAGE_DATA_DIR") {
            self.storage.data_dir = val;
        }
        
        if let Ok(val) = env::var("NODE_STORAGE_DB_CACHE_MB") {
            if let Ok(cache) = val.parse() {
                self.storage.db_cache_mb = cache;
            }
        }
        
        // Logging overrides
        if let Ok(val) = env::var("NODE_LOGGING_LEVEL") {
            self.logging.level = val;
        }
        
        // Validator overrides
        if let Ok(val) = env::var("NODE_VALIDATOR_ENABLED") {
            self.validator.enabled = val.parse().unwrap_or(false);
        }
    }
    
    /// Validate configuration for consistency and safety
    fn validate(&self) -> Result<(), anyhow::Error> {
        // Validate ports are within valid range
        if self.network.p2p_port == 0 || self.network.p2p_port > 65535 {
            anyhow::bail!("Invalid P2P port: {}", self.network.p2p_port);
        }
        
        if self.network.rpc_port == 0 || self.network.rpc_port > 65535 {
            anyhow::bail!("Invalid RPC port: {}", self.network.rpc_port);
        }
        
        // Ensure ports don't conflict
        if self.network.p2p_port == self.network.rpc_port {
            anyhow::bail!("P2P and RPC ports cannot be the same");
        }
        
        // Validate block time is reasonable
        if self.consensus.block_time_ms < 500 {
            anyhow::bail!("Block time cannot be less than 500ms");
        }
        
        // Validate storage directory exists or can be created
        let data_dir = Path::new(&self.storage.data_dir);
        if !data_dir.exists() {
            std::fs::create_dir_all(data_dir)?;
        }
        
        // Validate logging level
        match self.logging.level.as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {}
            _ => anyhow::bail!("Invalid log level: {}", self.logging.level),
        }
        
        Ok(())
    }
    
    /// Generate a default configuration for development/testing
    pub fn default_for_development() -> Self {
        Self {
            network: NetworkConfig {
                chain_id: "dev-1".to_string(),
                p2p_port: 9000,
                rpc_port: 9933,
                ws_port: 9944,
                bootstrap_nodes: vec![],
                max_peers: 50,
                peer_timeout_secs: 10,
            },
            consensus: ConsensusConfig {
                block_time_ms: 3000,
                max_validators: 100,
                min_validator_stake: 1000,
                epoch_length: 100,
                max_rounds_per_height: 1000,
            },
            validator: ValidatorConfig {
                enabled: true,
                commission_rate: Some("0.05".to_string()),
                key_file: None,
            },
            storage: StorageConfig {
                data_dir: "./data".to_string(),
                db_cache_mb: 512,
                db_compression: true,
                pruning_mode: "full".to_string(),
            },
            api: ApiConfig {
                enabled: true,
                address: "127.0.0.1".to_string(),
                cors_origins: vec!["http://localhost:3000".to_string()],
                rate_limit: 1000,
            },
            metrics: MetricsConfig {
                enabled: false,
                address: "127.0.0.1:9090".to_string(),
            },
            logging: LoggingConfig {
                level: "debug".to_string(),
                format: "pretty".to_string(),
                file_path: "".to_string(),
            },
            security: SecurityConfig {
                max_tx_size_bytes: 1024 * 1024,
                max_block_size_bytes: 10 * 1024 * 1024,
                max_tx_per_block: 5000,
                dos_protection_enabled: true,
                max_gas_per_block: 30_000_000,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_config_load_and_validate() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        
        let toml_content = r#"
            [network]
            chain_id = "test-1"
            p2p_port = 9001
            
            [storage]
            data_dir = "./test_data"
            
            [validator]
            enabled = true
        "#;
        
        std::fs::write(&config_path, toml_content).unwrap();
        
        let config = NodeConfig::load(&config_path).unwrap();
        assert_eq!(config.network.chain_id, "test-1");
        assert_eq!(config.network.p2p_port, 9001);
        assert!(config.validator.enabled);
    }
    
    #[test]
    fn test_env_override() {
        env::set_var("NODE_NETWORK_P2P_PORT", "9999");
        
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        
        let toml_content = r#"
            [network]
            chain_id = "test-1"
            p2p_port = 9001
            
            [storage]
            data_dir = "./test_data"
        "#;
        
        std::fs::write(&config_path, toml_content).unwrap();
        
        let config = NodeConfig::load(&config_path).unwrap();
        
        // Environment variable should override file value
        assert_eq!(config.network.p2p_port, 9999);
        
        env::remove_var("NODE_NETWORK_P2P_PORT");
    }
    
    #[test]
    fn test_invalid_port_rejected() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        
        let toml_content = r#"
            [network]
            chain_id = "test-1"
            p2p_port = 0  # Invalid port
            
            [storage]
            data_dir = "./test_data"
        "#;
        
        std::fs::write(&config_path, toml_content).unwrap();
        
        let result = NodeConfig::load(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("P2P port"));
    }
}