use anyhow::Result;
use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{Address, Bytes, CreateScheme, ExecutionResult, Output, TransactTo, U256},
    DatabaseCommit, EVM,
};
use std::str::FromStr;

pub struct EvmExecutor {
    // For now, we use an ephemeral DB for each execution or shared cache
    // In a real node, this would wrap our Storage trait
    db: CacheDB<EmptyDB>,
}

impl Default for EvmExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl EvmExecutor {
    pub fn new() -> Self {
        Self {
            db: CacheDB::new(EmptyDB::default()),
        }
    }

    pub fn execute_transaction(
        &mut self,
        caller: &str,
        to: Option<&str>,
        value: u64,
        data: &[u8],
    ) -> Result<Vec<u8>> {
        let mut evm = EVM::new();
        evm.database(self.db.clone());

        let caller_addr =
            Address::from_str(caller).map_err(|e| anyhow::anyhow!("Invalid caller: {}", e))?;

        evm.env.tx.caller = caller_addr;
        evm.env.tx.value = U256::from(value);
        evm.env.tx.data = Bytes::copy_from_slice(data);
        evm.env.tx.gas_limit = 1_000_000; // Mock gas limit

        if let Some(to_addr) = to {
            let to_addr =
                Address::from_str(to_addr).map_err(|e| anyhow::anyhow!("Invalid to: {}", e))?;
            evm.env.tx.transact_to = TransactTo::Call(to_addr);
        } else {
            evm.env.tx.transact_to = TransactTo::Create(CreateScheme::Create); // Create contract
        }

        let result_and_state = evm
            .transact()
            .map_err(|e| anyhow::anyhow!("EVM execution failed: {:?}", e))?;

        // Commit state changes
        self.db.commit(result_and_state.state);

        match result_and_state.result {
            ExecutionResult::Success { output, .. } => match output {
                Output::Call(value) => Ok(value.to_vec()),
                Output::Create(value, address) => {
                    println!("Contract created at: {:?}", address);
                    Ok(value.to_vec())
                }
            },
            ExecutionResult::Revert { output, .. } => {
                Err(anyhow::anyhow!("Reverted: {:?}", output))
            }
            ExecutionResult::Halt { reason, .. } => Err(anyhow::anyhow!("Halted: {:?}", reason)),
        }
    }
}
