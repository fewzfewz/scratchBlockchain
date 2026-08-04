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

#![recursion_limit = "256"]
//! - Consensus (BFT engine)
//! - Network (P2P layer)
//! - Mempool (transaction pool)
//! - Storage (block and state persistence)
//! - Rewards & Slashing (validator economics)
//! - RPC (API for clients)
//! - Metrics (monitoring)

use clap::{Parser, Subcommand};
use libp2p::request_response::{RequestId, ResponseChannel};
use libp2p::PeerId;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

// Import from our modules
mod config;
mod rewards;
mod sync;

use config::NodeConfig;
use rewards::{RewardConfig, RewardManager};

// External imports
use common::consensus_types::{ConsensusMessage, Proposal, Step, Vote};
use common::crypto::SigningKey;
use common::traits::Consensus;
use common::types::{Block, Header, Transaction};

use consensus::bft::{BftEngine, BftEvent};
use consensus::slashing::{SlashingConfig, SlashingTracker};
use consensus::{EnhancedConsensus, FinalityGadget, SlashingCondition, ValidatorInfo};

use execution::evm::EvmExecutor;
use node::evm_store::ChainStoreEvmStore;

use network::protocol::BlockResponse;
use network::{NetworkCommand, NetworkEvent, NetworkService};

use node::tx_pool::TxPool;

use storage::receipt_store::ReceiptStore;
use storage::trie::PatriciaTrie;
use storage::{ChainStore, ColumnFamily, PruneConfig, WriteBatch};

use node::block_producer::{BlockExecutor, BlockProducer, BlockProducerConfig};
use node::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use node::fork_choice::ForkChoice;
use node::light_client::{LightClient, SyncCommittee, SyncCommitteeManager};
use node::metrics::Metrics;
use node::runtime_upgrade::RuntimeUpgradeManager;

/// Seconds without a finalized block before a node attempts to re-sync from peers.
const SYNC_GRACE_SECS: u64 = 20;
/// Number of blocks requested per sync batch.
const SYNC_BATCH_SIZE: u32 = 128;

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

        #[arg(
            long,
            help = "Path to genesis JSON file",
            default_value = "genesis.json"
        )]
        genesis: PathBuf,
    },

    /// Generate a new validator keypair
    KeyGen {
        #[arg(
            long,
            help = "Output path for the key file",
            default_value = "validator_key.json"
        )]
        output: PathBuf,
    },

    /// Submit a transaction to the node
    SubmitTx {
        #[arg(long, help = "Transaction payload (hex or plaintext)")]
        payload: String,

        #[arg(
            long,
            help = "RPC endpoint URL",
            default_value = "http://localhost:9933"
        )]
        rpc_url: String,
    },

    /// Query account balance
    QueryBalance {
        #[arg(long, help = "Account address (hex with 0x prefix)")]
        address: String,

        #[arg(
            long,
            help = "RPC endpoint URL",
            default_value = "http://localhost:9933"
        )]
        rpc_url: String,
    },

    /// Get block by height
    GetBlock {
        #[arg(long, help = "Block height")]
        height: u64,

        #[arg(
            long,
            help = "RPC endpoint URL",
            default_value = "http://localhost:9933"
        )]
        rpc_url: String,
    },

    /// Connect to a specific peer
    ConnectPeer {
        #[arg(long, help = "Peer multiaddress (e.g., /ip4/192.168.1.1/tcp/9000)")]
        multiaddr: String,

        #[arg(
            long,
            help = "RPC endpoint URL",
            default_value = "http://localhost:9933"
        )]
        rpc_url: String,
    },

    /// Start the faucet service (testnet token dispenser)
    Faucet,

    /// Show node status and statistics
    Status {
        #[arg(
            long,
            help = "RPC endpoint URL",
            default_value = "http://localhost:9933"
        )]
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

    // Transaction pool (mempool + MEV + account abstraction)
    tx_pool: Arc<TxPool>,

    // Slashing tracker
    slashing_tracker: Arc<Mutex<SlashingTracker>>,

    // Network
    network_cmd_sender: Arc<tokio::sync::mpsc::Sender<NetworkCommand>>,
    network_event_receiver: tokio::sync::mpsc::Receiver<NetworkEvent>,

    // Monitoring
    metrics: Arc<Metrics>,
    circuit_breaker: Arc<CircuitBreaker>,
    stats: NodeStats,

    // Persistence
    bft_state_path: PathBuf,

    // Block sync / catch-up state
    syncing: bool,
    last_sync_attempt_at: Instant,
    last_finalize_at: Instant,

    // State pruning
    prune_config: PruneConfig,
}

