use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use common::types::{Transaction, Block, Header};
use execution::NativeExecutor;
use mempool::Mempool;
use std::collections::HashMap;

fn benchmark_transaction_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("transaction_execution");
    
    for tx_count in [10, 50, 100, 500].iter() {
        group.benchmark_with_input(BenchmarkId::from_parameter(tx_count), tx_count, |b, &tx_count| {
            let executor = NativeExecutor::new();
            let mut state = HashMap::new();
            
            // Create test transactions
            let transactions: Vec<Transaction> = (0..tx_count)
                .map(|i| {
                    let mut tx = Transaction::test_transaction([i as u8; 20], i as u64);
                    tx.max_fee_per_gas = 2_000_000_000;
                    tx.max_priority_fee_per_gas = 1_000_000_000;
                    tx
                })
                .collect();
            
            b.iter(|| {
                for tx in &transactions {
                    let _ = executor.execute_transaction(black_box(tx), black_box(&mut state));
                }
            });
        });
    }
    
    group.finish();
}

fn benchmark_block_production(c: &mut Criterion) {
    let mut group = c.benchmark_group("block_production");
    
    for tx_count in [10, 50, 100].iter() {
        group.benchmark_with_input(BenchmarkId::from_parameter(tx_count), tx_count, |b, &tx_count| {
            let mempool = Mempool::new(1000, 1_000_000_000);
            
            // Add transactions to mempool
            for i in 0..*tx_count {
                let mut tx = Transaction::test_transaction([i as u8; 20], i as u64);
                tx.max_fee_per_gas = 2_000_000_000;
                tx.max_priority_fee_per_gas = 1_000_000_000;
                let _ = mempool.add_transaction(tx);
            }
            
            b.iter(|| {
                let transactions = mempool.get_transactions(black_box(100));
                let header = Header::new([0; 32], 0);
                let _block = Block::new(header, transactions);
            });
        });
    }
    
    group.finish();
}

fn benchmark_mempool_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("mempool");
    
    group.bench_function("add_transaction", |b| {
        let mempool = Mempool::new(10000, 1_000_000_000);
        let mut counter = 0u64;
        
        b.iter(|| {
            let mut tx = Transaction::test_transaction([0; 20], counter);
            tx.max_fee_per_gas = 2_000_000_000;
            tx.max_priority_fee_per_gas = 1_000_000_000;
            let _ = mempool.add_transaction(black_box(tx));
            counter += 1;
        });
    });
    
    group.bench_function("get_transactions", |b| {
        let mempool = Mempool::new(10000, 1_000_000_000);
        
        // Pre-fill mempool
        for i in 0..1000 {
            let mut tx = Transaction::test_transaction([0; 20], i);
            tx.max_fee_per_gas = 2_000_000_000;
            tx.max_priority_fee_per_gas = 1_000_000_000;
            let _ = mempool.add_transaction(tx);
        }
        
        b.iter(|| {
            let _txs = mempool.get_transactions(black_box(100));
        });
    });
    
    group.finish();
}

criterion_group!(benches, benchmark_transaction_execution, benchmark_block_production, benchmark_mempool_operations);
criterion_main!(benches);
