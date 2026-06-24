//! # Modular Blockchain Node - Main Entry Point
//! 
//! This file implements the main node process that ties together all components:
//! - Consensus (BFT engine)
//! - Network (P2P layer)
//! - Mempool (transaction pool)
//! - Storage (block and state persistence)
//! - Rewards & Slashing (validator economics)
//! - RPC (API for clients)
//! - Metrics (monitoring)

use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{error, info, warn, debug};

// Import from our modules
mod config;
mod rewards;
mod sync;

use config::NodeConfig;
use rewards::{RewardManager, RewardConfig};

// External imports
use common::consensus_types::{ConsensusMessage, Proposal, Vote, Step};
use common::traits::Consensus;
use common::types::{Block, Transaction, Header};
use common::crypto::SigningKey;

use consensus::bft::{BftEngine, BftEvent};
use consensus::{EnhancedConsensus, FinalityGadget, ValidatorInfo, SlashingCondition};

use execution::evm::EvmExecutor;

use network::{NetworkCommand, NetworkEvent, NetworkService};

use mempool::Mempool;

use storage::{ChainStore, ColumnFamily, WriteBatch};
use storage::trie::PatriciaTrie;
use storage::receipt_store::ReceiptStore;

use node::block_producer::{BlockProducer, BlockExecutor, BlockProducerConfig};
use node::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use node::fork_choice::ForkChoice;
use node::metrics::Metrics;
use node::light_client::{LightClient, SyncCommittee, SyncCommitteeManager};
use node::runtime_upgrade::RuntimeUpgradeManager;

/// CLI arguments parser
#[derive(Parser)]
#[command(name = "modular-node")]
#[command(about = "A production-ready modular blockchain node")]
#[command(version = "2.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the node (normal operation)
    Start {
        #[arg(long, help = "Path to TOML configuration file")]
        config: PathBuf,
        
        #[arg(long, help = "Path to genesis JSON file", default_value = "genesis.json")]
        genesis: PathBuf,
    },
    
    /// Generate a new validator keypair
    KeyGen {
        #[arg(long, help = "Output path for the key file", default_value = "validator_key.json")]
        output: PathBuf,
    },
    
    /// Submit a transaction to the node
    SubmitTx {
        #[arg(long, help = "Transaction payload (hex or plaintext)")]
        payload: String,
        
        #[arg(long, help = "RPC endpoint URL", default_value = "http://localhost:9933")]
        rpc_url: String,
    },
    
    /// Query account balance
    QueryBalance {
        #[arg(long, help = "Account address (hex with 0x prefix)")]
        address: String,
        
        #[arg(long, help = "RPC endpoint URL", default_value = "http://localhost:9933")]
        rpc_url: String,
    },
    
    /// Get block by height
    GetBlock {
        #[arg(long, help = "Block height")]
        height: u64,
        
        #[arg(long, help = "RPC endpoint URL", default_value = "http://localhost:9933")]
        rpc_url: String,
    },
    
    /// Connect to a specific peer
    ConnectPeer {
        #[arg(long, help = "Peer multiaddress (e.g., /ip4/192.168.1.1/tcp/9000)")]
        multiaddr: String,
        
        #[arg(long, help = "RPC endpoint URL", default_value = "http://localhost:9933")]
        rpc_url: String,
    },
    
    /// Start the faucet service (testnet token dispenser)
    Faucet,
    
    /// Show node status and statistics
    Status {
        #[arg(long, help = "RPC endpoint URL", default_value = "http://localhost:9933")]
        rpc_url: String,
    },
}

/// BFT state persistence for crash recovery
#[derive(serde::Serialize, serde::Deserialize)]
struct BftPersistedState {
    height: u64,
    round: u64,
    locked_block_hash: Option<[u8; 32]>,
    locked_round: u64,
    valid_block_hash: Option<[u8; 32]>,
    valid_round: u64,
    last_saved: u64,
}

/// Node statistics
struct NodeStats {
    start_time: Instant,
    blocks_processed: u64,
    transactions_processed: u64,
    last_block_time: Instant,
}