impl Node {
    async fn new(
        config_path: PathBuf,
        genesis_path: PathBuf,
    ) -> Result<Self, Box<dyn std::error::Error>> {
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
            let genesis_config: common::types::GenesisConfig =
                serde_json::from_str(&genesis_content)?;

            // Initialize genesis accounts in state trie
            for account in genesis_config.accounts {
                let account_json = serde_json::to_vec(&account)?;
                state_trie
                    .lock()
                    .await
                    .insert(&account.address, &account_json)?;
            }

            // Initialize genesis validators in state trie (for /validators RPC)
            let validator_infos: Vec<serde_json::Value> = genesis_config
                .validators
                .iter()
                .map(|v| {
                    serde_json::json!({
                        "address": format!("0x{}", hex::encode(v.address)),
                        "public_key": v.public_key,
                        "stake": v.stake.to_string(),
                        "commission_rate": (v.commission_rate * 100.0) as u64,
                        "is_active": true,
                        "blocks_produced": 0u64,
                        "blocks_missed": 0u64,
                        "delegator_count": 0u64,
                        "total_delegated": "0",
                    })
                })
                .collect();
            let validators_json = serde_json::to_vec(&validator_infos)?;
            state_trie
                .lock()
                .await
                .insert(b"validators", &validators_json)?;
            info!(
                "✅ Initialized {} validators in state trie",
                validator_infos.len()
            );

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

        let genesis_validators: Vec<ValidatorInfo> = genesis_config
            .validators
            .iter()
            .map(|v| ValidatorInfo {
                public_key: hex::decode(&v.public_key).unwrap_or_default(),
                stake: v.stake as u64,
                slashed: false,
            })
            .collect();

        let chain_tip = chain_store.get_latest_height()?.unwrap_or(0);
        let validators: Vec<ValidatorInfo> = if chain_tip > 0 {
            match node::governance_store::load_consensus_validators(&state_trie).await {
                Ok(v) if !v.is_empty() => {
                    info!("✅ Loaded {} validators from state trie", v.len());
                    v
                }
                _ => {
                    info!(
                        "✅ Loaded {} validators from genesis (trie empty)",
                        genesis_validators.len()
                    );
                    genesis_validators.clone()
                }
            }
        } else {
            info!(
                "✅ Loaded {} validators from genesis",
                genesis_validators.len()
            );
            genesis_validators.clone()
        };

        // ================================================================
        // 6. Initialize Transaction Pool (MEV + account abstraction)
        // ================================================================
        let mempool_config = mempool::MempoolConfig {
            max_capacity: (config.security.max_tx_per_block * 2) as usize,
            max_per_sender: 100,
            min_fee_per_gas: 1_000_000_000,
            chain_id: Some(1),
        };
        let validator_pubkeys: Vec<Vec<u8>> =
            validators.iter().map(|v| v.public_key.clone()).collect();
        let tx_pool = Arc::new(TxPool::new(mempool_config, validator_pubkeys));

        // ================================================================
        // 6b. Initialize Slashing Tracker
        // ================================================================
        let slashing_tracker =
            Arc::new(Mutex::new(SlashingTracker::new(SlashingConfig::default())));

        // ================================================================
        // 7. Initialize BFT Engine
        // ================================================================
        let bft_state_path = PathBuf::from(data_dir).join("bft_state.json");
        // Resume consensus from the persisted chain tip on restart. A fresh chain
        // (only the genesis block at slot 0) keeps the historical start (slot 0,
        // BFT height 1); an existing chain continues at slot tip+1 / height tip+2
        // so restarts don't re-produce blocks from slot 0 and break quorum.
        let (producer_start_slot, bft_start_height) = if chain_tip == 0 {
            (0u64, 1u64)
        } else {
            (chain_tip + 1, chain_tip + 2)
        };
        let bft_engine = Arc::new(Mutex::new(BftEngine::new(
            public_key.clone(),
            validators.clone(),
            bft_start_height,
            signing_key.clone(),
        )));

        // ================================================================
        // 8. Initialize Finality Gadget
        // ================================================================
        let finality_gadget = Arc::new(Mutex::new(FinalityGadget::new(validators.clone())));

        // ================================================================
        // 9. Initialize Block Executor
        // ================================================================
        let chain_id: u64 = config.network.chain_id.parse().unwrap_or(1);
        let evm_store = Arc::new(ChainStoreEvmStore::new(chain_store.clone()));
        let block_executor = BlockExecutor::new(
            EvmExecutor::with_store(evm_store.clone(), chain_id),
            chain_store.clone(),
        );
        let block_executor = Arc::new(Mutex::new(block_executor));

        // ================================================================
        // 11. Initialize Block Producer
        // ================================================================
        let block_producer_config = BlockProducerConfig {
            max_transactions_per_block: config.security.max_tx_per_block,
            max_gas_per_block: config.security.max_gas_per_block,
            target_gas_utilization: 0.5,
        };

        let producer_executor = BlockExecutor::new(
            EvmExecutor::with_store(evm_store, chain_id),
            chain_store.clone(),
        );
        let block_producer = Arc::new(Mutex::new(BlockProducer::new(
            tx_pool.clone(),
            bft_engine.clone(),
            producer_executor,
            signing_key.clone(),
            public_key.clone().try_into().unwrap_or([0u8; 20]),
            block_producer_config,
        )));
        {
            let mut producer = block_producer.lock().await;
            producer.set_current_slot(producer_start_slot);
        }

        // ================================================================
        // 12. Initialize Economic Components
        // ================================================================
        let reward_config = RewardConfig::default();
        let reward_manager = Arc::new(Mutex::new(RewardManager::new(reward_config)));

        // ================================================================
        // 13. Initialize Network
        // ================================================================
        let peer_store_path = format!("{}/peers.json", data_dir);
        let (network, network_cmd_sender, network_event_receiver) =
            NetworkService::new(config.network.bootstrap_nodes.clone(), &peer_store_path)?;
        let network_cmd_sender = Arc::new(network_cmd_sender);

        // Start listening
        let p2p_addr: libp2p::Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", config.network.p2p_port)
            .parse()
            .expect("Invalid P2P address");
        network_cmd_sender
            .send(NetworkCommand::StartListening(p2p_addr))
            .await?;

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
            tx_pool.clone(),
            chain_store.clone(),
            state_trie.clone(),
            metrics.clone(),
            (*network_cmd_sender).clone(),
            slashing_tracker.clone(),
            config.api.rate_limit,
            chain_id,
        );

