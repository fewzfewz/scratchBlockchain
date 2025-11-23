use clap::{Parser, Subcommand};
use common::traits::Consensus;
use common::types::{Block, Transaction};
use consensus::EnhancedConsensus;
use ed25519_dalek::{Signer, SigningKey};
use execution::Executor;
use network::{NetworkCommand, NetworkEvent, NetworkService};
use node::block_producer::BlockProducer;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use common::consensus_types::{ConsensusMessage, Vote, Proposal};
use consensus::bft::BftEvent;

#[derive(Parser)]
#[command(name = "modular-node")]
#[command(about = "A modular blockchain node", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the node
    Start {
        #[arg(long)]
        tls_cert: Option<String>,
        #[arg(long)]
        tls_key: Option<String>,
    },
    /// Generate a new keypair
    KeyGen,
    /// Submit a transaction
    SubmitTx {
        #[arg(long)]
        payload: String,
    },
    /// Query account balance
    QueryBalance {
        #[arg(long)]
        address: String,
    },
    /// Get block by height
    GetBlock {
        #[arg(long)]
        height: u64,
    },
    /// Connect to a specific peer
    ConnectPeer {
        #[arg(long)]
        multiaddr: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Start => {
            info!("Starting Modular Blockchain Node...");

            // Initialize components
            use node::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
            let _circuit_breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
            info!("Circuit breaker initialized");
            
            use storage::PersistentStore;
            let _store = PersistentStore::default().expect("Failed to initialize persistent storage");
            info!("Persistent storage initialized");
            
            // Initialize mempool
            use mempool::{Mempool, MempoolConfig};
            let mempool = Arc::new(Mempool::new(MempoolConfig::default()));
            info!("Mempool initialized");
            
            // Initialize state store
            let state_store = Arc::new(storage::StateStore::new("./data/state_db")?);
            info!("State store initialized");
            
            // Initialize genesis state if needed
            let genesis_config = common::types::GenesisConfig::default();
            if state_store.get_account(&genesis_config.accounts[0].address)?.is_none() {
                info!("Initializing genesis state...");
                state_store.initialize_genesis(&genesis_config)?;
            }

            // Initialize block store
            let block_store = Arc::new(storage::BlockStore::new("./data/block_db")?);
            info!("Block store initialized");

            // Initialize peer store
            let _peer_store = Arc::new(network::peer_store::PeerStore::new("./data/peers.json")?);
            info!("Peer store initialized");
            
            // Setup validators (for now, just generate a random one for this node)
            use common::crypto::SigningKey;
            use consensus::{EnhancedConsensus, ValidatorInfo, FinalityGadget};
            
            // Generate or load key
            let signing_key = SigningKey::generate();
            let pubkey_bytes = signing_key.public_key();
            
            info!("Node Public Key: {:?}", hex::encode(&pubkey_bytes));
            
            let validators = vec![ValidatorInfo {
                public_key: pubkey_bytes.clone(),
                stake: 100,
                slashed: false,
            }];
            
            let consensus = Arc::new(Mutex::new(EnhancedConsensus::new(validators.clone())));
            let finality_gadget = Arc::new(Mutex::new(FinalityGadget::new(validators.clone())));
            
            // Initialize network
            let (network, network_cmd_sender, mut network_event_receiver) = NetworkService::new()?;
            let network_cmd_sender = Arc::new(network_cmd_sender);

            // Initialize block producer
            let mut block_producer = BlockProducer::new(
                mempool.clone(),
                consensus.clone(),
                state_store.clone(),
                block_store.clone(),
                finality_gadget.clone(),
                signing_key.clone(),
            );

            // Initialize fork choice
            let fork_choice = Arc::new(Mutex::new(node::fork_choice::ForkChoice::new()));

            // Initialize metrics
            let metrics = Arc::new(node::metrics::Metrics::new());

            // Initialize receipt store
            let receipt_store = Arc::new(storage::receipt_store::ReceiptStore::new("./data/receipts_db")?);
            info!("Receipt store initialized");

            // Start RPC server
            let rpc_server = node::rpc::RpcServer::new(
                mempool.clone(),
                block_store.clone(),
                state_store.clone(),
                receipt_store.clone(),
                finality_gadget.clone(),
                metrics.clone(),
                (*network_cmd_sender).clone(),
            );
            tokio::spawn(async move {
                rpc_server.run(9933, tls_config).await;
            });

            info!("Components initialized. Running network...");
            
            // Spawn network task
            tokio::spawn(network.run());

            // Main event loop
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            let mut last_block = Block::genesis();

            // Initialize BFT Engine
            let mut bft_engine = consensus::bft::BftEngine::new(
                pubkey_bytes.clone(),
                validators.clone(),
                1, // Start at height 1
                signing_key,
            );
            
            // Start first round
            let mut pending_bft_events = bft_engine.start_round(0);

            loop {
                // Process pending BFT events
                let mut new_events = Vec::new();
                for event in pending_bft_events.drain(..) {
                    match event {
                        BftEvent::BroadcastVote(vote) => {
                            let msg = ConsensusMessage::Vote(vote);
                            if let Err(e) = network_cmd_sender.send(NetworkCommand::BroadcastConsensusMessage(msg)).await {
                                error!("Failed to broadcast vote: {}", e);
                            }
                        }
                        BftEvent::BroadcastProposal(proposal) => {
                            let msg = ConsensusMessage::Proposal(proposal);
                            if let Err(e) = network_cmd_sender.send(NetworkCommand::BroadcastConsensusMessage(msg)).await {
                                error!("Failed to broadcast proposal: {}", e);
                            }
                        }
                        BftEvent::FinalizeBlock(block) => {
                            info!("Finalizing block at height {}", block.header.slot);
                            
                            // Load current state
                            let mut state = match state_store.get_all_accounts() {
                                Ok(s) => s,
                                Err(e) => {
                                    error!("Failed to load state: {}", e);
                                    std::collections::HashMap::new()
                                }
                            };

                            // Execute block
                            let executor = execution::NativeExecutor::new();
                            if let Err(e) = executor.execute_block(&block, &mut state) {
                                error!("Block execution failed during finalization: {}", e);
                            }
                            
                            // Persist state
                            for (address, account) in state {
                                if let Err(e) = state_store.put_account(&address, &account) {
                                    error!("Failed to persist account state: {}", e);
                                }
                            }
                            
                            // Persist block
                            if let Err(e) = block_store.put_block(&block) {
                                error!("Failed to persist block: {}", e);
                            }
                            if let Err(e) = block_store.set_latest_height(block.header.slot) {
                                error!("Failed to update latest height: {}", e);
                            }
                            
                            // Update metrics
                            metrics.record_block();
                            metrics.update_finalized_height(block.header.slot);
                            
                            // Remove included transactions from mempool
                            mempool.remove_transactions(&block.extrinsics);
                            
                            // Broadcast finalized block to network (for observers/sync)
                            if let Err(e) = network_cmd_sender.send(NetworkCommand::BroadcastBlock(block.clone())).await {
                                error!("Failed to broadcast finalized block: {}", e);
                            }

                            last_block = block;
                        }
                        BftEvent::NewRound(height, round) => {
                            info!("Starting new round: height {}, round {}", height, round);
                            if bft_engine.is_proposer(height, round) {
                                info!("I am the proposer for height {}, round {}", height, round);
                                match block_producer.produce_block(&last_block).await {
                                    Ok(block) => {
                                        info!("Produced block for consensus");
                                        let events = bft_engine.create_proposal(block);
                                        new_events.extend(events);
                                    }
                                    Err(e) => {
                                        if e.to_string() != "No transactions available" {
                                            warn!("Failed to produce block: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        BftEvent::Timeout(step) => {
                            warn!("Timeout in step {:?} at height {}, round {}", 
                                  step, bft_engine.height, bft_engine.round);
                            
                            use common::consensus_types::Step;
                            let events = match step {
                                Step::Propose => {
                                    info!("Propose timeout - proposer didn't send proposal");
                                    bft_engine.handle_timeout_propose()
                                }
                                Step::Prevote => {
                                    info!("Prevote timeout - didn't get enough prevotes");
                                    bft_engine.handle_timeout_prevote()
                                }
                                Step::Precommit => {
                                    info!("Precommit timeout - didn't get enough precommits, moving to next round");
                                    bft_engine.handle_timeout_precommit()
                                }
                                Step::Commit => {
                                    error!("Unexpected timeout in Commit step");
                                    vec![]
                                }
                            };
                            new_events.extend(events);
                        }
                    }
                }
                pending_bft_events.extend(new_events);

                tokio::select! {
                    // Heartbeat - check for timeouts
                    _ = interval.tick() => {
                        // Check if BFT timeout has expired
                        if let Some(timeout_event) = bft_engine.check_timeout() {
                            pending_bft_events.push(timeout_event);
                        }
                    }

                    // Network events
                    Some(event) = network_event_receiver.recv() => {
                        match event {
                            NetworkEvent::TransactionReceived(tx) => {
                                info!("Received transaction from network");
                                if let Err(e) = mempool.add_transaction(tx) {
                                    warn!("Failed to add transaction to mempool: {}", e);
                                }
                            }
                            NetworkEvent::ConsensusMessageReceived(msg) => {
                                let events = match msg {
                                    ConsensusMessage::Vote(vote) => bft_engine.handle_vote(vote),
                                    ConsensusMessage::Proposal(proposal) => bft_engine.handle_proposal(proposal),
                                };
                                pending_bft_events.extend(events);
                            }
                            NetworkEvent::BlockReceived { block, source } => {
                                info!("Received block from network: slot {}", block.header.slot);
                                
                                // Check if parent is known (for MVP, just check against last_block)
                                // In a real system, check storage/index
                                if block.header.parent_hash != last_block.hash() && block.header.slot > 0 {
                                    info!("Received orphan block (parent unknown). Requesting missing blocks from height {}", last_block.header.slot + 1);
                                    // Request missing blocks from the source peer
                                    if let Err(e) = network_cmd_sender.send(NetworkCommand::RequestBlock { 
                                        peer: source, 
                                        start_height: last_block.header.slot + 1,
                                        limit: 10, // Request up to 10 blocks at a time
                                    }).await {
                                        error!("Failed to send block request: {}", e);
                                    }
                                } else {
                                    // Verify block with consensus
                                    let consensus = consensus.lock().await;
                                    if let Err(e) = consensus.verify_block(&block) {
                                        warn!("Invalid block received: {}", e);
                                    } else {
                                        // Verify state transition
                                        info!("Verifying state transition for block {}", block.header.slot);
                                        
                                        // Load current state
                                        let mut state = match state_store.get_all_accounts() {
                                            Ok(s) => s,
                                            Err(e) => {
                                                error!("Failed to load state: {}", e);
                                                continue;
                                            }
                                        };
                                        
                                        // Execute block
                                        let executor = execution::NativeExecutor::new();
                                        if let Err(e) = executor.execute_block(&block, &mut state) {
                                            warn!("Block execution failed: {}", e);
                                            continue;
                                        }
                                        
                                        // Verify state root
                                        let computed_root = storage::StateStore::compute_root(&state);
                                        if computed_root != block.header.state_root {
                                            warn!("Invalid state root. Expected {:?}, got {:?}", block.header.state_root, computed_root);
                                            continue;
                                        }
                                        
                                        // Use fork choice to decide what to do
                                        let mut fc = fork_choice.lock().await;
                                        match fc.handle_incoming_block(&block, &block_store) {
                                            Ok(node::fork_choice::ForkDecision::Accept) => {
                                                info!("Block accepted: slot {}", block.header.slot);
                                                
                                                // Persist state
                                                for (address, account) in state {
                                                    if let Err(e) = state_store.put_account(&address, &account) {
                                                        error!("Failed to persist account state: {}", e);
                                                    }
                                                }
                                                
                                                // Persist block
                                                if let Err(e) = block_store.put_block(&block) {
                                                    error!("Failed to persist block: {}", e);
                                                }
                                                if let Err(e) = block_store.set_latest_height(block.header.slot) {
                                                    error!("Failed to update latest height: {}", e);
                                                }
                                                
                                                // Update metrics
                                                metrics.record_block();
                                                metrics.update_finalized_height(block.header.slot);
                                                
                                                // Remove included transactions from mempool
                                                mempool.remove_transactions(&block.extrinsics);
                                                
                                                last_block = block;
                                            }
                                            Ok(node::fork_choice::ForkDecision::Reorg { new_tip, new_height }) => {
                                                warn!("Reorg needed to tip {:?} at height {}", new_tip, new_height);
                                                // TODO: Implement full reorg logic (state rollback, etc.)
                                                // For now, just accept if it's strictly better and we haven't finalized
                                                
                                                // Persist state (Note: this overwrites current state, which is correct for simple reorg to better chain)
                                                for (address, account) in state {
                                                    if let Err(e) = state_store.put_account(&address, &account) {
                                                        error!("Failed to persist account state: {}", e);
                                                    }
                                                }
                                                
                                                if let Err(e) = block_store.put_block(&block) {
                                                    error!("Failed to persist block: {}", e);
                                                }
                                                if let Err(e) = block_store.set_latest_height(block.header.slot) {
                                                    error!("Failed to update latest height: {}", e);
                                                }
                                                last_block = block;
                                            }
                                            Ok(node::fork_choice::ForkDecision::Ignore) => {
                                                info!("Ignoring block (old or duplicate)");
                                            }
                                            Err(e) => {
                                                error!("Fork choice error: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                            NetworkEvent::BlockRequestReceived { peer, request_id: _, start_height, limit, channel } => {
                                info!("Received block request from {:?} for range {}..{}", peer, start_height, start_height + limit as u64);
                                
                                // Fetch blocks from storage
                                let mut blocks = Vec::new();
                                for height in start_height..(start_height + limit as u64) {
                                    match block_store.get_block_by_height(height) {
                                        Ok(Some(block)) => blocks.push(block),
                                        Ok(None) => {
                                            info!("Block at height {} not found", height);
                                            break;
                                        }
                                        Err(e) => {
                                            error!("Failed to fetch block at height {}: {}", height, e);
                                            break;
                                        }
                                    }
                                }
                                
                                if let Err(e) = network_cmd_sender.send(NetworkCommand::SendBlockResponse { channel, blocks }).await {
                                    error!("Failed to send block response: {:?}", e);
                                }
                            }
                            NetworkEvent::BlockResponseReceived { peer, request_id: _, blocks } => {
                                info!("Received {} blocks from {:?}", blocks.len(), peer);
                                
                                // Process each block in order
                                for block in blocks {
                                    // Verify block with consensus
                                    let consensus = consensus.lock().await;
                                    if let Err(e) = consensus.verify_block(&block) {
                                        warn!("Invalid block received in sync response: {}", e);
                                        break;
                                    }
                                    drop(consensus);
                                    
                                    // Load current state
                                    let mut state = match state_store.get_all_accounts() {
                                        Ok(s) => s,
                                        Err(e) => {
                                            error!("Failed to load state: {}", e);
                                            break;
                                        }
                                    };
                                    
                                    // Execute block
                                    let executor = execution::NativeExecutor::new();
                                    if let Err(e) = executor.execute_block(&block, &mut state) {
                                        warn!("Block execution failed during sync: {}", e);
                                        break;
                                    }
                                    
                                    // Verify state root
                                    let computed_root = storage::StateStore::compute_root(&state);
                                    if computed_root != block.header.state_root {
                                        warn!("Invalid state root during sync. Expected {:?}, got {:?}", block.header.state_root, computed_root);
                                        break;
                                    }
                                    
                                    // Persist state
                                    for (address, account) in state {
                                        if let Err(e) = state_store.put_account(&address, &account) {
                                            error!("Failed to persist account state: {}", e);
                                        }
                                    }
                                    
                                    // Persist block
                                    if let Err(e) = block_store.put_block(&block) {
                                        error!("Failed to persist block: {}", e);
                                        break;
                                    }
                                    if let Err(e) = block_store.set_latest_height(block.header.slot) {
                                        error!("Failed to update latest height: {}", e);
                                        break;
                                    }
                                    
                                    // Update metrics
                                    metrics.record_block();
                                    metrics.update_finalized_height(block.header.slot);
                                    
                                    // Remove included transactions from mempool
                                    mempool.remove_transactions(&block.extrinsics);
                                    
                                    last_block = block.clone();
                                    info!("Synced block at height {}", block.header.slot);
                                }
                            }
                            NetworkEvent::ListeningOn(addr) => {
                                info!("Network listening on {:?}", addr);
                            }
                        }
                    }
                    
                    // Shutdown signal
                    _ = tokio::signal::ctrl_c() => {
                        info!("Shutdown signal received. Saving peers and exiting...");
                        if let Err(e) = network_cmd_sender.send(NetworkCommand::SavePeers).await {
                            error!("Failed to send SavePeers command: {}", e);
                        }
                        // Give some time for the command to be processed
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        break;
                    }
                }
            }
        }
        Commands::KeyGen => {
            println!("Generating keypair...");
            use ed25519_dalek::SigningKey;
            use rand::rngs::OsRng;
            use rand::RngCore;
            
            let mut csprng = OsRng;
            let mut secret_bytes = [0u8; 32];
            csprng.fill_bytes(&mut secret_bytes);
            let signing_key = SigningKey::from_bytes(&secret_bytes);
            let verifying_key = signing_key.verifying_key();
            
            println!("Secret Key: {}", hex::encode(secret_bytes));
            println!("Public Key: {}", hex::encode(verifying_key.to_bytes()));
        }
        Commands::SubmitTx { payload } => {
            // Connect to local node and submit transaction
            // For MVP, we'll just print what we would do
            // In a real implementation, this would connect via RPC/HTTP
            println!("Submitting transaction with payload: {}", payload);
            
            use ed25519_dalek::SigningKey;
            use rand::rngs::OsRng;
            use rand::RngCore;
            
            let mut csprng = OsRng;
            let mut secret_bytes = [0u8; 32];
            csprng.fill_bytes(&mut secret_bytes);
            let signing_key = SigningKey::from_bytes(&secret_bytes);
            
            let mut tx = Transaction::test_transaction([0; 20], 0);
            tx.payload = payload.as_bytes().to_vec();
            tx.signature = vec![]; // Placeholder
            
            // Sign tx
            let tx_hash = tx.hash();
            let signature = signing_key.sign(&tx_hash);
            let mut signed_tx = tx;
            signed_tx.signature = signature.to_bytes().to_vec();
            
            println!("Signed transaction: {:?}", signed_tx);
            println!("(To actually submit, we need to implement RPC/API layer)");
        }
        Commands::QueryBalance { address } => {
            info!("Querying balance for address: {}", address);
            
            // Call RPC endpoint
            let url = format!("http://localhost:9933/balance/{}", address);
            match reqwest::get(&url).await {
                Ok(response) => {
                    if let Ok(text) = response.text().await {
                        println!("Balance response: {}", text);
                    } else {
                        eprintln!("Failed to read response");
                    }
                }
                Err(e) => {
                    eprintln!("Failed to query balance: {}", e);
                    eprintln!("Make sure the node is running with: cargo run --bin node start");
                }
            }
        }
        Commands::GetBlock { height } => {
            info!("Getting block at height: {}", height);
            
            // Call RPC endpoint
            let url = format!("http://localhost:9933/block/{}", height);
            match reqwest::get(&url).await {
                Ok(response) => {
                    if let Ok(text) = response.text().await {
                        println!("Block response: {}", text);
                    } else {
                        eprintln!("Failed to read response");
                    }
                }
                Err(e) => {
                    eprintln!("Failed to get block: {}", e);
                    eprintln!("Make sure the node is running with: cargo run --bin node start");
                }
            }
        }
        Commands::ConnectPeer { multiaddr } => {
            info!("Connecting to peer: {}", multiaddr);
            println!("Peer connection via CLI not yet implemented.");
            println!("You can connect peers by modifying the network configuration.");
            println!("Multiaddr: {}", multiaddr);
        }
    }

    Ok(())
}