impl NodeStats {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            blocks_processed: 0,
            transactions_processed: 0,
            last_block_time: Instant::now(),
        }
    }
    
    fn record_block(&mut self, tx_count: usize) {
        self.blocks_processed += 1;
        self.transactions_processed += tx_count as u64;
        self.last_block_time = Instant::now();
    }
    
    fn get_throughput(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.blocks_processed as f64 / elapsed
        } else {
            0.0
        }
    }
}

/// Main node structure
struct Node {
    // Core components
    config: NodeConfig,
    chain_store: Arc<ChainStore>,
    receipt_store: Arc<ReceiptStore>,
    state_trie: Arc<Mutex<PatriciaTrie>>,
    
    // Consensus
    bft_engine: Arc<Mutex<BftEngine>>,
    finality_gadget: Arc<Mutex<FinalityGadget>>,
    
    // Execution
    block_executor: Arc<Mutex<BlockExecutor>>,
    block_producer: Arc<Mutex<BlockProducer>>,
    
    // Economic components
    reward_manager: Arc<Mutex<RewardManager>>,
    
    // Mempool
    mempool: Arc<Mempool>,
    
    // Network
    network_cmd_sender: Arc<tokio::sync::mpsc::Sender<NetworkCommand>>,
    network_event_receiver: tokio::sync::mpsc::Receiver<NetworkEvent>,
    
    // Monitoring
    metrics: Arc<Metrics>,
    circuit_breaker: Arc<CircuitBreaker>,
    stats: NodeStats,
    
    // Persistence
    bft_state_path: PathBuf,
}