        let rpc_port = config.network.rpc_port;
        let enable_cors = !config.api.cors_origins.is_empty();
        let rpc_server = Arc::new(rpc_server);
        tokio::spawn({
            let rpc_server = rpc_server.clone();
            async move {
                info!("🔌 RPC server listening on port {}", rpc_port);
                rpc_server.run(rpc_port, enable_cors).await;
            }
        });

        if config.api.enabled {
            let api_port = api_port_from_address(&config.api.address);
            if let Some(api_port) = api_port {
                tokio::spawn({
                    let rpc_server = rpc_server.clone();
                    async move {
                        info!("🔌 API server listening on port {}", api_port);
                        rpc_server.run(api_port, enable_cors).await;
                    }
                });
            } else {
                warn!(
                    "⚠️ Invalid API address {:?}, API server not started",
                    config.api.address
                );
            }
        }

        // Ensure on-chain governance state exists (seeds it for fresh chains and
        // for existing chains that predate governance).
        node::governance_store::load_or_init(&state_trie).await?;

        let prune_config = PruneConfig::from_mode(
            &config.storage.pruning_mode,
            Some(config.storage.blocks_to_keep),
            Some(config.storage.prune_every_n_blocks),
        );
        info!(
            "Storage pruning mode={} keep={} every={} blocks",
            prune_config.mode, prune_config.blocks_to_keep, prune_config.prune_every_n_blocks
        );

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
            tx_pool,
            slashing_tracker,
            network_cmd_sender,
            network_event_receiver,
            metrics,
            circuit_breaker,
            stats: NodeStats::new(),
            bft_state_path,
            syncing: false,
            last_sync_attempt_at: Instant::now(),
            last_finalize_at: Instant::now(),
            prune_config,
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

        let mut consensus_interval = interval(Duration::from_millis(1));
        let mut metrics_interval = interval(Duration::from_secs(5));
        let mut sync_interval = interval(Duration::from_secs(5));
        let mut repropose_interval = interval(Duration::from_secs(1));
        let mut bft_align_interval = interval(Duration::from_secs(10));

