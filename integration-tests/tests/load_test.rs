use common::types::Transaction;
use node::test_utils;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::test]
async fn test_load_performance() {
    // Setup 3 nodes
    let (node1, _) = test_utils::create_test_node(9953, 30353).await;
    let (node2, _) = test_utils::create_test_node(9954, 30354).await;
    let (node3, _) = test_utils::create_test_node(9955, 30355).await;

    // Connect nodes
    node1.connect_peer(node2.local_peer_id(), node2.listen_addr()).await;
    node1.connect_peer(node3.local_peer_id(), node3.listen_addr()).await;
    node2.connect_peer(node3.local_peer_id(), node3.listen_addr()).await;

    sleep(Duration::from_secs(2)).await;

    // Generate 100 transactions
    let mut txs = Vec::new();
    for i in 0..100 {
        let mut tx = test_utils::create_dummy_transaction();
        tx.nonce = i;
        txs.push(tx);
    }

    let start_time = Instant::now();

    // Submit transactions to all nodes (load balancing)
    for (i, tx) in txs.iter().enumerate() {
        match i % 3 {
            0 => node1.submit_transaction(tx.clone()).await.unwrap(),
            1 => node2.submit_transaction(tx.clone()).await.unwrap(),
            2 => node3.submit_transaction(tx.clone()).await.unwrap(),
            _ => unreachable!(),
        }
    }

    // Wait for processing
    // Assuming 100 TPS, this should take ~1s + block time
    // Let's give it 10s
    sleep(Duration::from_secs(10)).await;

    let elapsed = start_time.elapsed();
    println!("Submitted 100 txs in {:?}", elapsed);

    // Verify all nodes processed transactions
    let height1 = node1.get_block_height().await;
    assert!(height1 > 0, "Should have produced blocks");

    // Check total transactions in blocks
    let mut total_txs = 0;
    for h in 1..=height1 {
        if let Ok(block) = node1.block_store.get_block_by_height(h) {
            if let Some(block) = block {
                total_txs += block.transactions.len();
            }
        }
    }

    println!("Total transactions processed: {}", total_txs);
    assert!(total_txs > 0, "Should have processed transactions");
    
    // Calculate TPS
    let tps = total_txs as f64 / elapsed.as_secs_f64();
    println!("TPS: {:.2}", tps);
}
