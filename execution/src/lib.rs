pub mod evm;
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
