use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Parse a 20-byte Ethereum address from hex (with or without `0x`).
fn parse_eth_address(hex_str: &str) -> [u8; 20] {
    let s = hex_str.trim_start_matches("0x");
    let mut out = [0u8; 20];
    for (i, byte) in out.iter_mut().enumerate() {
        let pair = &s[i * 2..i * 2 + 2];
        *byte = u8::from_str_radix(pair, 16).unwrap_or(0);
    }
    out
}

/// Deterministic Nebula-side token address for a bridged Ethereum asset.
fn mapped_chain_address(eth: [u8; 20]) -> [u8; 20] {
    let mut h = Sha256::new();
    h.update(b"nebula-bridge-token-v1:");
    h.update(eth);
    let digest: [u8; 32] = h.finalize().into();
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&digest[..20]);
    addr
}

/// Token information for bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub eth_address: [u8; 20],
    pub chain_address: [u8; 20],
    pub min_amount: u128,
    pub max_amount: u128,
    pub enabled: bool,
}

/// Token registry for managing supported tokens
#[derive(Debug, Clone)]
pub struct TokenRegistry {
    tokens: HashMap<String, TokenInfo>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    /// Add a token to the registry
    pub fn add_token(&mut self, symbol: String, info: TokenInfo) -> Result<(), String> {
        if self.tokens.contains_key(&symbol) {
            return Err(format!("Token {} already exists", symbol));
        }
        self.tokens.insert(symbol, info);
        Ok(())
    }

    /// Get token info by symbol
    pub fn get_token(&self, symbol: &str) -> Option<&TokenInfo> {
        self.tokens.get(symbol)
    }

    /// Check if token is supported
    pub fn is_supported(&self, symbol: &str) -> bool {
        self.tokens.get(symbol).map_or(false, |t| t.enabled)
    }

    /// Get all supported tokens
    pub fn get_supported_tokens(&self) -> Vec<&TokenInfo> {
        self.tokens.values().filter(|t| t.enabled).collect()
    }

    /// Enable/disable a token
    pub fn set_enabled(&mut self, symbol: &str, enabled: bool) -> Result<(), String> {
        if let Some(token) = self.tokens.get_mut(symbol) {
            token.enabled = enabled;
            Ok(())
        } else {
            Err(format!("Token {} not found", symbol))
        }
    }

    /// Validate amount for a token
    pub fn validate_amount(&self, symbol: &str, amount: u128) -> Result<(), String> {
        if let Some(token) = self.tokens.get(symbol) {
            if amount < token.min_amount {
                return Err(format!("Amount below minimum: {}", token.min_amount));
            }
            if amount > token.max_amount {
                return Err(format!("Amount above maximum: {}", token.max_amount));
            }
            Ok(())
        } else {
            Err(format!("Token {} not found", symbol))
        }
    }
}

impl Default for TokenRegistry {
    fn default() -> Self {
        let mut registry = Self::new();

        let usdc_eth = parse_eth_address("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
        registry
            .add_token(
                "USDC".to_string(),
                TokenInfo {
                    symbol: "USDC".to_string(),
                    name: "USD Coin".to_string(),
                    decimals: 6,
                    eth_address: usdc_eth,
                    chain_address: mapped_chain_address(usdc_eth),
                    min_amount: 1_000_000,
                    max_amount: 1_000_000_000_000,
                    enabled: true,
                },
            )
            .ok();

        let usdt_eth = parse_eth_address("0xdAC17F958D2ee523a2206206994597C13D831ec7");
        registry
            .add_token(
                "USDT".to_string(),
                TokenInfo {
                    symbol: "USDT".to_string(),
                    name: "Tether USD".to_string(),
                    decimals: 6,
                    eth_address: usdt_eth,
                    chain_address: mapped_chain_address(usdt_eth),
                    min_amount: 1_000_000,
                    max_amount: 1_000_000_000_000,
                    enabled: true,
                },
            )
            .ok();

        let weth_eth = parse_eth_address("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        registry
            .add_token(
                "ETH".to_string(),
                TokenInfo {
                    symbol: "ETH".to_string(),
                    name: "Ethereum".to_string(),
                    decimals: 18,
                    eth_address: weth_eth,
                    chain_address: mapped_chain_address(weth_eth),
                    min_amount: 10_000_000_000_000_000,
                    max_amount: 100_000_000_000_000_000_000,
                    enabled: true,
                },
            )
            .ok();

        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_registry() {
        let registry = TokenRegistry::default();

        assert!(registry.is_supported("USDC"));
        assert!(registry.is_supported("USDT"));
        assert!(registry.is_supported("ETH"));
        assert!(!registry.is_supported("UNKNOWN"));

        let usdc = registry.get_token("USDC").unwrap();
        assert_eq!(usdc.symbol, "USDC");
        assert_eq!(usdc.decimals, 6);
        assert_ne!(usdc.eth_address, [0u8; 20]);
        assert_ne!(usdc.chain_address, [0u8; 20]);
        assert_eq!(usdc.chain_address, mapped_chain_address(usdc.eth_address));
    }

    #[test]
    fn test_amount_validation() {
        let registry = TokenRegistry::default();

        // Valid amount
        assert!(registry.validate_amount("USDC", 10_000_000).is_ok());

        // Below minimum
        assert!(registry.validate_amount("USDC", 100_000).is_err());

        // Above maximum
        assert!(registry.validate_amount("USDC", 10_000_000_000_000).is_err());
    }

    #[test]
    fn test_enable_disable() {
        let mut registry = TokenRegistry::default();

        registry.set_enabled("USDC", false).unwrap();
        assert!(!registry.is_supported("USDC"));

        registry.set_enabled("USDC", true).unwrap();
        assert!(registry.is_supported("USDC"));
    }
}