impl Node {
    async fn new(config_path: PathBuf, genesis_path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        info!("🚀 Initializing node components...");
        
        // ================================================================
        // 1. Load Configuration
        // ================================================================
        let config = NodeConfig::load(&config_path)?;
        info!("✅ Loaded configuration from {:?}", config_path);
        
        // ================================================================
        // 2. Initialize Storage
        // ================================================================
        let data_dir = &config.storage.data_dir;
        fs::create_dir_all(data_dir)?;
        
        let chain_store = Arc::new(ChainStore::open(data_dir)?);
        let kv_store = chain_store.inner().clone();
        let receipt_store = Arc::new(ReceiptStore::new(kv_store.clone()));
        let state_trie = Arc::new(Mutex::new(PatriciaTrie::new(kv_store)?));
        
        // ================================================================
        // 3. Load Genesis if needed
        // ================================================================
        if chain_store.get_latest_height()?.is_none() {
            info!("📦 Initializing genesis state...");
            let genesis_content = fs::read_to_string(&genesis_path)?;
            let genesis_config: common::types::GenesisConfig = serde_json::from_str(&genesis_content)?;
            
            // Initialize genesis accounts in state trie
            for account in genesis_config.accounts {
                let account_json = serde_json::to_vec(&account)?;
                state_trie.lock().await.insert(&account.address, &account_json)?;
            }
            
            // Store genesis block
            let genesis_block = Block::genesis();
            let block_hash = genesis_block.hash();
            let block_data = serde_json::to_vec(&genesis_block)?;
            chain_store.put_block(&block_hash, &block_data)?;
            chain_store.put_block_height(0, &block_hash)?;
            chain_store.set_latest_height(0)?;
        }
        
        // ================================================================
        // 4. Load Validator Keys
        // ================================================================
        let key_path = PathBuf::from(data_dir).join("validator_key.json");
        let signing_key = load_or_generate_key(&key_path)?;
        let public_key = signing_key.public_key();
        
        // ================================================================
        // 5. Setup Validator Set from Genesis
        // ================================================================
        let genesis_content = fs::read_to_string(&genesis_path)?;
        let genesis_config: common::types::GenesisConfig = serde_json::from_str(&genesis_content)?;
        
        let validators: Vec<ValidatorInfo> = genesis_config
            .validators
            .iter()
            .map(|v| ValidatorInfo {
                public_key: hex::decode(&v.public_key).unwrap_or_default(),
                stake: v.stake as u64,
                slashed: false,
            })
            .collect();
        
        info!("✅ Loaded {} validators from genesis", validators.len());
        
        // ================================================================
        // 6. Initialize Mempool
        // ================================================================
        let mempool_config = mempool::MempoolConfig {
            max_capacity: (config.security.max_tx_per_block * 2) as usize,
            max_per_sender: 100,
            min_fee_per_gas: 1_000_000_000,
            chain_id: Some(1),
        };
        let mempool = Arc::new(Mempool::new(mempool_config));
        
        // ================================================================
        // 7. Initialize BFT Engine
        // ================================================================
        let bft_state_path = PathBuf::from(data_dir).join("bft_state.json");
        let bft_engine = Arc::new(Mutex::new(BftEngine::new(
            public_key.clone(),
            validators.clone(),
            1,
            signing_key.clone(),
        )));
        
        // ================================================================
        // 8. Initialize Finality Gadget
        // ================================================================
        let finality_gadget = Arc::new(Mutex::new(FinalityGadget::new(validators.clone())));
        
        // ================================================================
        // 9. Initialize Block Executor
        // ================================================================
        let block_executor = BlockExecutor::new(EvmExecutor::new(), chain_store.clone());
        let block_executor = Arc::new(Mutex::new(block_executor));
        
        // ================================================================
        // 11. Initialize Block Producer
        // ================================================================
        let block_producer_config = BlockProducerConfig {
            max_transactions_per_block: config.security.max_tx_per_block,
            max_gas_per_block: config.security.max_gas_per_block,
            target_gas_utilization: 0.5,
        };
        
        let producer_executor = BlockExecutor::new(EvmExecutor::new(), chain_store.clone());
        let block_producer = Arc::new(Mutex::new(BlockProducer::new(
            mempool.clone(),
            bft_engine.clone(),
            producer_executor,
            signing_key.clone(),
            public_key.clone().try_into().unwrap_or([0u8; 20]),
            block_producer_config,
        )));
        
        // ================================================================
        // 12. Initialize Economic Components
        // ================================================================
        let reward_config = RewardConfig::default();
        let reward_manager = Arc::new(Mutex::new(RewardManager::new(reward_config)));
        
        // ================================================================
        // 13. Initialize Network
        // ================================================================
        let peer_store_path = format!("{}/peers.json", data_dir);
        let (network, network_cmd_sender, network_event_receiver) = NetworkService::new(
            config.network.bootstrap_nodes.clone(),
            &peer_store_path,
        )?;
        let network_cmd_sender = Arc::new(network_cmd_sender);
        
        // Start listening
        let p2p_addr: libp2p::Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", config.network.p2p_port)
            .parse()
            .expect("Invalid P2P address");
        network_cmd_sender
            .send(NetworkCommand::StartListening(p2p_addr))
            .await?;
        
        // Set network channel for block producer
        {
            let mut producer = block_producer.lock().await;
            producer.set_network_channel((*network_cmd_sender).clone());
        }
        
        // Spawn network task
        tokio::spawn(network.run());
        
        // ================================================================
        // 14. Initialize Circuit Breaker
        // ================================================================
        let circuit_breaker_config = CircuitBreakerConfig::default();
        let circuit_breaker = Arc::new(CircuitBreaker::new(circuit_breaker_config));
        
        // ================================================================
        // 15. Initialize Metrics
        // ================================================================
        let metrics = Arc::new(Metrics::new());
        
        // ================================================================
        // 16. Start RPC Server
        // ================================================================
        let rpc_server = node::rpc::RpcServer::new(
            mempool.clone(),
            chain_store.clone(),
            metrics.clone(),
            (*network_cmd_sender).clone(),
        );
        
        let rpc_port = config.network.rpc_port;
        let enable_cors = !config.api.cors_origins.is_empty();
        tokio::spawn(async move {
            info!("🔌 RPC server listening on port {}", rpc_port);
            rpc_server.run(rpc_port, enable_cors).await;
        });
        
        Ok(Self {
            config,
            chain_store,
            receipt_store,
            state_trie,
            bft_engine,
            finality_gadget,
            block_executor,
            block_producer,
            reward_manager,
            mempool,
            network_cmd_sender,
            network_event_receiver,
            metrics,
            circuit_breaker,
            stats: NodeStats::new(),
            bft_state_path,
        })
    }
    
