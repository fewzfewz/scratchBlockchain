use anyhow::Result;
use common::types::{Address, Transaction};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// UserOperation for account abstraction (ERC-4337 style)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserOperation {
    /// Sender (smart contract account address)
    pub sender: Address,
    /// Nonce for replay protection
    pub nonce: u64,
    /// Initialization code (for account deployment)
    pub init_code: Vec<u8>,
    /// Call data to execute
    pub call_data: Vec<u8>,
    /// Gas limit for validation
    pub verification_gas_limit: u64,
    /// Gas limit for execution
    pub call_gas_limit: u64,
    /// Gas price
    pub max_fee_per_gas: u128,
    /// Priority fee
    pub max_priority_fee_per_gas: u128,
    /// Paymaster address (optional, for gas sponsorship)
    pub paymaster: Option<Address>,
    /// Paymaster data
    pub paymaster_data: Vec<u8>,
    /// Signature for validation
    pub signature: Vec<u8>,
}

impl UserOperation {
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.sender);
        hasher.update(self.nonce.to_le_bytes());
        hasher.update(&self.init_code);
        hasher.update(&self.call_data);
        hasher.update(self.verification_gas_limit.to_le_bytes());
        hasher.update(self.call_gas_limit.to_le_bytes());
        hasher.update(self.max_fee_per_gas.to_le_bytes());
        hasher.update(self.max_priority_fee_per_gas.to_le_bytes());
        if let Some(paymaster) = &self.paymaster {
            hasher.update(paymaster);
        }
        hasher.update(&self.paymaster_data);
        hasher.finalize().into()
    }

    /// Convert to a standard transaction (for execution)
    pub fn to_transaction(&self) -> Transaction {
        Transaction {
            sender: self.sender,
            nonce: self.nonce,
            payload: self.call_data.clone(),
            signature: self.signature.clone(),
            gas_limit: self.call_gas_limit + self.verification_gas_limit,
            max_fee_per_gas: 1_000_000_000, // Default 1 Gwei
            max_priority_fee_per_gas: 100_000_000, // Default 0.1 Gwei
            chain_id: Some(1), // Default chain ID
            to: None, // Account abstraction doesn't have explicit 'to'
            value: 0, // Value is in call_data
        }
    }
}

/// Bundler for aggregating user operations
pub struct Bundler {
    /// Pending user operations
    operations: Vec<UserOperation>,
    /// Maximum operations per bundle
    max_bundle_size: usize,
}

impl Bundler {
    pub fn new(max_bundle_size: usize) -> Self {
        Self {
            operations: Vec::new(),
            max_bundle_size,
        }
    }

    /// Add a user operation to the bundle
    pub fn add_operation(&mut self, op: UserOperation) -> Result<()> {
        if self.operations.len() >= self.max_bundle_size {
            return Err(anyhow::anyhow!("Bundle is full"));
        }

        // Check for duplicate nonce from same sender
        if self
            .operations
            .iter()
            .any(|existing| existing.sender == op.sender && existing.nonce == op.nonce)
        {
            return Err(anyhow::anyhow!("Duplicate operation"));
        }

        self.operations.push(op);
        Ok(())
    }

    /// Get operations ready for bundling
    pub fn get_bundle(&mut self, limit: usize) -> Vec<UserOperation> {
        let count = std::cmp::min(limit, self.operations.len());
        self.operations.drain(..count).collect()
    }

    /// Remove operations that have been included
    pub fn remove_operations(&mut self, ops: &[UserOperation]) {
        let op_hashes: Vec<[u8; 32]> = ops.iter().map(|op| op.hash()).collect();
        self.operations
            .retain(|op| !op_hashes.contains(&op.hash()));
    }

    /// Get number of pending operations
    pub fn size(&self) -> usize {
        self.operations.len()
    }
}

/// Account abstraction executor
pub struct AccountAbstractionExecutor {
    bundler: Bundler,
}

impl AccountAbstractionExecutor {
    pub fn new(max_bundle_size: usize) -> Self {
        Self {
            bundler: Bundler::new(max_bundle_size),
        }
    }

    /// Validate a user operation
    pub fn validate_operation(&self, op: &UserOperation) -> Result<()> {
        // Basic validation
        if op.signature.is_empty() {
            return Err(anyhow::anyhow!("Empty signature"));
        }

        if op.call_data.is_empty() && op.init_code.is_empty() {
            return Err(anyhow::anyhow!("No call data or init code"));
        }

        // Gas validation
        if op.verification_gas_limit == 0 || op.call_gas_limit == 0 {
            return Err(anyhow::anyhow!("Invalid gas limits"));
        }

        // In a real implementation, we would:
        // 1. Simulate the validation function on the account
        // 2. Check paymaster validity if present
        // 3. Verify signature against account's validation logic

        Ok(())
    }

    /// Submit a user operation
    pub fn submit_operation(&mut self, op: UserOperation) -> Result<()> {
        self.validate_operation(&op)?;
        self.bundler.add_operation(op)
    }

    /// Get bundled operations as transactions
    pub fn get_bundled_transactions(&mut self, limit: usize) -> Vec<Transaction> {
        let ops = self.bundler.get_bundle(limit);
        ops.iter().map(|op| op.to_transaction()).collect()
    }

    /// Get bundler size
    pub fn pending_operations(&self) -> usize {
        self.bundler.size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_user_operation(nonce: u64) -> UserOperation {
        UserOperation {
            sender: [1; 20],
            nonce,
            init_code: vec![],
            call_data: vec![1, 2, 3],
            verification_gas_limit: 100000,
            call_gas_limit: 200000,
            max_fee_per_gas: 1000,
            max_priority_fee_per_gas: 100,
            paymaster: None,
            paymaster_data: vec![],
            signature: vec![0; 64],
        }
    }

    #[test]
    fn test_user_operation_hash() {
        let op1 = create_test_user_operation(1);
        let op2 = create_test_user_operation(1);
        let op3 = create_test_user_operation(2);

        assert_eq!(op1.hash(), op2.hash());
        assert_ne!(op1.hash(), op3.hash());
    }

    #[test]
    fn test_bundler_add_operation() {
        let mut bundler = Bundler::new(10);
        let op = create_test_user_operation(1);

        assert!(bundler.add_operation(op).is_ok());
        assert_eq!(bundler.size(), 1);
    }

    #[test]
    fn test_bundler_duplicate_nonce() {
        let mut bundler = Bundler::new(10);
        let op1 = create_test_user_operation(1);
        let op2 = create_test_user_operation(1);

        bundler.add_operation(op1).unwrap();
        assert!(bundler.add_operation(op2).is_err());
    }

    #[test]
    fn test_account_abstraction_executor() {
        let mut executor = AccountAbstractionExecutor::new(10);
        let op = create_test_user_operation(1);

        assert!(executor.submit_operation(op).is_ok());
        assert_eq!(executor.pending_operations(), 1);

        let txs = executor.get_bundled_transactions(10);
        assert_eq!(txs.len(), 1);
        assert_eq!(executor.pending_operations(), 0);
    }

    #[test]
    fn test_validation_empty_signature() {
        let executor = AccountAbstractionExecutor::new(10);
        let mut op = create_test_user_operation(1);
        op.signature = vec![];

        assert!(executor.validate_operation(&op).is_err());
    }

    #[test]
    fn test_to_transaction() {
        let op = create_test_user_operation(1);
        let tx = op.to_transaction();

        assert_eq!(tx.sender, op.sender);
        assert_eq!(tx.nonce, op.nonce);
        assert_eq!(tx.payload, op.call_data);
        assert_eq!(tx.signature, op.signature);
    }
}
