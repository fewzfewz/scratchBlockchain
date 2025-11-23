use common::types::Transaction;
use node::test_utils;
use std::time::Duration;
use tokio::time::sleep;
use rand::Rng;

#[tokio::test]
async fn test_chaos_random_disconnects() {
    // Setup 5 nodes for a more robust network
    let mut nodes = Vec::new();
    for i in 0..5 {
        let (node, _) = test_utils::create_test_node(9960 + i, 30360 + i).await;
        nodes.push(node);
    }

    // Fully connect mesh
    for i in 0..5 {
        for j in 0..5 {
            if i != j {
                nodes[i].connect_peer(nodes[j].local_peer_id(), nodes[j].listen_addr()).await;
            }
        }
    }

    sleep(Duration::from_secs(2)).await;

    // Start chaos: Randomly disconnect peers for 10 seconds
    let start_height = nodes[0].get_block_height().await;
    
    let chaos_duration = Duration::from_secs(10);
    let start_time = std::time::Instant::now();

    while start_time.elapsed() < chaos_duration {
        // Randomly pick a node and disconnect a random peer
        let node_idx = rand::thread_rng().gen_range(0..5);
        let peer_idx = rand::thread_rng().gen_range(0..5);
        
        if node_idx != peer_idx {
            // Simulate disconnect (log it)
            nodes[node_idx].disconnect_peer(nodes[peer_idx].local_peer_id()).await;
            
            // Submit a tx to ensure activity
            let _ = nodes[node_idx].submit_transaction(test_utils::create_dummy_transaction()).await;
        }
        
        sleep(Duration::from_millis(500)).await;
    }

    // Reconnect everything
    for i in 0..5 {
        for j in 0..5 {
            if i != j {
                nodes[i].connect_peer(nodes[j].local_peer_id(), nodes[j].listen_addr()).await;
            }
        }
    }

    sleep(Duration::from_secs(5)).await;

    // Verify chain progress despite chaos
    let end_height = nodes[0].get_block_height().await;
    assert!(end_height > start_height, "Chain should advance despite chaos");
    
    println!("Chain advanced from {} to {} under chaos", start_height, end_height);
}
