use anyhow::{anyhow, Result};

/// Gas costs for various operations (similar to Ethereum)
pub struct GasCosts;

impl GasCosts {
    // Base costs
    pub const TRANSACTION: u64 = 21_000;
    pub const CREATE: u64 = 32_000;
    pub const CALL: u64 = 700;
    
    // Memory operations
    pub const MEMORY_WORD: u64 = 3;
    pub const MEMORY_EXPANSION: u64 = 512;
    
    // Storage operations
    pub const SLOAD: u64 = 2_100;
    pub const SSTORE_SET: u64 = 20_000;
    pub const SSTORE_RESET: u64 = 5_000;
    pub const SSTORE_REFUND: u64 = 15_000;
    
    // Computational operations
    pub const ADD: u64 = 3;
    pub const MUL: u64 = 5;
    pub const DIV: u64 = 5;
    pub const SDIV: u64 = 5;
    pub const MOD: u64 = 5;
    pub const EXP: u64 = 10;
    pub const SHA3: u64 = 30;
    pub const SHA3_WORD: u64 = 6;
    
    // Account operations
    pub const BALANCE: u64 = 700;
    pub const EXTCODESIZE: u64 = 700;
    pub const EXTCODECOPY: u64 = 700;
    pub const BLOCKHASH: u64 = 20;
    
    // Logging
    pub const LOG: u64 = 375;
    pub const LOG_TOPIC: u64 = 375;
    pub const LOG_DATA: u64 = 8;
}

/// Gas meter for tracking gas usage during execution
#[derive(Debug, Clone)]
pub struct GasMeter {
    gas_limit: u64,
    gas_used: u64,
    gas_refund: u64,
}

impl GasMeter {
    pub fn new(gas_limit: u64) -> Self {
        Self {
            gas_limit,
            gas_used: 0,
            gas_refund: 0,
        }
    }

    /// Consume gas for an operation
    pub fn consume(&mut self, amount: u64) -> Result<()> {
        if self.gas_used + amount > self.gas_limit {
            return Err(anyhow!("Out of gas: used {} + {} > limit {}", 
                self.gas_used, amount, self.gas_limit));
        }
        self.gas_used += amount;
        Ok(())
    }

    /// Add gas refund (e.g., from SSTORE clearing)
    pub fn refund(&mut self, amount: u64) {
        self.gas_refund += amount;
    }

    /// Get remaining gas
    pub fn remaining(&self) -> u64 {
        self.gas_limit.saturating_sub(self.gas_used)
    }

    /// Get total gas used
    pub fn used(&self) -> u64 {
        self.gas_used
    }

    /// Get gas refund
    pub fn get_refund(&self) -> u64 {
        self.gas_refund
    }

    /// Calculate final gas cost after refunds
    pub fn finalize(&self) -> u64 {
        // Refund is capped at 50% of gas used (EIP-3529)
        let max_refund = self.gas_used / 2;
        let actual_refund = self.gas_refund.min(max_refund);
        self.gas_used.saturating_sub(actual_refund)
    }
}

/// Calculate base fee for next block (EIP-1559)
pub fn calculate_next_base_fee(
    parent_gas_used: u64,
    parent_gas_limit: u64,
    parent_base_fee: u64,
) -> u64 {
    const TARGET_GAS_USED: f64 = 0.5; // Target 50% block utilization
    const BASE_FEE_MAX_CHANGE_DENOMINATOR: u64 = 8;

    if parent_gas_limit == 0 {
        return parent_base_fee;
    }

    let gas_used_ratio = parent_gas_used as f64 / parent_gas_limit as f64;
    
    if gas_used_ratio > TARGET_GAS_USED {
        // Increase base fee
        let delta = parent_base_fee / BASE_FEE_MAX_CHANGE_DENOMINATOR;
        let increase = ((gas_used_ratio - TARGET_GAS_USED) * 2.0 * delta as f64) as u64;
        parent_base_fee + increase.max(1)
    } else if gas_used_ratio < TARGET_GAS_USED {
        // Decrease base fee
        let delta = parent_base_fee / BASE_FEE_MAX_CHANGE_DENOMINATOR;
        let decrease = ((TARGET_GAS_USED - gas_used_ratio) * 2.0 * delta as f64) as u64;
        parent_base_fee.saturating_sub(decrease)
    } else {
        parent_base_fee
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_meter_basic() {
        let mut meter = GasMeter::new(100);
        
        assert!(meter.consume(50).is_ok());
        assert_eq!(meter.used(), 50);
        assert_eq!(meter.remaining(), 50);
        
        assert!(meter.consume(50).is_ok());
        assert_eq!(meter.used(), 100);
        assert_eq!(meter.remaining(), 0);
        
        // Should fail - out of gas
        assert!(meter.consume(1).is_err());
    }

    #[test]
    fn test_gas_refund() {
        let mut meter = GasMeter::new(100);
        meter.consume(80).unwrap();
        meter.refund(20);
        
        // Refund capped at 50% of gas used
        // Used: 80. Max refund: 40. Actual refund: 20.
        // Final: 80 - 20 = 60.
        assert_eq!(meter.finalize(), 60);
    }

    #[test]
    fn test_base_fee_calculation() {
        // Block half full - base fee stays same
        let base_fee = calculate_next_base_fee(5_000_000, 10_000_000, 1_000_000_000);
        assert_eq!(base_fee, 1_000_000_000);
        
        // Block more than half full - base fee increases
        let base_fee = calculate_next_base_fee(8_000_000, 10_000_000, 1_000_000_000);
        assert!(base_fee > 1_000_000_000);
        
        // Block less than half full - base fee decreases
        let base_fee = calculate_next_base_fee(2_000_000, 10_000_000, 1_000_000_000);
        assert!(base_fee < 1_000_000_000);
    }
}
