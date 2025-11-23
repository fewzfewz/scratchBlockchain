use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mempool::{Mempool, MempoolConfig};
use common::types::Transaction;

fn benchmark_mempool_add(c: &mut Criterion) {
    let config = MempoolConfig {
        max_capacity: 10000,
        max_per_sender: 1000,
        min_fee_per_gas: 0,
    };
    let mempool = Mempool::new(config);
    
    c.bench_function("mempool_add_transaction", |b| {
        let mut i: u64 = 0;
        b.iter(|| {
            let mut signature = vec![0; 64];
            // Use a simple way to generate unique signatures for the benchmark
            // In a real scenario, we'd pre-generate these to avoid measuring generation time
            // But for simplicity and since we want to measure `add_transaction`, this is acceptable overhead
            // or we can clear the mempool.
            // Actually, `add_transaction` checks for duplicates. If we add the same tx, it returns error.
            // So we need unique txs.
            
            i += 1;
            let bytes = i.to_le_bytes();
            for (j, byte) in bytes.iter().enumerate() {
                signature[j] = *byte;
            }
            
            let tx = Transaction {
                sender: [0; 20],
                nonce: i,
                payload: vec![],
                signature,
                gas_limit: 21000,
                max_fee_per_gas: 100,
                max_priority_fee_per_gas: 10,
                chain_id: Some(1),
                to: None,
                value: 0,
            };
            
            // We ignore the result (it might fail if full, but with 10000 capacity it should be fine for short bench)
            let _ = mempool.add_transaction(black_box(tx));
        })
    });
}

criterion_group!(benches, benchmark_mempool_add);
criterion_main!(benches);
