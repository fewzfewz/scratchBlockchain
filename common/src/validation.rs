use anyhow::{anyhow, Result};
use crate::types::{Address, Transaction, Account};
use std::collections::HashMap;

/// Comprehensive transaction validation
pub struct TransactionValidator {
    chain_id: u64,
}

impl TransactionValidator {
    pub fn new(chain_id: u64) -> Self {
        Self { chain_id }
    }

    /// Validate transaction before accepting into mempool
    pub fn validate(
        &self,
        tx: &Transaction,
        state: &HashMap<Address, Account>,
    ) -> Result<()> {
        // 1. Chain ID validation (replay protection)
        if let Some(tx_chain_id) = tx.chain_id {
            if tx_chain_id != self.chain_id {
                return Err(anyhow!("Invalid chain ID: expected {}, got {}", 
                    self.chain_id, tx_chain_id));
            }
        }

        // 2. Gas validation
        if tx.gas_limit == 0 {
            return Err(anyhow!("Gas limit cannot be zero"));
        }

        if tx.gas_limit > 30_000_000 {
            return Err(anyhow!("Gas limit too high: {}", tx.gas_limit));
        }

        if tx.max_fee_per_gas == 0 {
            return Err(anyhow!("Max fee per gas cannot be zero"));
        }

        if tx.max_priority_fee_per_gas > tx.max_fee_per_gas {
            return Err(anyhow!(
                "Max priority fee ({}) exceeds max fee ({})",
                tx.max_priority_fee_per_gas,
                tx.max_fee_per_gas
            ));
        }

        // 3. Sender account validation
        let account = state.get(&tx.sender)
            .ok_or_else(|| anyhow!("Sender account not found"))?;

        // 4. Nonce validation
        if tx.nonce != account.nonce {
            return Err(anyhow!(
                "Invalid nonce: expected {}, got {}",
                account.nonce,
                tx.nonce
            ));
        }

        // 5. Balance validation
        let max_cost = (tx.gas_limit * tx.max_fee_per_gas + tx.value) as u128;
        if account.balance < max_cost {
            return Err(anyhow!(
                "Insufficient balance: has {}, needs {}",
                account.balance,
                max_cost
            ));
        }

        // 6. Signature validation
        // Note: Signature verification is done separately with public key

        Ok(())
    }

    /// Validate transaction for inclusion in block (additional checks)
    pub fn validate_for_block(
        &self,
        tx: &Transaction,
        base_fee: u64,
    ) -> Result<()> {
        // Check if transaction can pay base fee
        if tx.max_fee_per_gas < base_fee {
            return Err(anyhow!(
                "Max fee per gas ({}) below base fee ({})",
                tx.max_fee_per_gas,
                base_fee
            ));
        }

        Ok(())
    }

    /// Calculate effective gas price for transaction
    pub fn effective_gas_price(tx: &Transaction, base_fee: u64) -> u64 {
        let max_priority_fee = tx.max_priority_fee_per_gas;
        let max_fee = tx.max_fee_per_gas;
        
        // Effective priority fee is min of max_priority_fee and (max_fee - base_fee)
        let priority_fee = max_priority_fee.min(max_fee.saturating_sub(base_fee));
        
        base_fee + priority_fee
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Account;

    fn create_test_account(balance: u64, nonce: u64) -> Account {
        Account {
            balance: balance as u128,
            nonce,
        }
    }

    fn create_test_tx(nonce: u64, gas_limit: u64, max_fee: u64, value: u64) -> Transaction {
        Transaction {
            sender: [1; 20],
            nonce,
            payload: vec![],
            signature: vec![0; 64],
            gas_limit,
            max_fee_per_gas: max_fee,
            max_priority_fee_per_gas: max_fee / 10,
            chain_id: Some(1),
            to: Some([2; 20]),
            value,
        }
    }

    #[test]
    fn test_valid_transaction() {
        let validator = TransactionValidator::new(1);
        let mut state = HashMap::new();
        state.insert([1; 20], create_test_account(100_000_000_000, 0));

        let tx = create_test_tx(0, 21_000, 1_000_000, 100);
        assert!(validator.validate(&tx, &state).is_ok());
    }

    #[test]
    fn test_invalid_nonce() {
        let validator = TransactionValidator::new(1);
        let mut state = HashMap::new();
        state.insert([1; 20], create_test_account(1_000_000_000, 5));

        let tx = create_test_tx(0, 21_000, 1_000_000, 100);
        assert!(validator.validate(&tx, &state).is_err());
    }

    #[test]
    fn test_insufficient_balance() {
        let validator = TransactionValidator::new(1);
        let mut state = HashMap::new();
        state.insert([1; 20], create_test_account(1000, 0));

        let tx = create_test_tx(0, 21_000, 1_000_000, 100);
        assert!(validator.validate(&tx, &state).is_err());
    }

    #[test]
    fn test_invalid_chain_id() {
        let validator = TransactionValidator::new(1);
        let mut state = HashMap::new();
        state.insert([1; 20], create_test_account(1_000_000_000, 0));

        let mut tx = create_test_tx(0, 21_000, 1_000_000, 100);
        tx.chain_id = Some(999);
        assert!(validator.validate(&tx, &state).is_err());
    }

    #[test]
    fn test_effective_gas_price() {
        let tx = create_test_tx(0, 21_000, 1_000_000_000, 0);
        let base_fee = 900_000_000;
        
        let effective = TransactionValidator::effective_gas_price(&tx, base_fee);
        // Should be base_fee + min(priority_fee, max_fee - base_fee)
        // = 900_000_000 + min(100_000_000, 100_000_000) = 1_000_000_000
        assert_eq!(effective, 1_000_000_000);
    }
}