        loop {
            // Process BFT events
            let mut new_events = Vec::new();
            for event in pending_events.drain(..) {
                if let Err(e) = self.handle_bft_event(event, &mut new_events).await {
                    warn!("Error handling BFT event: {}", e);
                }
            }
            pending_events.extend(new_events);

            tokio::select! {
                _ = consensus_interval.tick() => {
                    let mut engine = self.bft_engine.lock().await;
                    if let Some(timeout) = engine.check_timeout() {
                        pending_events.push(timeout);
                    }
                }

                _ = metrics_interval.tick() => {
                    if let Err(e) = self.update_live_metrics().await {
                        warn!("Error updating metrics: {}", e);
                    }
                    if let Err(e) = self.sync_validator_set().await {
                        warn!("Validator set sync failed: {}", e);
                    }
                }

                _ = sync_interval.tick() => {
                    if let Err(e) = self.maybe_trigger_sync().await {
                        warn!("Error in sync tick: {}", e);
                    }
                }

                _ = repropose_interval.tick() => {
                    // Re-broadcast our proposal so peers that entered this
                    // height/round late still receive it and can vote.
                    let events = {
                        let mut engine = self.bft_engine.lock().await;
                        engine.re_propose()
                    };
                    pending_events.extend(events);
                }

                _ = bft_align_interval.tick() => {
                    // Re-anchor BFT height if it drifted from the chain tip.
                    let tip = self.chain_store.get_latest_height()?.unwrap_or(0);
                    if tip > 0 {
                        let expected_bft = tip + 2;
                        let mut engine = self.bft_engine.lock().await;
                        if engine.height.saturating_add(1) < expected_bft
                            || engine.height > expected_bft + 2
                        {
                            warn!(
                                "BFT height {} out of sync with chain tip {} — re-anchoring to {}",
                                engine.height, tip, expected_bft
                            );
                            let events = engine.reset_to_height(expected_bft);
                            pending_events.extend(events);
                            drop(engine);
                            self.block_producer.lock().await.set_current_slot(tip + 1);
                        }
                    }
                }

                Some(event) = self.network_event_receiver.recv() => {
                    if let Err(e) = self.handle_network_event(event, &mut pending_events).await {
                        warn!("Error handling network event: {}", e);
                    }
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        match event {
            BftEvent::BroadcastVote(vote) => {
                debug!("Broadcasting vote for round {}", vote.round);
                self.network_cmd_sender
                    .send(NetworkCommand::BroadcastConsensusMessage(
                        ConsensusMessage::Vote(vote),
                    ))
                    .await?;
            }

            BftEvent::BroadcastProposal(proposal) => {
                info!("Broadcasting proposal for round {}", proposal.round);
                self.network_cmd_sender
                    .send(NetworkCommand::BroadcastConsensusMessage(
                        ConsensusMessage::Proposal(proposal),
                    ))
                    .await?;
            }

            BftEvent::FinalizeBlock(block) => {
                info!("🔒 Finalizing block at height {}", block.header.slot);
                self.last_finalize_at = Instant::now();

                // Execute and commit block
                let receipts = self
                    .block_executor
                    .lock()
                    .await
                    .execute_and_commit(&block)?;

                // Calculate total fees
                let total_fees: u64 = receipts
                    .iter()
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
                        info!(
                            "💰 Rewarded {} with {} tokens",
                            hex::encode(&validator[..4]),
                            amount
                        );
                    }
                }

                // Apply on-chain governance actions included in this block
                if let Err(e) = node::governance_store::apply_extrinsics(
                    &self.state_trie,
                    &block.extrinsics,
                    block.header.slot,
                )
                .await
                {
                    warn!("Error applying governance actions: {}", e);
                }

                // Update metrics
                self.metrics.record_block();
                self.metrics.update_finalized_height(block.header.slot);
                self.stats.record_block(block.extrinsics.len());

                // Remove transactions from pool
                self.tx_pool.remove_transactions(&block.extrinsics);

                // Slashing: advance block height tracker
                {
                    let mut tracker = self.slashing_tracker.lock().await;
                    tracker.update_height(block.header.slot);
                }

                // Economic engine: burn 50% of fees; 10% to treasury
                if total_fees > 0 {
                    let burned = total_fees / 2;
                    info!("🔥 Fee burn: {} tokens burned from block fees", burned);
                    if let Err(e) = node::governance_store::collect_treasury_fee(
                        &self.state_trie,
                        total_fees / 10,
                    )
                    .await
                    {
                        warn!("Treasury fee collection failed: {}", e);
                    }
                }

                // Prune old blocks/receipts below retention window
                match self
                    .chain_store
                    .maybe_prune(&self.prune_config, block.header.slot)
                {
                    Ok(stats) if stats.blocks_pruned > 0 => {
                        info!(
                            "🧹 Pruned {} blocks, {} receipts (cutoff height {})",
                            stats.blocks_pruned, stats.receipts_pruned, stats.to_height
                        );
                        if let Ok(removed) = self.state_trie.lock().await.gc_orphan_nodes() {
                            if removed > 0 {
                                info!("🧹 Trie GC removed {} orphan nodes", removed);
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(e) => warn!("State pruning failed: {}", e),
                }

                // Hot-reload BFT validator set from trie (post-governance extrinsics)
                if let Err(e) = self.sync_validator_set().await {
                    warn!("Validator set sync failed: {}", e);
                }

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
                    match producer.produce_block().await {
                        Ok(block) => {
                            info!(
                                "✅ Produced block with {} transactions",
                                block.extrinsics.len()
                            );
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
                if let Err(e) = self.tx_pool.add_transaction(tx) {
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

            NetworkEvent::BlockRequestReceived {
                peer,
                request_id,
                start_height,
                limit,
                channel,
            } => {
                self.handle_block_request(peer, request_id, start_height, limit, channel)
                    .await?;
            }

            NetworkEvent::BlockResponseReceived { peer, blocks, .. } => {
                self.handle_block_response(peer, blocks, pending_events)
                    .await?;
            }

            NetworkEvent::ListeningOn(addr) => {
                info!("🔊 Network listening on {}", addr);
            }

            _ => {}
        }

        Ok(())
    }

    /// Handle an incoming block from gossip.
    ///
    /// Only blocks that extend our chain tip by exactly one (with a matching
    /// parent) are applied immediately. A block that skips heights indicates a
    /// gap in our chain — we request a contiguous sync from the sender instead
    /// of committing it out of order (which previously let the chain fork or
    /// regress). Blocks behind our tip are ignored.
    async fn handle_incoming_block(
        &mut self,
        block: Block,
        source: PeerId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check if we already have this block
        let block_hash = block.hash();
        if self.chain_store.get_block(&block_hash)?.is_some() {
            debug!("Block already known, ignoring");
            return Ok(());
        }

        let tip = self.chain_store.get_latest_height()?.unwrap_or(0);
        let block_slot = block.header.slot;

        // Ignore genesis-shadow blocks delivered via gossip; the canonical
        // genesis lives at slot 0 and the first produced block is committed
        // through the BFT finalize path.
        if block_slot == 0 {
            return Ok(());
        }

        // A block ahead of our tip means we are missing intermediate blocks.
        // Trigger a contiguous sync from the sender to fill the gap.
        if block_slot > tip + 1 {
            info!(
                "Block at slot {} ahead of tip {} — requesting sync",
                block_slot, tip
            );
            if !self.syncing {
                self.syncing = true;
                self.network_cmd_sender
                    .send(NetworkCommand::RequestBlock {
                        peer: source,
                        start_height: if tip == 0 { 0 } else { tip + 1 },
                        limit: SYNC_BATCH_SIZE,
                    })
                    .await?;
            }
            return Ok(());
        }

        // Stale block (already past).
        if block_slot <= tip {
            return Ok(());
        }

        // block_slot == tip + 1: must extend our current tip.
        let tip_hash = self.current_tip_hash()?;
        if block.header.parent_hash != tip_hash {
            warn!(
                "Block at slot {} has mismatched parent (fork) — ignoring",
                block_slot
            );
            return Ok(());
        }

        if let Err(e) = self.apply_block(&block).await {
            warn!("Failed to apply gossip block at slot {}: {}", block_slot, e);
        }
        Ok(())
    }

    /// Verify, execute, and commit a block that we know is a contiguous
    /// extension of our chain. Shared by the gossip path and the sync path.
    async fn apply_block(&mut self, block: &Block) -> Result<(), Box<dyn std::error::Error>> {
        // Verify block signature by a known validator
        let validators = self.get_validator_list().await?;
        let consensus = EnhancedConsensus::new(validators);
        if let Err(e) = consensus.verify_block(block) {
            warn!("Invalid block: {}", e);
            return Err(format!("Invalid block: {}", e).into());
        }

        // Execute block
        let receipts = self.block_executor.lock().await.execute_and_commit(block)?;

        // Verify state root when the header records one (non-zero).
        if block.header.state_root.iter().any(|b| *b != 0) {
            let parent_root = if block.header.slot == 0 {
                [0u8; 32]
            } else {
                match self
                    .chain_store
                    .get_block_hash_by_height(block.header.slot.saturating_sub(1))
                {
                    Ok(Some(hash_bytes)) if hash_bytes.len() == 32 => {
                        let mut hash = [0u8; 32];
                        hash.copy_from_slice(&hash_bytes);
                        if let Ok(Some(encoded)) = self.chain_store.get_block(&hash) {
                            if let Ok(parent) = serde_json::from_slice::<Block>(&encoded) {
                                parent.header.state_root
                            } else {
                                [0u8; 32]
                            }
                        } else {
                            [0u8; 32]
                        }
                    }
                    _ => [0u8; 32],
                }
            };
            let computed = node::block_producer::BlockProducer::compute_state_root(
                &parent_root,
                &block.header.extrinsics_root,
            );
            if computed != block.header.state_root {
                warn!("State root mismatch");
                return Err("State root mismatch".into());
            }
        }

        // Process rewards
        let total_fees: u64 = receipts
            .iter()
            .map(|r| r.gas_used * block.header.base_fee)
            .sum();
        let proposer = block.header.validator_set_id.to_le_bytes().to_vec();
        let voters = vec![];

        let rewards = {
            let mut rm = self.reward_manager.lock().await;
            rm.process_block(block, proposer, &voters, total_fees)
        };
        for (validator, amount) in rewards {
            if amount > 0 {
                self.apply_reward(&validator, amount as u64).await?;
            }
        }

        // Apply on-chain governance actions included in this block
        if let Err(e) = node::governance_store::apply_extrinsics(
            &self.state_trie,
            &block.extrinsics,
            block.header.slot,
        )
        .await
        {
            warn!("Error applying governance actions: {}", e);
        }

        // Update mempool
        self.tx_pool.remove_transactions(&block.extrinsics);

        info!("✅ Processed block at height {}", block.header.slot);
        self.metrics.record_block();
        self.stats.record_block(block.extrinsics.len());

        Ok(())
    }

    /// Hash of the block currently at our chain tip.
    fn current_tip_hash(&self) -> Result<[u8; 32], Box<dyn std::error::Error>> {
        match self.chain_store.get_latest_height()? {
            Some(height) => match self.chain_store.get_block_by_height(height)? {
                Some(data) => Ok(serde_json::from_slice::<Block>(&data)?.hash()),
                None => Ok(Block::genesis().hash()),
            },
            None => Ok(Block::genesis().hash()),
        }
    }

    /// Periodically check whether consensus has stalled and, if so, request a
    /// contiguous block sync from a connected peer so the node can catch up.
    async fn maybe_trigger_sync(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.syncing {
            return Ok(());
        }
        // Don't hammer peers if a previous sync attempt found nothing.
        if self.last_sync_attempt_at.elapsed() < Duration::from_secs(SYNC_GRACE_SECS) {
            return Ok(());
        }
        if self.last_finalize_at.elapsed() < Duration::from_secs(SYNC_GRACE_SECS) {
            return Ok(());
        }

        let tip = self.chain_store.get_latest_height()?.unwrap_or(0);

        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.network_cmd_sender
            .send(NetworkCommand::ListConnectedPeers(reply_tx))
            .await?;
        let peers = reply_rx.await.unwrap_or_default();
        let peer = peers.into_iter().next();

        match peer {
            Some(peer) => {
                info!(
                    "⚠️ Consensus stalled at height {} for {}s — requesting sync from {}",
                    tip, SYNC_GRACE_SECS, peer
                );
                self.syncing = true;
                self.network_cmd_sender
                    .send(NetworkCommand::RequestBlock {
                        peer,
                        start_height: if tip == 0 { 0 } else { tip + 1 },
                        limit: SYNC_BATCH_SIZE,
                    })
                    .await?;
            }
            None => {
                debug!("Consensus stalled but no peers connected — will retry");
            }
        }
        Ok(())
    }

    /// Serve a block-sync request from a peer with the blocks we have stored.
    async fn handle_block_request(
        &self,
        peer: PeerId,
        _request_id: RequestId,
        start_height: u64,
        limit: u32,
        channel: ResponseChannel<BlockResponse>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut blocks = Vec::new();
        let tip = self.chain_store.get_latest_height()?.unwrap_or(0);
        if start_height <= tip {
            let end = (start_height + limit as u64 - 1).min(tip);
            for height in start_height..=end {
                if let Some(data) = self.chain_store.get_block_by_height(height)? {
                    if let Ok(block) = serde_json::from_slice::<Block>(&data) {
                        blocks.push(block);
                    }
                }
            }
        }
        debug!(
            "Serving {} blocks to {} starting at height {}",
            blocks.len(),
            peer,
            start_height
        );
        self.network_cmd_sender
            .send(NetworkCommand::SendBlockResponse { channel, blocks })
            .await?;
        Ok(())
    }

    /// Apply a batch of blocks received in response to a sync request.
    ///
    /// Blocks must arrive in contiguous order. The first block may be a
    /// replacement of our current tip (e.g. the produced slot-0 block replacing
    /// the genesis entry at height 0) — that is adopted if it differs. When the
    /// peer's chain has been fully fetched the BFT engine is reset to the new
    /// chain tip so consensus resumes on the canonical chain.
    async fn handle_block_response(
        &mut self,
        peer: PeerId,
        blocks: Vec<Block>,
        pending_events: &mut Vec<BftEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !self.syncing {
            return Ok(());
        }

        let total_received = blocks.len();
        let mut applied = 0usize;
        let mut advanced = 0usize;

        for block in blocks {
            let tip = self.chain_store.get_latest_height()?.unwrap_or(0);
            let slot = block.header.slot;

            // Replacement of our current tip (handles genesis → produced block 0).
            if slot == tip {
                let tip_hash = self.current_tip_hash()?;
                if block.hash() != tip_hash {
                    if let Err(e) = self.apply_block(&block).await {
                        warn!(
                            "Sync: failed to replace tip at slot {}: {} — stopping",
                            slot, e
                        );
                        break;
                    }
                    applied += 1;
                }
                continue;
            }

            if slot != tip + 1 {
                warn!(
                    "Sync: block at slot {} not contiguous with tip {} — stopping",
                    slot, tip
                );
                break;
            }
            if block.header.parent_hash != self.current_tip_hash()? {
                warn!(
                    "Sync: block at slot {} has mismatched parent — stopping",
                    slot
                );
                break;
            }

            if let Err(e) = self.apply_block(&block).await {
                warn!(
                    "Sync: failed to apply block at slot {}: {} — stopping",
                    slot, e
                );
                break;
            }
            applied += 1;
            advanced += 1;
        }

        if applied == 0 {
            warn!("Sync: no blocks applied from {}", peer);
            self.syncing = false;
            self.last_sync_attempt_at = Instant::now();
            return Ok(());
        }

        let tip = self.chain_store.get_latest_height()?.unwrap_or(0);
        debug!(
            "Sync: applied {} blocks (advanced {}) — tip now {}",
            applied, advanced, tip
        );

        // If the peer sent fewer blocks than the batch size, we've reached the
        // end of its chain. Reset consensus to the synced tip and resume.
        if total_received < SYNC_BATCH_SIZE as usize && advanced > 0 {
            self.finish_sync(pending_events).await?;
            return Ok(());
        }

        // Fetch the next contiguous batch from the same peer.
        let start = if tip == 0 { 0 } else { tip + 1 };
        self.network_cmd_sender
            .send(NetworkCommand::RequestBlock {
                peer,
                start_height: start,
                limit: SYNC_BATCH_SIZE,
            })
            .await?;
        Ok(())
    }

    /// Stop syncing and re-align the BFT engine with the (now caught-up) chain
    /// tip so the node resumes producing/voting on the canonical chain.
    async fn finish_sync(
        &mut self,
        pending_events: &mut Vec<BftEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.syncing = false;
        self.last_sync_attempt_at = Instant::now();

        let tip = self.chain_store.get_latest_height()?.unwrap_or(0);
        if tip == 0 {
            return Ok(());
        }

        // BFT height h finalizes block slot h-1, so a chain tip of `tip`
        // corresponds to resuming consensus at height tip+2.
        let bft_height = tip + 2;
        let events = {
            let mut engine = self.bft_engine.lock().await;
            engine.reset_to_height(bft_height)
        };
        {
            let mut producer = self.block_producer.lock().await;
            producer.set_current_slot(tip + 1);
        }
        info!(
            "🔄 Chain re-synced to height {} — BFT resumed at height {}",
            tip, bft_height
        );
        pending_events.extend(events);
        Ok(())
    }

    /// Get current validator list
    async fn get_validator_list(&self) -> Result<Vec<ValidatorInfo>, Box<dyn std::error::Error>> {
        Ok(
            node::governance_store::load_consensus_validators(&self.state_trie)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?,
        )
    }

    /// Reload BFT + finality validator sets from the state trie when changed.
    async fn sync_validator_set(&self) -> Result<(), Box<dyn std::error::Error>> {
        let validators =
            node::governance_store::load_consensus_validators(&self.state_trie).await?;
        if validators.is_empty() {
            return Ok(());
        }
        let mut engine = self.bft_engine.lock().await;
        if engine.validator_count() != validators.len() {
            let count = validators.len();
            engine.update_validator_set(validators.clone());
            drop(engine);
            self.finality_gadget
                .lock()
                .await
                .update_validator_set(validators);
            info!("🔄 BFT validator set hot-reloaded ({} validators)", count);
        }
        Ok(())
    }

    /// Periodically refresh live metrics (peers, mempool, consensus round, network bytes)
    async fn update_live_metrics(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Mempool size
        self.metrics.update_mempool_size(self.tx_pool.size());

        // Consensus round
        let round = self.bft_engine.lock().await.round;
        self.metrics.update_consensus_round(round);

        // Validator set (count + total stake)
        let validators = self.get_validator_list().await?;
        let stake_total = validators
            .iter()
            .map(|v| v.stake)
            .fold(0u64, |acc, s| acc.saturating_add(s));
        self.metrics
            .update_validator_set(validators.len(), stake_total);

        // Network stats (peer count, connection count, gossip bytes)
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.network_cmd_sender
            .send(NetworkCommand::GetStats(reply_tx))
            .await?;
        if let Ok(stats) = reply_rx.await {
            self.metrics.update_peer_count(stats.peer_count);
            self.metrics.update_network_bytes_rx(stats.bytes_rx);
            self.metrics.update_network_bytes_tx(stats.bytes_tx);
        }
        Ok(())
    }

    /// Apply reward to validator
    async fn apply_reward(
        &mut self,
        validator: &[u8],
        amount: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
        self.network_cmd_sender
            .send(NetworkCommand::SavePeers)
            .await?;
        info!(
            "Node shutdown complete. Processed {} blocks at {:.2} blocks/sec",
            self.stats.blocks_processed,
            self.stats.get_throughput()
        );
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
        let secret_hex = key_json["secret_key"]
            .as_str()
            .ok_or("Missing secret_key")?;
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
                        None => {
                            return Ok::<warp::reply::Json, warp::Rejection>(warp::reply::json(
                                &serde_json::json!({ "error": "Missing address" }),
                            ))
                        }
                    };

                    let addr_bytes =
                        match hex::decode(address.strip_prefix("0x").unwrap_or(address)) {
                            Ok(bytes) => bytes,
                            Err(_) => {
                                return Ok::<warp::reply::Json, warp::Rejection>(warp::reply::json(
                                    &serde_json::json!({ "error": "Invalid hex" }),
                                ))
                            }
                        };

                    let addr_array: [u8; 20] = match addr_bytes.try_into() {
                        Ok(arr) => arr,
                        Err(_) => {
                            return Ok::<warp::reply::Json, warp::Rejection>(warp::reply::json(
                                &serde_json::json!({ "error": "Invalid address length" }),
                            ))
                        }
                    };

                    let mut faucet_guard = faucet.lock().await;
                    match faucet_guard.request_tokens(addr_array) {
                        Ok(amount) => {
                            info!("💸 Dripped {} tokens to {}", amount, address);
                            Ok::<warp::reply::Json, warp::Rejection>(warp::reply::json(
                                &serde_json::json!({
                                    "status": "success",
                                    "amount": amount.to_string()
                                }),
                            ))
                        }
                        Err(e) => Ok::<warp::reply::Json, warp::Rejection>(warp::reply::json(
                            &serde_json::json!({ "error": e }),
                        )),
                    }
                }
            })
    };

    let faucet_port = env::var("FAUCET_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3006);
    info!(
        "🌊 Faucet listening on http://0.0.0.0:{}/faucet",
        faucet_port
    );
    warp::serve(route).run(([0, 0, 0, 0], faucet_port)).await;

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
                println!(
                    "✅ Transaction submitted: {}",
                    serde_json::to_string_pretty(&result)?
                );
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

/// Extract the port from an API address like "0.0.0.0:8545".
/// Returns None if the address has no usable port.
fn api_port_from_address(address: &str) -> Option<u16> {
    let addr = address.trim();
    let port_str = if addr.starts_with('[') {
        // IPv6 like [::1]:8545
        let end = addr.rfind(':')?;
        &addr[end + 1..]
    } else {
        match addr.rfind(':') {
            Some(idx) => &addr[idx + 1..],
            None => addr,
        }
    };
    port_str.trim().parse::<u16>().ok()
}
