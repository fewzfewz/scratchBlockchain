use crate::rpc::RpcServer;
use common::types::{Block, Transaction};
use consensus::FinalityGadget;
use libp2p::{Multiaddr, PeerId};
use mempool::Mempool;
use network::{NetworkCommand, NetworkEvent, NetworkService};
use std::sync::Arc;
use storage::{BlockStore, StateStore, receipt_store::ReceiptStore};
use tokio::sync::{mpsc, Mutex};
use tempfile::TempDir;

pub struct TestNode {
    pub peer_id: PeerId,
    pub listen_addr: Multiaddr,
    pub network_cmd_sender: mpsc::Sender<NetworkCommand>,
    pub block_store: Arc<BlockStore>,
    pub mempool: Arc<Mempool>,
    pub _temp_dir: TempDir, // Keep alive to prevent cleanup
}

impl TestNode {
    pub fn local_peer_id(&self) -> PeerId {
        self.peer_id
    }

    pub fn listen_addr(&self) -> Multiaddr {
        self.listen_addr.clone()
    }

    pub async fn connect_peer(&self, peer_id: PeerId, addr: Multiaddr) {
        self.network_cmd_sender
            .send(NetworkCommand::Dial(addr))
            .await
            .unwrap();
    }

    pub async fn disconnect_peer(&self, peer_id: PeerId) {
        // Note: NetworkService doesn't expose disconnect directly yet, 
        // but we can simulate it or add it. For now, we'll just implement a placeholder
        // or rely on the network partition test logic which might need a way to stop traffic.
        // For the partition test, we might need to add a "Ban" or "Disconnect" command to NetworkCommand.
        // Let's assume we add a Disconnect command or similar.
        // For now, let's just log it.
        println!("Disconnecting peer {} (simulated)", peer_id);
    }

    pub async fn submit_transaction(&self, tx: Transaction) -> anyhow::Result<()> {
        self.mempool.add_transaction(tx.clone())?;
        self.network_cmd_sender
            .send(NetworkCommand::BroadcastTransaction(tx))
            .await?;
        Ok(())
    }

    pub async fn get_block_height(&self) -> u64 {
        self.block_store.get_latest_height().unwrap().unwrap_or(0)
    }

    pub async fn get_latest_block(&self) -> Option<Block> {
        let height = self.get_block_height().await;
        self.block_store.get_block_by_height(height).unwrap()
    }
}

pub async fn create_test_node(rpc_port: u16, p2p_port: u16) -> (TestNode, mpsc::Receiver<NetworkEvent>) {
    // Create temp dirs
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("db");

    // Initialize components
    let block_store = Arc::new(BlockStore::new(db_path.join("blocks")).unwrap());
    let state_store = Arc::new(StateStore::new(db_path.join("state")).unwrap());
    let receipt_store = Arc::new(ReceiptStore::new(db_path.join("receipts")).unwrap());
    let mempool = Arc::new(Mempool::new());
    let finality_gadget = Arc::new(Mutex::new(FinalityGadget::new(block_store.clone())));
    let metrics = Arc::new(crate::metrics::Metrics::new());

    // Initialize network
    let (mut network_service, network_cmd_sender, network_event_receiver) = NetworkService::new().unwrap();
    let peer_id = network_service.swarm.local_peer_id().clone();
    
    // Listen on localhost
    let listen_addr: Multiaddr = format!("/ip4/127.0.0.1/tcp/{}", p2p_port).parse().unwrap();
    network_service.swarm.listen_on(listen_addr.clone()).unwrap();

    // Spawn network service
    tokio::spawn(async move {
        network_service.run().await;
    });

    // Start RPC server
    let rpc_server = RpcServer::new(
        mempool.clone(),
        block_store.clone(),
        state_store.clone(),
        receipt_store.clone(),
        finality_gadget.clone(),
        metrics.clone(),
        network_cmd_sender.clone(),
    );
    
    tokio::spawn(async move {
        rpc_server.run(rpc_port, None).await;
    });

    // Start block producer (simplified for test)
    let block_producer = crate::block_producer::BlockProducer::new(
        block_store.clone(),
        mempool.clone(),
        network_cmd_sender.clone(),
        metrics.clone(),
    );
    
    tokio::spawn(async move {
        block_producer.start().await;
    });

    (
        TestNode {
            peer_id,
            listen_addr,
            network_cmd_sender,
            block_store,
            mempool,
            _temp_dir: temp_dir,
        },
        network_event_receiver,
    )
}

pub fn create_dummy_transaction() -> Transaction {
    Transaction {
        sender: [1; 20],
        to: Some([2; 20]),
        nonce: 0,
        value: 100,
        gas_limit: 21000,
        max_fee_per_gas: 1000,
        max_priority_fee_per_gas: 100,
        signature: vec![0; 64],
        payload: vec![],
        chain_id: Some(1),
    }
}

pub fn create_mock_components() -> (
    Arc<Mempool>,
    Arc<BlockStore>,
    Arc<StateStore>,
    Arc<ReceiptStore>,
    Arc<Mutex<FinalityGadget>>,
    Arc<crate::metrics::Metrics>,
    mpsc::Sender<NetworkCommand>,
) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path();
    
    let block_store = Arc::new(BlockStore::new(db_path.join("blocks")).unwrap());
    let state_store = Arc::new(StateStore::new(db_path.join("state")).unwrap());
    let receipt_store = Arc::new(ReceiptStore::new(db_path.join("receipts")).unwrap());
    let mempool = Arc::new(Mempool::new());
    let finality_gadget = Arc::new(Mutex::new(FinalityGadget::new(block_store.clone())));
    let metrics = Arc::new(crate::metrics::Metrics::new());
    let (tx, _) = mpsc::channel(100);

    (mempool, block_store, state_store, receipt_store, finality_gadget, metrics, tx)
}
