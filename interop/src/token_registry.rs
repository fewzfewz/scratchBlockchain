use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

        // Add USDC
        registry.add_token(
            "USDC".to_string(),
            TokenInfo {
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                decimals: 6,
                eth_address: [0u8; 20], // Placeholder
                chain_address: [0u8; 20], // Placeholder
                min_amount: 1_000_000, // 1 USDC
                max_amount: 1_000_000_000_000, // 1M USDC
                enabled: true,
            },
        ).ok();

        // Add USDT
        registry.add_token(
            "USDT".to_string(),
            TokenInfo {
                symbol: "USDT".to_string(),
                name: "Tether USD".to_string(),
                decimals: 6,
                eth_address: [0u8; 20], // Placeholder
                chain_address: [0u8; 20], // Placeholder
                min_amount: 1_000_000, // 1 USDT
                max_amount: 1_000_000_000_000, // 1M USDT
                enabled: true,
            },
        ).ok();

        // Add ETH
        registry.add_token(
            "ETH".to_string(),
            TokenInfo {
                symbol: "ETH".to_string(),
                name: "Ethereum".to_string(),
                decimals: 18,
                eth_address: [0u8; 20], // Native token
                chain_address: [0u8; 20], // Wrapped ETH on chain
                min_amount: 10_000_000_000_000_000, // 0.01 ETH
                max_amount: 100_000_000_000_000_000_000, // 100 ETH
                enabled: true,
            },
        ).ok();

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
