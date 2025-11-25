use common::types::{Address, Account};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Faucet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaucetConfig {
    /// Amount to distribute per request (in smallest unit)
    pub drip_amount: u128,
    /// Cooldown period in seconds
    pub cooldown_seconds: u64,
    /// Maximum requests per address
    pub max_requests_per_address: u32,
}

impl Default for FaucetConfig {
    fn default() -> Self {
        Self {
            drip_amount: 100_000_000_000, // 100 tokens
            cooldown_seconds: 86400,      // 24 hours
            max_requests_per_address: 10,
        }
    }
}

/// Faucet service for distributing test tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faucet {
    config: FaucetConfig,
    /// Track last request time per address
    last_request: HashMap<Address, u64>,
    /// Track total requests per address
    request_count: HashMap<Address, u32>,
    /// Total tokens distributed
    total_distributed: u128,
}

impl Faucet {
    pub fn new(config: FaucetConfig) -> Self {
        Self {
            config,
            last_request: HashMap::new(),
            request_count: HashMap::new(),
            total_distributed: 0,
        }
    }

    /// Request tokens from faucet
    pub fn request_tokens(&mut self, address: Address) -> Result<u128, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check cooldown
        if let Some(&last_time) = self.last_request.get(&address) {
            let elapsed = now - last_time;
            if elapsed < self.config.cooldown_seconds {
                let remaining = self.config.cooldown_seconds - elapsed;
                return Err(format!("Cooldown active. Try again in {} seconds", remaining));
            }
        }

        // Check max requests
        let count = self.request_count.get(&address).copied().unwrap_or(0);
        if count >= self.config.max_requests_per_address {
            return Err("Maximum requests reached for this address".into());
        }

        // Update tracking
        self.last_request.insert(address, now);
        self.request_count.insert(address, count + 1);
        self.total_distributed += self.config.drip_amount;

        Ok(self.config.drip_amount)
    }

    /// Get time until next request is allowed
    pub fn time_until_next_request(&self, address: &Address) -> Option<u64> {
        self.last_request.get(address).map(|&last_time| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let elapsed = now - last_time;
            if elapsed >= self.config.cooldown_seconds {
                0
            } else {
                self.config.cooldown_seconds - elapsed
            }
        })
    }

    /// Get remaining requests for an address
    pub fn remaining_requests(&self, address: &Address) -> u32 {
        let count = self.request_count.get(address).copied().unwrap_or(0);
        self.config.max_requests_per_address.saturating_sub(count)
    }

    /// Get total distributed amount
    pub fn total_distributed(&self) -> u128 {
        self.total_distributed
    }

    /// Reset an address (admin only)
    pub fn reset_address(&mut self, address: &Address) {
        self.last_request.remove(address);
        self.request_count.remove(address);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_faucet_request() {
        let mut faucet = Faucet::new(FaucetConfig::default());
        let address = [1u8; 20];

        let amount = faucet.request_tokens(address).unwrap();
        assert_eq!(amount, 100_000_000_000);
        assert_eq!(faucet.total_distributed(), 100_000_000_000);
    }

    #[test]
    fn test_faucet_cooldown() {
        let config = FaucetConfig {
            drip_amount: 100_000_000_000,
            cooldown_seconds: 1, // 1 second for testing
            max_requests_per_address: 10,
        };
        let mut faucet = Faucet::new(config);
        let address = [1u8; 20];

        // First request should succeed
        assert!(faucet.request_tokens(address).is_ok());

        // Second immediate request should fail
        assert!(faucet.request_tokens(address).is_err());

        // Wait for cooldown
        sleep(Duration::from_secs(2));

        // Third request should succeed
        assert!(faucet.request_tokens(address).is_ok());
    }

    #[test]
    fn test_max_requests() {
        let config = FaucetConfig {
            drip_amount: 100_000_000_000,
            cooldown_seconds: 0, // No cooldown for testing
            max_requests_per_address: 2,
        };
        let mut faucet = Faucet::new(config);
        let address = [1u8; 20];

        // First two requests should succeed
        assert!(faucet.request_tokens(address).is_ok());
        sleep(Duration::from_millis(10));
        assert!(faucet.request_tokens(address).is_ok());

        // Third request should fail
        sleep(Duration::from_millis(10));
        assert!(faucet.request_tokens(address).is_err());
    }

    #[test]
    fn test_remaining_requests() {
        let mut faucet = Faucet::new(FaucetConfig::default());
        let address = [1u8; 20];

        assert_eq!(faucet.remaining_requests(&address), 10);
        
        faucet.request_tokens(address).unwrap();
        assert_eq!(faucet.remaining_requests(&address), 9);
    }
}
