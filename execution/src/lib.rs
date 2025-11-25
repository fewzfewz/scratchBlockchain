pub mod evm;
pub mod account_abstraction;
pub mod gas;
pub use evm::EvmExecutor;

use anyhow::Result;
use wasmtime::{Engine, Linker, Module, Store};

pub struct WasmExecutor {
    engine: Engine,
}

impl WasmExecutor {
    pub fn new() -> Result<Self> {
        let engine = Engine::default();
        Ok(Self { engine })
    }

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

pub fn init() {
    println!("Execution initialized (use WasmExecutor::new)");
}

use rayon::prelude::*;

pub struct ParallelExecutor;

impl Default for ParallelExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ParallelExecutor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute_block_parallel(&self, transactions: &[Vec<u8>]) -> Result<()> {
        // In a real implementation, we would:
        // 1. Analyze dependencies (read/write sets)
        // 2. Group non-conflicting transactions
        // 3. Execute groups in parallel

        // For now, we just iterate in parallel assuming no conflicts (unsafe but demonstrates the pattern)
        transactions.par_iter().for_each(|_tx| {
            // Mock execution: spin a bit or call WasmExecutor
            // println!("Executing tx in thread {:?}", std::thread::current().id());
        });

        Ok(())
    }
}

// Executor trait
use common::types::{Block, Transaction, Account, Address};
use std::collections::HashMap;

pub trait Executor {
    fn execute_block(&self, block: &Block, state: &mut HashMap<Address, Account>) -> Result<u64>;
}

pub struct NativeExecutor;

impl Default for NativeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeExecutor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute_transaction(&self, tx: &Transaction, state: &mut HashMap<Address, Account>) -> Result<u64> {
        // 1. Initialize Gas Meter
        let mut gas_meter = crate::gas::GasMeter::new(tx.gas_limit);
        
        // 2. Charge Base Fee
        gas_meter.consume(crate::gas::GasCosts::TRANSACTION)?;
        
        // 3. Charge Payload Gas (simplified: 8 gas per byte)
        gas_meter.consume(tx.payload.len() as u64 * 8)?;

        // 4. Verify signature
        // For MVP: require public key in payload prefix if not recoverable
        // Format: [pubkey(32 bytes)]...
        if tx.payload.len() < 32 {
            return Err(anyhow::anyhow!("Payload too short - missing public key"));
        }
        
        let mut public_key = [0u8; 32];
        public_key.copy_from_slice(&tx.payload[0..32]);
        
        if !tx.verify(&public_key) {
            return Err(anyhow::anyhow!("Invalid signature"));
        }

        // 5. Get sender account
        let sender_account = state.get_mut(&tx.sender)
            .ok_or_else(|| anyhow::anyhow!("Sender account not found"))?;

        // 6. Check nonce
        if sender_account.nonce != tx.nonce {
            return Err(anyhow::anyhow!("Invalid nonce"));
        }

        // 7. Check balance for max gas cost + value
        let max_gas_cost = tx.gas_limit as u128 * tx.max_fee_per_gas as u128;
        let total_cost = max_gas_cost + tx.value as u128;
        
        if sender_account.balance < total_cost {
            return Err(anyhow::anyhow!("Insufficient balance"));
        }

        // 8. Execute Transfer
        if let Some(_to) = tx.to {
            // Charge value transfer gas if value > 0
            if tx.value > 0 {
                // gas_meter.consume(crate::gas::GasCosts::CALL)?; // Or specific transfer cost
            }

            // Deduct value from sender
            sender_account.balance -= tx.value as u128;
            
            // Add to recipient
            // We need to re-borrow state to get recipient, which is tricky with mutable borrow of sender
            // So we'll do it after releasing sender borrow or use a different approach
            // For now, let's just update sender nonce and balance, then update recipient
        }

        // 6. Execute Transfer (Update State)
        // Deduct from sender
        if let Some(sender_account) = state.get_mut(&tx.sender) {
            sender_account.balance -= tx.value as u128;
            
            // Calculate actual gas fee
            let gas_used = gas_meter.used();
            let gas_fee = gas_used as u128 * tx.max_fee_per_gas as u128;
            sender_account.balance -= gas_fee;
            sender_account.nonce += 1;
        }

        // Add to recipient
        if let Some(to) = tx.to {
            if tx.value > 0 {
                state.entry(to)
                    .or_default()
                    .balance += tx.value as u128;
            }
        }
        
        Ok(gas_meter.used())
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

#[cfg(test)]
mod tests {
    use super::*;
    use common::types::{Block, Header, Transaction, Account};
    use std::collections::HashMap;
    use ed25519_dalek::{SigningKey, Signer};

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
        state.insert(sender, Account { nonce: 0, balance: 100_000_000_000_000 });
        
        // Create a signing key for testing
        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        
        let tx = create_signed_transaction(sender, 0, recipient, 500, &signing_key);
        
        let block = Block {
            header: Header::new([0; 32], 1),
            extrinsics: vec![tx],
        };
        
        executor.execute_block(&block, &mut state).unwrap();
        
        let sender_account = state.get(&sender).unwrap();
        assert!(sender_account.balance < 100_000_000_000_000); // Gas fee paid
        assert_eq!(sender_account.nonce, 1);
        assert_eq!(state.get(&recipient).unwrap().balance, 500);
    }

    #[test]
    fn test_native_executor_insufficient_balance() {
        let executor = NativeExecutor::new();
        
        let sender = [1u8; 20];
        let recipient = [2u8; 20];
        let mut state = HashMap::new();
        state.insert(sender, Account { nonce: 0, balance: 100 });
        
        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
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
        state.insert(sender, Account { nonce: 0, balance: 1000 });
        
        // Create transaction with wrong signature
        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let mut tx = create_signed_transaction(sender, 0, recipient, 500, &signing_key);
        
        // Corrupt the signature
        tx.signature[0] ^= 0xFF;
        
        let block = Block {
            header: Header::new([0; 32], 1),
            extrinsics: vec![tx],
        };
        
        assert!(executor.execute_block(&block, &mut state).is_err());
    }
}
