use common::types::{Block, Transaction};
use node::test_utils;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_multi_node_consensus() {
    // Setup 3 nodes
    let (node1, mut rx1) = test_utils::create_test_node(9933, 30333).await;
    let (node2, mut rx2) = test_utils::create_test_node(9934, 30334).await;
    let (node3, mut rx3) = test_utils::create_test_node(9935, 30335).await;

    // Connect nodes
    node1.connect_peer(node2.local_peer_id(), node2.listen_addr()).await;
    node1.connect_peer(node3.local_peer_id(), node3.listen_addr()).await;
    node2.connect_peer(node3.local_peer_id(), node3.listen_addr()).await;

    // Wait for connections
    sleep(Duration::from_secs(2)).await;

    // Submit transaction to Node 1
    let tx = test_utils::create_dummy_transaction();
    node1.submit_transaction(tx.clone()).await.unwrap();

    // Wait for block production and propagation
    sleep(Duration::from_secs(5)).await;

    // Verify all nodes have the block
    let height1 = node1.get_block_height().await;
    let height2 = node2.get_block_height().await;
    let height3 = node3.get_block_height().await;

    assert!(height1 > 0, "Node 1 should have produced blocks");
    assert_eq!(height1, height2, "Node 2 should sync with Node 1");
    assert_eq!(height1, height3, "Node 3 should sync with Node 1");

    // Verify transaction is in the block
    let block = node1.get_latest_block().await.unwrap();
    assert!(block.transactions.contains(&tx), "Transaction should be in the block");
}

#[tokio::test]
async fn test_network_partition_recovery() {
    // Setup 3 nodes
    let (node1, _) = test_utils::create_test_node(9943, 30343).await;
    let (node2, _) = test_utils::create_test_node(9944, 30344).await;
    let (node3, _) = test_utils::create_test_node(9945, 30345).await;

    // Connect all nodes initially
    node1.connect_peer(node2.local_peer_id(), node2.listen_addr()).await;
    node1.connect_peer(node3.local_peer_id(), node3.listen_addr()).await;
    node2.connect_peer(node3.local_peer_id(), node3.listen_addr()).await;

    sleep(Duration::from_secs(2)).await;

    // Simulate partition: Disconnect Node 3
    node3.disconnect_peer(node1.local_peer_id()).await;
    node3.disconnect_peer(node2.local_peer_id()).await;

    // Nodes 1 & 2 continue producing blocks
    sleep(Duration::from_secs(5)).await;
    let height_main = node1.get_block_height().await;
    let height_partitioned = node3.get_block_height().await;

    assert!(height_main > height_partitioned, "Main partition should advance faster");

    // Heal partition
    node3.connect_peer(node1.local_peer_id(), node1.listen_addr()).await;
    node3.connect_peer(node2.local_peer_id(), node2.listen_addr()).await;

    // Wait for sync
    sleep(Duration::from_secs(5)).await;

    let height_healed = node3.get_block_height().await;
    assert!(height_healed >= height_main, "Node 3 should catch up after partition heals");
}