    /// Run main event loop
    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🎉 Node fully initialized, starting consensus...");
        
        // Start first round
        let current_round;
        let mut pending_events = {
            let mut engine = self.bft_engine.lock().await;
            current_round = engine.round;
            engine.start_round(current_round)
        };
        
        let mut interval = interval(Duration::from_millis(1));
        let latest_block = self.get_latest_block().await?;
        
        loop {
            // Process BFT events
            let mut new_events = Vec::new();
            for event in pending_events.drain(..) {
                self.handle_bft_event(event, &mut new_events, &latest_block).await?;
            }
            pending_events.extend(new_events);
            
            tokio::select! {
                _ = interval.tick() => {
                    let mut engine = self.bft_engine.lock().await;
                    if let Some(timeout) = engine.check_timeout() {
                        pending_events.push(timeout);
                    }
                }
                
                Some(event) = self.network_event_receiver.recv() => {
                    self.handle_network_event(event, &mut pending_events).await?;
                }
                
                _ = tokio::signal::ctrl_c() => {
                    info!("🛑 Shutdown signal received");
                    self.shutdown().await?;
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle BFT events
    async fn handle_bft_event(
        &mut self,
        event: BftEvent,
        new_events: &mut Vec<BftEvent>,
        latest_block: &Block,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match event {
            BftEvent::BroadcastVote(vote) => {
                debug!("Broadcasting vote for round {}", vote.round);
                self.network_cmd_sender
                    .send(NetworkCommand::BroadcastConsensusMessage(ConsensusMessage::Vote(vote)))
                    .await?;
            }
            
            BftEvent::BroadcastProposal(proposal) => {
                info!("Broadcasting proposal for round {}", proposal.round);
                self.network_cmd_sender
                    .send(NetworkCommand::BroadcastConsensusMessage(ConsensusMessage::Proposal(proposal)))
                    .await?;
            }
            
            BftEvent::FinalizeBlock(block) => {
                info!("🔒 Finalizing block at height {}", block.header.slot);
                
                // Execute and commit block
                let receipts = self.block_executor.lock().await.execute_and_commit(&block)?;
                
                // Calculate total fees
                let total_fees: u64 = receipts.iter()
                    .map(|r| r.gas_used * block.header.base_fee)
                    .sum();
                
                // Get voters (simplified - in production, get from consensus)
                let voters: Vec<Vec<u8>> = vec![];
                
                // Process rewards
                let proposer = block.header.validator_set_id.to_le_bytes().to_vec();
                let rewards = {
                    let mut rm = self.reward_manager.lock().await;
                    rm.process_block(&block, proposer, &voters, total_fees)
                };
                
                // Apply rewards to state
                for (validator, amount) in rewards {
                    if amount > 0 {
                        self.apply_reward(&validator, amount as u64).await?;
                        info!("💰 Rewarded {} with {} tokens", hex::encode(&validator[..4]), amount);
                    }
                }
                
                // Update metrics
                self.metrics.record_block();
                self.stats.record_block(block.extrinsics.len());
                
                // Remove transactions from mempool
                self.mempool.remove_transactions(&block.extrinsics);
                
                // Broadcast block
                self.network_cmd_sender
                    .send(NetworkCommand::BroadcastBlock(block))
                    .await?;
            }
            
            BftEvent::NewRound(height, round) => {
                info!("🔄 New round: height={}, round={}", height, round);
                
                // Persist BFT state periodically
                if round % 10 == 0 {
                    self.persist_bft_state().await?;
                }
                
                // Check if we're proposer
                let is_proposer = {
                    let engine = self.bft_engine.lock().await;
                    engine.is_proposer(height, round)
                };
                
                if is_proposer {
                    info!("🎁 I am the proposer for round {}/{}", height, round);
                    let mut producer = self.block_producer.lock().await;
                    match producer.produce_block(latest_block).await {
                        Ok(block) => {
                            info!("✅ Produced block with {} transactions", block.extrinsics.len());
                            let mut engine = self.bft_engine.lock().await;
                            let events = engine.create_proposal(block);
                            new_events.extend(events);
                        }
                        Err(e) => warn!("Failed to produce block: {}", e),
                    }
                }
            }
            
            BftEvent::Timeout(step) => {
                warn!("⏰ Timeout at step {:?}", step);
                let mut engine = self.bft_engine.lock().await;
                let events = match step {
                    Step::Propose => engine.handle_timeout_propose(),
                    Step::Prevote => engine.handle_timeout_prevote(),
                    Step::Precommit => engine.handle_timeout_precommit(),
                    Step::Commit => vec![],
                };
                new_events.extend(events);
            }
        }
        
        Ok(())
    }
    
    /// Handle network events
    async fn handle_network_event(
        &mut self,
        event: NetworkEvent,
        pending_events: &mut Vec<BftEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match event {
            NetworkEvent::TransactionReceived(tx) => {
                if let Err(e) = self.mempool.add_transaction(tx) {
                    debug!("Transaction rejected: {}", e);
                }
            }
            
            NetworkEvent::ConsensusMessageReceived(msg) => {
                let events = match msg {
                    ConsensusMessage::Vote(vote) => {
                        let mut engine = self.bft_engine.lock().await;
                        engine.handle_vote(vote)
                    }
                    ConsensusMessage::Proposal(proposal) => {
                        let mut engine = self.bft_engine.lock().await;
                        engine.handle_proposal(proposal)
                    }
                };
                pending_events.extend(events);
            }
            
            NetworkEvent::BlockReceived { block, source } => {
                self.handle_incoming_block(block, source).await?;
            }
            
            NetworkEvent::ListeningOn(addr) => {
                info!("🔊 Network listening on {}", addr);
            }
            
            _ => {}
        }
        
        Ok(())
    }
    
    /// Handle incoming block from network
    async fn handle_incoming_block(
        &mut self,
        block: Block,
        _source: libp2p::PeerId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check if we already have this block
        let block_hash = block.hash();
        if self.chain_store.get_block(&block_hash)?.is_some() {
            debug!("Block already known, ignoring");
            return Ok(());
        }
        
        // Verify block
        let validators = self.get_validator_list().await?;
        let consensus = EnhancedConsensus::new(validators);
        if let Err(e) = consensus.verify_block(&block) {
            warn!("Invalid block: {}", e);
            return Ok(());
        }
        
        // Execute block
        let receipts = self.block_executor.lock().await.execute_and_commit(&block)?;
        
        // Verify state root
        let computed_root = {
            let trie = self.state_trie.lock().await;
            trie.root_hash()
        };
        if computed_root != block.header.state_root {
            warn!("State root mismatch");
            return Ok(());
        }
        
        // Process rewards
        let total_fees: u64 = receipts.iter().map(|r| r.gas_used * block.header.base_fee).sum();
        let proposer = block.header.validator_set_id.to_le_bytes().to_vec();
        let voters = vec![];
        
        let rewards = {
            let mut rm = self.reward_manager.lock().await;
            rm.process_block(&block, proposer, &voters, total_fees)
        };
        for (validator, amount) in rewards {
            if amount > 0 {
                self.apply_reward(&validator, amount as u64).await?;
            }
        }
        
        // Update mempool
        self.mempool.remove_transactions(&block.extrinsics);
        
        info!("✅ Processed block at height {}", block.header.slot);
        self.metrics.record_block();
        self.stats.record_block(block.extrinsics.len());
        
        Ok(())
    }
    
    /// Get latest block from storage
    async fn get_latest_block(&self) -> Result<Block, Box<dyn std::error::Error>> {
        match self.chain_store.get_latest_height()? {
            Some(height) => {
                let data = self.chain_store
                    .get_block_by_height(height)?
                    .ok_or_else(|| format!("Block not found at height {}", height))?;
                let block: Block = serde_json::from_slice(&data)?;
                Ok(block)
            }
            None => Ok(Block::genesis()),
        }
    }
    
    /// Get current validator list
    async fn get_validator_list(&self) -> Result<Vec<ValidatorInfo>, Box<dyn std::error::Error>> {
        // In production, load from state
        Ok(vec![])
    }
    
    /// Apply reward to validator
    async fn apply_reward(&mut self, validator: &[u8], amount: u64) -> Result<(), Box<dyn std::error::Error>> {
        let address = if validator.len() >= 20 {
            let mut addr = [0u8; 20];
            addr.copy_from_slice(&validator[..20]);
            addr
        } else {
            [0u8; 20]
        };
        
        let mut trie = self.state_trie.lock().await;
        let mut account: common::types::Account = match trie.get(&address)? {
            Some(data) => serde_json::from_slice(&data)?,
            None => common::types::Account::default(),
        };
        
        account.balance += amount as u128;
        trie.insert(&address, &serde_json::to_vec(&account)?)?;
        
        Ok(())
    }
    
    /// Persist BFT state to disk
    async fn persist_bft_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        let engine = self.bft_engine.lock().await;
        let state = BftPersistedState {
            height: engine.height,
            round: engine.round,
            locked_block_hash: None,
            locked_round: 0,
            valid_block_hash: None,
            valid_round: 0,
            last_saved: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        
        let content = serde_json::to_string_pretty(&state)?;
        fs::write(&self.bft_state_path, content)?;
        Ok(())
    }
    
    /// Graceful shutdown
    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Saving node state...");
        self.persist_bft_state().await?;
        self.network_cmd_sender.send(NetworkCommand::SavePeers).await?;
        info!("Node shutdown complete. Processed {} blocks at {:.2} blocks/sec",
              self.stats.blocks_processed, self.stats.get_throughput());
        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

use warp::Filter;

/// Load or generate validator key
fn load_or_generate_key(key_path: &PathBuf) -> Result<SigningKey, Box<dyn std::error::Error>> {
    if key_path.exists() {
        let content = fs::read_to_string(key_path)?;
        let key_json: serde_json::Value = serde_json::from_str(&content)?;
        let secret_hex = key_json["secret_key"].as_str().ok_or("Missing secret_key")?;
        let secret_bytes = hex::decode(secret_hex.trim())?;
        let secret_array: [u8; 32] = secret_bytes.as_slice().try_into()?;
        Ok(SigningKey::from_bytes(&secret_array)?)
    } else {
        let signing_key = SigningKey::generate();
        
        if let Some(parent) = key_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let key_data = serde_json::json!({
            "secret_key": hex::encode(signing_key.to_bytes()),
            "public_key": hex::encode(signing_key.public_key()),
            "key_type": "ed25519",
        });
        fs::write(key_path, serde_json::to_string_pretty(&key_data)?)?;
        Ok(signing_key)
    }
}

/// Run faucet service
async fn run_faucet() -> Result<(), Box<dyn std::error::Error>> {
    info!("💧 Starting Faucet Service...");
    
    let drip_amount = env::var("DRIP_AMOUNT")
        .unwrap_or_else(|_| "1000000000000000000000".to_string())
        .parse::<u128>()
        .unwrap();
    let cooldown = env::var("COOLDOWN_SECONDS")
        .unwrap_or_else(|_| "86400".to_string())
        .parse::<u64>()
        .unwrap();
    
    let config = node::faucet::FaucetConfig {
        drip_amount,
        cooldown_seconds: cooldown,
        max_requests_per_address: 10,
    };
    let faucet = Arc::new(Mutex::new(node::faucet::Faucet::new(config)));
    
    let route = {
        let faucet = faucet.clone();
        warp::path("faucet")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |req: serde_json::Value| {
                let faucet = faucet.clone();
                async move {
                    let address = match req.get("address").and_then(|v| v.as_str()) {
                        Some(addr) => addr,
                        None => return Ok::<warp::reply::Json, warp::Rejection>(warp::reply::json(&serde_json::json!({ "error": "Missing address" }))),
                    };
                    
                    let addr_bytes = match hex::decode(address.strip_prefix("0x").unwrap_or(address)) {
                        Ok(bytes) => bytes,
                        Err(_) => return Ok::<warp::reply::Json, warp::Rejection>(warp::reply::json(&serde_json::json!({ "error": "Invalid hex" }))),
                    };
                    
                    let addr_array: [u8; 20] = match addr_bytes.try_into() {
                        Ok(arr) => arr,
                        Err(_) => return Ok::<warp::reply::Json, warp::Rejection>(warp::reply::json(&serde_json::json!({ "error": "Invalid address length" }))),
                    };
                    
                    let mut faucet_guard = faucet.lock().await;
                    match faucet_guard.request_tokens(addr_array) {
                        Ok(amount) => {
                            info!("💸 Dripped {} tokens to {}", amount, address);
                            Ok::<warp::reply::Json, warp::Rejection>(warp::reply::json(&serde_json::json!({ 
                                "status": "success", 
                                "amount": amount.to_string()
                            })))
                        }
                        Err(e) => Ok::<warp::reply::Json, warp::Rejection>(warp::reply::json(&serde_json::json!({ "error": e }))),
                    }
                }
            })
    };
    
    info!("🌊 Faucet listening on http://0.0.0.0:3000/faucet");
    warp::serve(route).run(([0, 0, 0, 0], 3000)).await;
    
    Ok(())
}

// ============================================================================
// Main Entry Point
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(true)
        .with_thread_ids(true)
        .init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Start { config, genesis } => {
            let mut node = Node::new(config, genesis).await?;
            node.run().await?;
        }
        
        Commands::KeyGen { output } => {
            info!("🔑 Generating new validator keypair...");
            let signing_key = SigningKey::generate();
            let public_key = signing_key.public_key();
            
            let key_data = serde_json::json!({
                "secret_key": hex::encode(signing_key.to_bytes()),
                "public_key": hex::encode(&public_key),
                "key_type": "ed25519",
            });
            
            fs::write(&output, serde_json::to_string_pretty(&key_data)?)?;
            info!("✅ Keypair saved to {:?}", output);
            println!("Public Key: {}", hex::encode(&public_key));
        }
        
        Commands::SubmitTx { payload, rpc_url } => {
            let client = reqwest::Client::new();
            let url = format!("{}/submit_tx", rpc_url);
            
            let response = client
                .post(&url)
                .json(&serde_json::json!({ "payload": payload }))
                .send()
                .await?;
            
            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                println!("✅ Transaction submitted: {}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("❌ Submission failed: {}", response.status());
            }
        }
        
        Commands::QueryBalance { address, rpc_url } => {
            let url = format!("{}/balance/{}", rpc_url, address);
            let response = reqwest::get(&url).await?;
            
            if response.status().is_success() {
                let balance: serde_json::Value = response.json().await?;
                println!("Balance: {}", serde_json::to_string_pretty(&balance)?);
            } else {
                println!("❌ Query failed: {}", response.status());
            }
        }
        
        Commands::GetBlock { height, rpc_url } => {
            let url = format!("{}/block/{}", rpc_url, height);
            let response = reqwest::get(&url).await?;
            
            if response.status().is_success() {
                let block: serde_json::Value = response.json().await?;
                println!("Block: {}", serde_json::to_string_pretty(&block)?);
            } else {
                println!("❌ Query failed: {}", response.status());
            }
        }
        
        Commands::ConnectPeer { multiaddr, rpc_url } => {
            let client = reqwest::Client::new();
            let url = format!("{}/connect_peer", rpc_url);
            
            let response = client
                .post(&url)
                .json(&serde_json::json!({ "multiaddr": multiaddr }))
                .send()
                .await?;
            
            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                println!("✅ Connected: {}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("❌ Connection failed: {}", response.status());
            }
        }
        
        Commands::Faucet => {
            run_faucet().await?;
        }
        
        Commands::Status { rpc_url } => {
            let response = reqwest::get(&format!("{}/status", rpc_url)).await?;
            let status: serde_json::Value = response.json().await?;
            println!("Node Status: {}", serde_json::to_string_pretty(&status)?);
        }
    }
    
    Ok(())
}