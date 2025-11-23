use mempool::{Mempool, MempoolConfig};
use common::types::Transaction;

#[test]
fn test_mempool_capacity_integration() {
    let config = MempoolConfig {
        max_capacity: 10,
        max_per_sender: 10,
        min_fee_per_gas: 0,
    };
    let mempool = Mempool::new(config);
    
    // Fill mempool
    for i in 0..10 {
        let mut signature = vec![0; 64];
        signature[0] = i as u8; // Unique signature
        
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
        mempool.add_transaction(tx).unwrap();
    }
    
    assert_eq!(mempool.size(), 10);
    
    // Try to add one more
    let tx = Transaction {
        sender: [0; 20],
        nonce: 10,
        payload: vec![],
        signature: vec![1; 64],
        gas_limit: 21000,
        max_fee_per_gas: 100,
        max_priority_fee_per_gas: 10,
        chain_id: Some(1),
        to: None,
        value: 0,
    };
    
    assert!(mempool.add_transaction(tx).is_err());
}
