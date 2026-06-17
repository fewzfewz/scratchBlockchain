//! # Execution Module
//!
//! This module handles transaction execution and smart contract execution.
//! It supports:
//! - Native execution (simple transfers)
//! - EVM execution (Ethereum-compatible smart contracts)
//! - WASM execution (future contract language)
//! - Parallel execution (for high throughput)
//! - Account abstraction (ERC-4337 style)
//! - Gas metering (EIP-1559)
//!
//! ## Architecture
//! - `NativeExecutor`: Simple transfer execution (no smart contracts)
//! - `EvmExecutor`: Full EVM with persistent state backend
//! - `WasmExecutor`: WASM-based smart contracts (future)
//! - `ParallelExecutor`: Rayon-based parallel transaction execution

pub mod evm;
pub mod account_abstraction;
pub mod gas;
pub use evm::EvmExecutor;

use anyhow::{anyhow, Result};
use wasmtime::{Engine, Linker, Module, Store};

// ============================================================================
// WASM Executor (for future contract language support)
// ============================================================================

/// WebAssembly executor for smart contracts
/// 
/// This allows running WASM-based contracts (alternative to EVM).
/// Currently a placeholder - will be fully implemented when contract
/// language is finalized.
pub struct WasmExecutor {
    engine: Engine,
}

impl WasmExecutor {
    /// Create a new WASM executor
    pub fn new() -> Result<Self> {
        let engine = Engine::default();
        Ok(Self { engine })
    }

    /// Execute a WASM function
    /// 
    /// # Arguments
    /// * `wasm_binary` - Compiled WASM module bytes
    /// * `func_name` - Name of the function to call
    /// 
    /// # Returns
    /// * `Ok(())` if execution succeeded
    /// * `Err` if execution failed
    pub fn execute(&self, wasm_binary: &[u8], func_name: &str) -> Result<()> {
        let module = Module::new(&self.engine, wasm_binary)?;
        let mut store = Store::new(&self.engine, ());
        let linker = Linker::new(&self.engine);

        let instance = linker.instantiate(&mut store, &module)?;
        let func = instance.get_typed_func::<(), ()>(&mut store, func_name)?;

        func.call(&mut store, ())?;

        Ok(())
    }
}

// ============================================================================
// Parallel Executor (for high-throughput block processing)
// ============================================================================

use rayon::prelude::*;

/// Parallel executor for processing transactions concurrently
/// 
/// This uses Rayon's work-stealing executor to process transactions
/// in parallel when there are no dependencies between them.
/// 
/// ## Safety
/// Only safe to use when transactions have no conflicts (different accounts).
/// In production, you must analyze read/write sets first.
pub struct ParallelExecutor;

impl Default for ParallelExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ParallelExecutor {
    /// Create a new parallel executor
    pub fn new() -> Self {
        Self
    }

    /// Execute a batch of transactions in parallel
    /// 
    /// # Warning
    /// This assumes NO conflicts between transactions. Use only when
    /// you have verified that transactions affect different accounts.
    pub fn execute_block_parallel(&self, transactions: &[Vec<u8>]) -> Result<()> {
        // In a real implementation, we would:
        // 1. Analyze dependencies (read/write sets)
        // 2. Group non-conflicting transactions
        // 3. Execute groups in parallel
        // 4. Serialize conflicting transactions
        
        // For now, we just iterate in parallel assuming no conflicts
        // (unsafe but demonstrates the pattern)
        transactions.par_iter().for_each(|_tx| {
            // Mock execution: spin a bit or call WasmExecutor
            // println!("Executing tx in thread {:?}", std::thread::current().id());
        });

        Ok(())
    }
}

// ============================================================================
// Executor Trait (abstraction over execution backends)
// ============================================================================

use common::types::{Block, Transaction, Account, Address};
use std::collections::HashMap;

/// Core executor trait that all execution backends must implement
pub trait Executor {
    /// Execute an entire block
    /// 
    /// # Arguments
    /// * `block` - The block to execute
    /// * `state` - Mutable reference to current state (will be updated)
    /// 
    /// # Returns
    /// * `Ok(u64)` - Total gas used
    /// * `Err` - Execution error
    fn execute_block(&self, block: &Block, state: &mut HashMap<Address, Account>) -> Result<u64>;
}

// ============================================================================
// Native Executor (simple transfers without smart contracts)
// ============================================================================

/// Native executor for simple value transfers
/// 
/// This executor only handles basic transfers between accounts.
/// It does NOT support smart contracts.
/// Used for testing or simple blockchain configurations.
pub struct NativeExecutor;

