use common::types::{Block, Transaction};
use consensus::{EnhancedConsensus, ValidatorInfo, FinalityGadget};
use common::crypto::SigningKey;
use mempool::{Mempool, MempoolConfig};
use network::{NetworkCommand, NetworkEvent, NetworkService};
use node::block_producer::BlockProducer;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use storage::BlockStore;

fn create_test_components() -> (
    Arc<Mempool>,
    Arc<Mutex<EnhancedConsensus>>,
    SigningKey,
    Arc<storage::StateStore>,
    Arc<BlockStore>,
    Arc<Mutex<FinalityGadget>>,
) {
    let mempool = Arc::new(Mempool::new(MempoolConfig::default()));
    
    let signing_key = SigningKey::from_bytes(&[0u8; 32]).unwrap(); // Deterministic key for tests
    let public_key = signing_key.public_key();
    let validators = vec![ValidatorInfo {
        public_key: public_key.clone(),
        stake: 100,
        slashed: false,
    }];
    
    let consensus = Arc::new(Mutex::new(EnhancedConsensus::new(validators.clone())));
    let finality_gadget = Arc::new(Mutex::new(FinalityGadget::new(validators)));
    
    // Create temporary state store for testing
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("integration_test_state_db");
    let state_store = Arc::new(storage::StateStore::new(path.to_str().unwrap()).unwrap());
    
    // Initialize genesis state
    let genesis_config = common::types::GenesisConfig::default();
    state_store.initialize_genesis(&genesis_config).unwrap();

    // Create temporary block store for testing
    let dir2 = tempfile::tempdir().unwrap();
    let path2 = dir2.path().join("integration_test_block_db");
    let block_store = Arc::new(BlockStore::new(path2.to_str().unwrap()).unwrap());
    
    (mempool, consensus, signing_key, state_store, block_store, finality_gadget)
}

fn create_valid_transaction(nonce: u64, signing_key: &SigningKey) -> Transaction {
    let public_key = signing_key.public_key();
    let recipient = [2u8; 20];
    let amount = 100u128;

    // Build payload: [pubkey(32)][recipient(20)][amount(16)]
    let mut payload = Vec::new();
    payload.extend_from_slice(&public_key);
    payload.extend_from_slice(&recipient);
    payload.extend_from_slice(&amount.to_le_bytes());

    let mut tx = Transaction {
        sender: [1u8; 20], // Must match genesis account if checking balance, but for propagation it doesn't matter
        nonce,
        payload,
        signature: vec![],
        gas_limit: 30_000,
        max_fee_per_gas: 2_000_000_000,
        max_priority_fee_per_gas: 1_000_000_000,
        chain_id: Some(1),
        to: Some(recipient),
        value: amount as u64,
    };

    let message = tx.hash();
    let signature = signing_key.sign(&message);
    tx.signature = signature;
    
    tx
}

#[tokio::test]
async fn test_p2p_transaction_propagation() {
    // 1. Start Node 1
    let (node1, cmd_sender1, mut event_receiver1) = NetworkService::new().unwrap();
    tokio::spawn(node1.run());
    
    cmd_sender1.send(NetworkCommand::StartListening("/ip4/127.0.0.1/tcp/0".parse().unwrap())).await.unwrap();
    
    // Get Node 1 address
    let addr1 = loop {
        if let Some(NetworkEvent::ListeningOn(addr)) = event_receiver1.recv().await {
            break addr;
        }
    };
    println!("Node 1 listening on {:?}", addr1);

    // 2. Start Node 2
    let (node2, cmd_sender2, mut event_receiver2) = NetworkService::new().unwrap();
    tokio::spawn(node2.run());
    
    cmd_sender2.send(NetworkCommand::StartListening("/ip4/127.0.0.1/tcp/0".parse().unwrap())).await.unwrap();
    
    // Get Node 2 address (just to ensure it's up)
    let _addr2 = loop {
        if let Some(NetworkEvent::ListeningOn(addr)) = event_receiver2.recv().await {
            break addr;
        }
    };

    // 3. Connect Node 2 to Node 1
    cmd_sender2.send(NetworkCommand::Dial(addr1.clone())).await.unwrap();
    
    // Wait for connection (simplified: just wait a bit)
    tokio::time::sleep(Duration::from_secs(1)).await;

    // 4. Broadcast transaction from Node 1
    let signing_key = SigningKey::generate();
    let tx = create_valid_transaction(1, &signing_key);
    
    cmd_sender1.send(NetworkCommand::BroadcastTransaction(tx.clone())).await.unwrap();

    // 5. Verify Node 2 receives it
    let received = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(NetworkEvent::TransactionReceived(received_tx)) = event_receiver2.recv().await {
                return Some(received_tx);
            }
        }
    }).await;

    match received {
        Ok(Some(received_tx)) => {
            assert_eq!(received_tx.hash(), tx.hash());
            println!("Transaction propagated successfully!");
        }
        _ => panic!("Timed out waiting for transaction propagation"),
    }
}

#[tokio::test]
async fn test_block_production_success() {
    let (mempool, consensus, signing_key, state_store, block_store, finality_gadget) = create_test_components();
    
    // Setup genesis account with balance
    // The 'create_valid_transaction' uses sender [1u8; 20]
    // We need to make sure this account exists in state_store with balance
    let sender_addr = [1u8; 20];
    let account = common::types::Account {
        nonce: 1, // tx nonce is 1
        balance: 1_000_000_000_000_000_000,
    };
    state_store.put_account(&sender_addr, &account).unwrap();

    let mut producer = BlockProducer::new(
        mempool.clone(),
        consensus.clone(),
        state_store,
        block_store,
        finality_gadget,
        signing_key.clone()
    );

    // Add valid transaction
    let tx = create_valid_transaction(1, &signing_key);
    mempool.add_transaction(tx).unwrap();

    // Produce block
    let genesis = Block::genesis();
    let result = producer.produce_block(&genesis).await;

    match result {
        Ok(block) => {
            assert_eq!(block.extrinsics.len(), 1);
            println!("Block produced successfully with 1 transaction");
        }
        Err(e) => panic!("Failed to produce block: {}", e),
    }
}