impl Default for NativeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeExecutor {
    /// Create a new native executor
    pub fn new() -> Self {
        Self
    }

    /// Execute a single transaction
    /// 
    /// # Steps
    /// 1. Initialize gas meter
    /// 2. Charge base transaction fee
    /// 3. Charge payload gas
    /// 4. Verify signature
    /// 5. Check nonce
    /// 6. Check balance
    /// 7. Execute transfer
    /// 8. Charge gas fee
    /// 
    /// # Returns
    /// * `Ok(u64)` - Gas used
    /// * `Err` - Validation or execution error
    pub fn execute_transaction(
        &self,
        tx: &Transaction,
        state: &mut HashMap<Address, Account>,
    ) -> Result<u64> {
        use ed25519_dalek::{Signature, VerifyingKey};
        
        // 1. Initialize Gas Meter
        let mut gas_meter = crate::gas::GasMeter::new(tx.gas_limit);
        
        // 2. Charge Base Fee
        gas_meter.consume(crate::gas::GasCosts::TRANSACTION)?;
        
        // 3. Charge Payload Gas (simplified: 8 gas per byte)
        gas_meter.consume(tx.payload.len() as u64 * 8)?;

        // 4. Verify signature
        // FIX: Properly verify ed25519 signature using the public key
        if tx.signature.len() != 64 {
            return Err(anyhow!("Invalid signature length"));
        }
        
        let signature = Signature::from_slice(&tx.signature)
            .map_err(|e| anyhow!("Invalid signature: {}", e))?;
        
        // Recover or verify public key
        // For native executor, we assume the sender is correct and just verify
        // In production, you'd have the public key explicitly in the transaction
        let message = tx.hash();
        
        // Try to extract public key from payload or use a known mapping
        // This is simplified - a production system would have proper key recovery
        if tx.payload.len() < 32 {
            return Err(anyhow!("Payload too short - missing public key"));
        }
        
        let mut public_key_bytes = [0u8; 32];
        public_key_bytes.copy_from_slice(&tx.payload[0..32]);
        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
            .map_err(|e| anyhow!("Invalid public key: {}", e))?;
        
        if verifying_key.verify(&message, &signature).is_err() {
            return Err(anyhow!("Invalid signature"));
        }

        // 5. Get sender account
        let sender_account = state.get_mut(&tx.sender)
            .ok_or_else(|| anyhow!("Sender account not found"))?;

        // 6. Check nonce
        if sender_account.nonce != tx.nonce {
            return Err(anyhow!("Invalid nonce: expected {}, got {}", 
                sender_account.nonce, tx.nonce));
        }

        // 7. Check balance for max gas cost + value
        let max_gas_cost = tx.gas_limit as u128 * tx.max_fee_per_gas as u128;
        let total_cost = max_gas_cost + tx.value as u128;
        
        if sender_account.balance < total_cost {
            return Err(anyhow!("Insufficient balance: need {}, have {}", 
                total_cost, sender_account.balance));
        }

        // 8. Execute Transfer
        if let Some(to) = tx.to {
            // Check if recipient exists, create if not (accounts are created on first transfer)
            let recipient = state.entry(to).or_insert(Account::default());
            
            // Deduct from sender (we already have mutable borrow of sender_account)
            // Need to re-borrow because we can't have two mutable borrows at once
            // We'll update after we release the sender borrow
            let sender_balance = sender_account.balance;
            let sender_nonce = sender_account.nonce;
            
            // Release sender borrow by using the values we captured
            // FIX: Update sender after recipient logic
            drop(sender_account); // Explicitly drop to release borrow
            
            // Now update sender (re-borrow)
            let sender = state.get_mut(&tx.sender).unwrap();
            sender.balance = sender_balance - tx.value as u128;
            sender.nonce = sender_nonce + 1;
            
            // Update recipient
            recipient.balance += tx.value as u128;
        } else {
            // Contract creation or other non-transfer operation
            // For native executor, we don't support contract creation
            return Err(anyhow!("Native executor only supports transfers"));
        }
        
        // 9. Charge gas fee
        let gas_used = gas_meter.used();
        let gas_fee = gas_used as u128 * tx.max_fee_per_gas as u128;
        
        let sender = state.get_mut(&tx.sender).unwrap();
        if sender.balance < gas_fee {
            return Err(anyhow!("Insufficient balance for gas fee"));
        }
        sender.balance -= gas_fee;
        
        Ok(gas_used)
    }
}

impl Executor for NativeExecutor {
    fn execute_block(&self, block: &Block, state: &mut HashMap<Address, Account>) -> Result<u64> {
        let mut total_gas_used = 0;
        for tx in &block.extrinsics {
            total_gas_used += self.execute_transaction(tx, state)?;
        }
        Ok(total_gas_used)
    }
}

// ============================================================================
// Module Initialization
// ============================================================================

pub fn init() {
    println!("Execution module initialized");
    println!("  - EVM Executor: available");
    println!("  - Native Executor: available");
    println!("  - WASM Executor: available (placeholder)");
    println!("  - Parallel Executor: available");
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use common::types::{Block, Header, Transaction, Account};
    use ed25519_dalek::{SigningKey, Signer};
    use rand::rngs::OsRng;

    fn create_signed_transaction(
        sender: [u8; 20],
        nonce: u64,
        recipient: [u8; 20],
        amount: u128,
        signing_key: &SigningKey,
    ) -> Transaction {
        let public_key = signing_key.verifying_key().to_bytes();
        
        // Build payload: [pubkey(32)][recipient(20)][amount(16)]
        let mut payload = Vec::new();
        payload.extend_from_slice(&public_key);
        payload.extend_from_slice(&recipient);
        payload.extend_from_slice(&amount.to_le_bytes());
        
        // Create unsigned transaction to get hash
        let mut tx = Transaction {
            sender,
            nonce,
            payload,
            signature: vec![],
            gas_limit: 30_000,
            max_fee_per_gas: 1_000_000_000,
            max_priority_fee_per_gas: 100_000_000,
            chain_id: Some(1),
            to: Some(recipient),
            value: amount as u64,
            from: sender,
            data: vec![],
        };
        
        // Sign the transaction hash
        let message = tx.hash();
        let signature = signing_key.sign(&message);
        tx.signature = signature.to_bytes().to_vec();
        
        tx
    }

    #[test]
    fn test_native_executor_transfer() {
        let executor = NativeExecutor::new();
        
        let sender = [1u8; 20];
        let recipient = [2u8; 20];
        let mut state = HashMap::new();
        state.insert(sender, Account { 
            nonce: 0, 
            balance: 100_000_000_000_000,
            code: vec![],
            storage: HashMap::new(),
        });
        
        // Create a signing key for testing
        let signing_key = SigningKey::generate(&mut OsRng);
        
        let tx = create_signed_transaction(sender, 0, recipient, 500, &signing_key);
        
        let block = Block {
            header: Header::new([0; 32], 1),
            extrinsics: vec![tx],
        };
        
        executor.execute_block(&block, &mut state).unwrap();
        
        let sender_account = state.get(&sender).unwrap();
        // Balance should be reduced by transfer amount + gas fee
        assert!(sender_account.balance < 100_000_000_000_000);
        assert_eq!(sender_account.nonce, 1);
        
        let recipient_account = state.get(&recipient).unwrap();
        assert_eq!(recipient_account.balance, 500);
    }

    #[test]
    fn test_native_executor_insufficient_balance() {
        let executor = NativeExecutor::new();
        
        let sender = [1u8; 20];
        let recipient = [2u8; 20];
        let mut state = HashMap::new();
        state.insert(sender, Account { 
            nonce: 0, 
            balance: 100,
            code: vec![],
            storage: HashMap::new(),
        });
        
        let signing_key = SigningKey::generate(&mut OsRng);
        let tx = create_signed_transaction(sender, 0, recipient, 500, &signing_key);
        
        let block = Block {
            header: Header::new([0; 32], 1),
            extrinsics: vec![tx],
        };
        
        assert!(executor.execute_block(&block, &mut state).is_err());
    }

    #[test]
    fn test_native_executor_invalid_signature() {
        let executor = NativeExecutor::new();
        
        let sender = [1u8; 20];
        let recipient = [2u8; 20];
        let mut state = HashMap::new();
        state.insert(sender, Account { 
            nonce: 0, 
            balance: 1000,
            code: vec![],
            storage: HashMap::new(),
        });
        
        // Create transaction with wrong signature
        let signing_key = SigningKey::generate(&mut OsRng);
        let mut tx = create_signed_transaction(sender, 0, recipient, 500, &signing_key);
        
        // Corrupt the signature
        if !tx.signature.is_empty() {
            tx.signature[0] ^= 0xFF;
        }
        
        let block = Block {
            header: Header::new([0; 32], 1),
            extrinsics: vec![tx],
        };
        
        assert!(executor.execute_block(&block, &mut state).is_err());
    }
}