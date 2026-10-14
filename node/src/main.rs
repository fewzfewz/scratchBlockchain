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
use tracing::{debug, info, warn};

// Import from our modules
mod config;
mod rewards;
mod sync;

use config::NodeConfig;
use rewards::{RewardConfig, RewardManager};

// External imports
use common::consensus_types::{ConsensusMessage, Step};
use common::crypto::SigningKey;
use common::traits::Consensus;
use common::types::Block;

use consensus::bft::{BftEngine, BftEvent, BftSafetyState};
use consensus::slashing::{SlashingConfig, SlashingTracker};
use consensus::{EnhancedConsensus, FinalityGadget, ValidatorInfo};

use execution::evm::EvmExecutor;
use node::evm_store::ChainStoreEvmStore;

use network::protocol::BlockResponse;
use network::{NetworkCommand, NetworkEvent, NetworkService};

use node::tx_pool::TxPool;

use storage::receipt_store::ReceiptStore;
use storage::trie::PatriciaTrie;
use storage::{ChainStore, KeyValueStore, PruneConfig, StagedStore};

use node::block_producer::{BlockExecutor, BlockProducer, BlockProducerConfig};
use node::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use node::metrics::Metrics;

/// Seconds without a finalized block before a node attempts to re-sync from peers.
const SYNC_GRACE_SECS: u64 = 20;
/// Number of blocks requested per sync batch.
const SYNC_BATCH_SIZE: u32 = 128;
/// A sync request left in-flight longer than this is presumed dead (the peer
/// stopped responding without emitting an OutboundFailure) and is retried.
const SYNC_STUCK_SECS: u64 = 10;

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

/// BFT state persisted for crash recovery: the lock and the signing
/// watermark. Written (atomically) before every signed vote or proposal is
/// broadcast, so a restarted validator cannot vote against its lock or sign
/// twice for the same height/round/step.
#[derive(serde::Serialize, serde::Deserialize)]
struct BftPersistedState {
    safety: BftSafetyState,
    last_saved: u64,
}

/// Load persisted BFT safety state. A missing or unreadable file (e.g. the
/// pre-Phase-2 format) yields `None`.
fn load_bft_state(path: &PathBuf) -> Option<BftSafetyState> {
    let content = fs::read_to_string(path).ok()?;
    match serde_json::from_str::<BftPersistedState>(&content) {
        Ok(state) => Some(state.safety),
        Err(e) => {
            warn!("Ignoring unreadable BFT state at {}: {}", path.display(), e);
            None
        }
    }
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
    #[allow(dead_code)]
    config: NodeConfig,
    chain_store: Arc<ChainStore>,
    /// Write stage over the database: each block's whole state transition is
    /// committed as one atomic batch (or discarded).
    staged: Arc<StagedStore>,
    #[allow(dead_code)]
    receipt_store: Arc<ReceiptStore>,
    state_trie: Arc<Mutex<PatriciaTrie>>,
    chain_id: u64,
    max_gas_per_block: u64,
    /// Set when our state diverged from a finalized block; the node stops.
    fatal_error: Option<String>,

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
    #[allow(dead_code)]
    circuit_breaker: Arc<CircuitBreaker>,
    stats: NodeStats,

    // Persistence
    bft_state_path: PathBuf,

    // Block sync / catch-up state
    syncing: bool,
    /// Rotates sync requests across connected peers, so one peer that
    /// stops answering cannot stall catch-up forever.
    sync_peer_cursor: usize,
    /// Peer the in-flight sync request went to.
    sync_peer: Option<PeerId>,
    last_sync_attempt_at: Instant,
    last_finalize_at: Instant,
    /// Set when the consensus engine signals that we are behind peers
    /// (BehindSync/NeedBlock) so the next sync tick triggers a fetch promptly.
    sync_requested: bool,

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

        let base_store = ChainStore::open(data_dir)?.inner().clone();
        let staged = Arc::new(StagedStore::new(base_store));
        let kv_store: Arc<dyn storage::KeyValueStore> = staged.clone();
        let chain_store = Arc::new(ChainStore::new(kv_store.clone()));
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

            // The whole genesis state is written as one atomic batch, together
            // with its state root (which block 0's header must commit to).
            staged.begin()?;

            // Genesis balances live where transfers and /balance read them.
            for account in &genesis_config.accounts {
                node::accounts::store(&chain_store, &account.address, account.balance, 0)?;
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

            // Bridge relayer set: mints need `threshold` of these relayers.
            if let Some(bridge) = &genesis_config.bridge {
                let config = node::bridge_mint::BridgeConfig {
                    relayers: bridge.relayers.clone(),
                    threshold: bridge.threshold,
                };
                state_trie.lock().await.insert(
                    node::bridge_mint::BRIDGE_CONFIG_KEY,
                    &serde_json::to_vec(&config)?,
                )?;
                info!(
                    "🌉 Bridge: {}-of-{} relayers",
                    bridge.threshold,
                    bridge.relayers.len()
                );
            }

            // Governance state is part of genesis (seeded once, here, so every
            // node starts from the same trie).
            node::governance_store::load_or_init(&state_trie).await?;
            node::runtime_upgrade_store::load_or_init(&state_trie).await?;

            // Store genesis block
            let genesis_block = Block::genesis();
            let block_hash = genesis_block.hash();
            let block_data = serde_json::to_vec(&genesis_block)?;
            chain_store.put_block(&block_hash, &block_data)?;
            chain_store.put_block_height(0, &block_hash)?;
            chain_store.set_latest_height(0)?;

            let trie_root = state_trie.lock().await.root_hash();
            let flat = staged.staged_lthash()?.checksum();
            let root = node::state_root::compute(&flat, &trie_root);
            let mut meta = storage::WriteBatch::new();
            meta.put(storage::ColumnFamily::Meta, storage::STATE_ROOT_KEY, root);
            staged.commit(meta)?;
            info!("🌱 Genesis state root {}", hex::encode(root));
        }
        if chain_store.get_state_root()?.is_none() {
            return Err(
                "Database has no state root (created before Phase 3). Wipe the data directory and restart."
                    .into(),
            );
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
                stake: v.stake,
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
        // Numeric chain ID signed into every tx. Non-numeric config values
        // (e.g. "modular-testnet-1") fall back to 1, which clients default to.
        let chain_id: u64 = config.network.chain_id.parse().unwrap_or(1);
        let mempool_config = mempool::MempoolConfig {
            max_capacity: (config.security.max_tx_per_block * 2),
            max_per_sender: 100,
            min_fee_per_gas: 1_000_000_000,
            chain_id: Some(chain_id),
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
        let mut engine = BftEngine::new(
            public_key.clone(),
            validators.clone(),
            bft_start_height,
            signing_key.clone(),
        );
        if let Some(saved) = load_bft_state(&bft_state_path) {
            info!(
                "🔐 Restored BFT safety state (last signed {:?}, locked: {})",
                saved.last_signed,
                saved.locked.is_some()
            );
            engine.restore_safety_state(saved);
        }
        let bft_engine = Arc::new(Mutex::new(engine));

        // ================================================================
        // 8. Initialize Finality Gadget
        // ================================================================
        let finality_gadget = Arc::new(Mutex::new(FinalityGadget::new(validators.clone())));

        // ================================================================
        // 9. Initialize Block Executor
        // ================================================================
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
            .map_err(|e| format!("Invalid P2P address: {}", e))?;
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
        )
        .with_security(node::rpc::RpcSecurity {
            dev_mode: config.api.dev_mode,
            admin_token: config.api.admin_token.clone(),
            cors_origins: config.api.cors_origins.clone(),
            dev_funding_key: match (&config.api.dev_funding_key, config.api.dev_mode) {
                (Some(hex_seed), true) => Some(
                    hex::decode(hex_seed.trim())
                        .ok()
                        .and_then(|b| <[u8; 32]>::try_from(b.as_slice()).ok())
                        .ok_or("api.dev_funding_key must be a 32-byte hex seed")?,
                ),
                _ => None,
            },
        });
        if config.api.dev_mode {
            warn!(
                "⚠️ RPC dev mode: faucet, validator registration, delegation, bridge \
                 relayer and runtime-upgrade routes write state outside consensus. \
                 Never enable this on a shared network."
            );
            if config.api.admin_token.is_none() {
                warn!("⚠️ No api.admin_token set: admin routes are open to anyone who can reach the RPC port");
            }
        }

        let rpc_port = config.network.rpc_port;
        let rpc_server = Arc::new(rpc_server);
        tokio::spawn({
            let rpc_server = rpc_server.clone();
            async move {
                info!("🔌 RPC server listening on port {}", rpc_port);
                rpc_server.run(rpc_port).await;
            }
        });

        if config.api.enabled {
            let api_port = api_port_from_address(&config.api.address);
            if let Some(api_port) = api_port {
                tokio::spawn({
                    let rpc_server = rpc_server.clone();
                    async move {
                        info!("🔌 API server listening on port {}", api_port);
                        rpc_server.run(api_port).await;
                    }
                });
            } else {
                warn!(
                    "⚠️ Invalid API address {:?}, API server not started",
                    config.api.address
                );
            }
        }

        // Governance is seeded as part of genesis; nothing may write state
        // here, outside a block (it would change the committed state root).

        let prune_config = PruneConfig::from_mode(
            &config.storage.pruning_mode,
            Some(config.storage.blocks_to_keep),
            Some(config.storage.prune_every_n_blocks),
        );
        info!(
            "Storage pruning mode={} keep={} every={} blocks",
            prune_config.mode, prune_config.blocks_to_keep, prune_config.prune_every_n_blocks
        );

        let max_gas_per_block = config.security.max_gas_per_block;
        Ok(Self {
            config,
            chain_store,
            staged,
            receipt_store,
            state_trie,
            chain_id,
            max_gas_per_block,
            fatal_error: None,
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
            sync_peer_cursor: 0,
            sync_peer: None,
            last_sync_attempt_at: Instant::now(),
            last_finalize_at: Instant::now(),
            sync_requested: false,
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

        loop {
            // Process BFT events
            let mut new_events = Vec::new();
            for event in pending_events.drain(..) {
                if let Err(e) = self.handle_bft_event(event, &mut new_events).await {
                    warn!("Error handling BFT event: {}", e);
                }
            }
            pending_events.extend(new_events);

            if let Some(reason) = self.fatal_error.take() {
                tracing::error!("🛑 State diverged from the network — stopping: {}", reason);
                self.shutdown().await?;
                return Err(reason.into());
            }

            tokio::select! {
                _ = consensus_interval.tick() => {
                    let engine = self.bft_engine.lock().await;
                    if let Some(timeout) = engine.check_timeout() {
                        pending_events.push(timeout);
                    }
                }

                _ = metrics_interval.tick() => {
                    if let Err(e) = self.update_live_metrics().await {
                        warn!("Error updating metrics: {}", e);
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
                // Persist the lock + signing watermark before the vote leaves
                // the node; if this fails, do not broadcast.
                self.persist_bft_state().await?;
                debug!("Broadcasting vote for round {}", vote.round);
                self.network_cmd_sender
                    .send(NetworkCommand::BroadcastConsensusMessage(
                        ConsensusMessage::Vote(vote),
                    ))
                    .await?;
            }

            BftEvent::BroadcastProposal(proposal) => {
                self.persist_bft_state().await?;
                info!("Broadcasting proposal for round {}", proposal.round);
                self.network_cmd_sender
                    .send(NetworkCommand::BroadcastConsensusMessage(
                        ConsensusMessage::Proposal(proposal),
                    ))
                    .await?;
            }

            BftEvent::FinalizeBlock(block, _) => {
                info!("🔒 Finalizing block at height {}", block.header.slot);
                self.last_finalize_at = Instant::now();

                if let Err(e) = self.apply_state_transition(&block).await {
                    // +2/3 of the stake finalized this block, so failing to apply
                    // it means our state is not theirs. Stop rather than vote on.
                    self.fatal_error = Some(format!(
                        "cannot apply finalized block {}: {}",
                        block.header.slot, e
                    ));
                    return Err(e);
                }

                // Remove transactions from pool
                self.tx_pool.remove_transactions(&block.extrinsics);

                // Slashing: advance block height tracker
                {
                    let mut tracker = self.slashing_tracker.lock().await;
                    tracker.update_height(block.header.slot);
                }

                self.prune_after(block.header.slot).await;

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

            BftEvent::BehindSync(_target_height) => {
                // Peers are ahead of us — request a contiguous sync on the
                // next sync tick so we catch up to the canonical chain.
                info!(
                    "🚩 Behind peers (height {}) — flagging sync",
                    _target_height
                );
                self.sync_requested = true;
            }

            BftEvent::Equivocation(evidence) => {
                if evidence.is_valid() {
                    let first = evidence.first.block_hash.unwrap_or([0u8; 32]);
                    let second = evidence.second.block_hash.unwrap_or([0u8; 32]);
                    self.slashing_tracker.lock().await.record_double_sign(
                        evidence.first.voter.clone(),
                        evidence.first.height,
                        first,
                        second,
                    );
                }
            }

            BftEvent::NeedBlock(_hash, _height) => {
                // The network finalized a block we don't hold — fetch the
                // contiguous chain from a peer.
                info!(
                    "📥 Need block at height {} from peers — flagging sync",
                    _height
                );
                self.sync_requested = true;
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
                        let engine_height = self.bft_engine.lock().await.height;
                        if proposal.height == engine_height {
                            if let Err(e) = self.validate_proposed_block(
                                &proposal.block,
                                &proposal.proposer,
                                engine_height,
                            ) {
                                // Not voting for it: we prevote nil on timeout.
                                warn!(
                                    "Rejecting proposal at height {} round {}: {}",
                                    proposal.height, proposal.round, e
                                );
                                return Ok(());
                            }
                        }
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

            NetworkEvent::BlockRequestFailed { .. } => {
                // A sync request failed (peer unreachable or timed out) and no
                // response will ever arrive. Clear the in-flight sync flag so
                // the next sync tick retries — otherwise `syncing` stays true
                // forever and the node never catches up.
                if self.syncing {
                    warn!("Block sync request failed — clearing sync so it can retry");
                    self.syncing = false;
                    self.sync_requested = true;
                }
            }

            NetworkEvent::ListeningOn(addr) => {
                info!("🔊 Network listening on {}", addr);
            }
        }

        Ok(())
    }

    /// Handle an incoming block from gossip.
    ///
    /// Blocks are NEVER applied here: a block may only enter the chain store
    /// through a BFT `FinalizeBlock` (the node participated in the quorum) or
    /// through a contiguous sync fetch from a peer. Applying gossip blocks
    /// directly previously let each validator's store diverge (forks) and let
    /// the engine drift from the store, which then required an unsafe engine
    /// reset that could reuse block slots.
    ///
    /// Instead a gossip block only informs sync: if it is further ahead than
    /// our tip by more than one, we are missing intermediate blocks and request
    /// a contiguous sync from the sender.
    async fn handle_incoming_block(
        &mut self,
        block: Block,
        source: PeerId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if block.header.slot == 0 {
            return Ok(());
        }

        let tip = self.chain_store.get_latest_height()?.unwrap_or(0);
        let block_slot = block.header.slot;

        // Current or near-future block: we either already have it or will
        // finalize slot tip+1 through BFT ourselves.
        if block_slot <= tip + 1 {
            return Ok(());
        }

        // We are missing intermediate blocks. Request a contiguous sync from
        // the sender, which presumably holds the canonical chain.
        if !self.syncing {
            info!(
                "Block at slot {} ahead of tip {} — requesting sync",
                block_slot, tip
            );
            self.syncing = true;
            self.last_sync_attempt_at = Instant::now();
            self.sync_peer = Some(source);
            self.network_cmd_sender
                .send(NetworkCommand::RequestBlock {
                    peer: source,
                    start_height: if tip == 0 { 0 } else { tip + 1 },
                    limit: SYNC_BATCH_SIZE,
                })
                .await?;
        }
        Ok(())
    }

    /// Application-level validity of a proposed block for BFT height
    /// `height` (which finalizes slot `height - 1`) on top of our tip.
    fn validate_proposed_block(
        &self,
        block: &Block,
        proposer: &[u8],
        height: u64,
    ) -> Result<(), String> {
        // Rewards and block stats credit `header.proposer` (covered by the
        // block hash), so it must be the validator proposing it this round.
        if block.header.proposer.as_slice() != proposer {
            return Err("block header names a different proposer".into());
        }
        let parent = latest_block(&self.chain_store).map_err(|e| e.to_string())?;
        let state_root = self
            .chain_store
            .get_state_root()
            .map_err(|e| e.to_string())?
            .ok_or("no committed state root")?;
        let parent_validators = self.parent_cert_set().map_err(|e| e.to_string())?;
        node::block_producer::validate_block(
            block,
            &parent,
            &parent_validators,
            height.saturating_sub(1),
            &state_root,
            self.chain_id,
            self.max_gas_per_block,
        )
    }

    /// Verify and apply a block fetched by sync, which we know is a
    /// contiguous extension of our chain.
    async fn apply_block(&mut self, block: &Block) -> Result<(), Box<dyn std::error::Error>> {
        let validators = self.get_validator_list().await?;
        // Finality: a synced block must carry +2/3 precommits from the
        // validator set. A proposer signature alone proves nothing — one
        // validator key could otherwise feed us an arbitrary chain.
        // (BFT height h finalizes block slot h - 1.)
        match &block.commit {
            Some(cert) => {
                cert.verify(&block.hash(), block.header.slot + 1, &validators)
                    .map_err(|e| format!("Invalid commit certificate: {}", e))?;
            }
            None => return Err("Block has no commit certificate".into()),
        }
        let consensus = EnhancedConsensus::new(validators);
        if let Err(e) = consensus.verify_block(block) {
            warn!("Invalid block: {}", e);
            return Err(format!("Invalid block: {}", e).into());
        }
        // The proposer named in the header (and signing it) must be one of
        // the validators.
        if consensus.verify_block_author(block).as_deref() != Some(&block.header.proposer[..]) {
            return Err("Block header is not signed by its proposer".into());
        }

        if let Err(e) = self.apply_state_transition(block).await {
            // The block is certified, so a failure here means we diverged.
            self.fatal_error = Some(format!(
                "cannot apply certified block {}: {}",
                block.header.slot, e
            ));
            return Err(e);
        }

        self.tx_pool.remove_transactions(&block.extrinsics);
        info!("✅ Processed block at height {}", block.header.slot);
        Ok(())
    }

    /// The state transition for one finalized block, applied atomically.
    ///
    /// 1. The header must commit to our state after its parent.
    /// 2. Execute transactions, then rewards, block stats, governance,
    ///    runtime upgrades and the treasury fee — all staged in memory.
    /// 3. Compute the new state root and write everything (block, receipts,
    ///    state, trie, commitment, root) as one batch. On any error the stage
    ///    is discarded and the database is unchanged.
    async fn apply_state_transition(
        &mut self,
        block: &Block,
    ) -> Result<Vec<execution::evm::TransactionReceipt>, Box<dyn std::error::Error>> {
        let expected = self
            .chain_store
            .get_state_root()?
            .ok_or("No committed state root")?;
        if block.header.state_root != expected {
            return Err(format!(
                "state root mismatch at slot {}: block commits to {}, ours is {}",
                block.header.slot,
                hex::encode(block.header.state_root),
                hex::encode(expected)
            )
            .into());
        }

        // Rewards and block stats credit the proposer named in the header,
        // never one derived from local bookkeeping (identical blocks from two
        // proposers once made nodes disagree on who proposed).
        if !block.header.verify_proposer_signature() {
            return Err(
                format!("block {} is not signed by its proposer", block.header.slot).into(),
            );
        }
        let proposer = block.header.proposer.to_vec();

        self.staged.begin()?;
        let result = self.stage_block_effects(block, proposer).await;
        let receipts = match result {
            Ok(receipts) => receipts,
            Err(e) => {
                self.staged.discard();
                self.state_trie.lock().await.reload_root()?;
                return Err(e);
            }
        };

        let trie_root = self.state_trie.lock().await.root_hash();
        let flat = self.staged.staged_lthash()?.checksum();
        let root = node::state_root::compute(&flat, &trie_root);
        let mut meta = storage::WriteBatch::new();
        meta.put(storage::ColumnFamily::Meta, storage::STATE_ROOT_KEY, root);
        if let Err(e) = self.staged.commit(meta) {
            self.staged.discard();
            self.state_trie.lock().await.reload_root()?;
            return Err(e);
        }
        debug!(
            "State root after slot {}: {}",
            block.header.slot,
            hex::encode(root)
        );

        self.metrics.record_block();
        self.metrics.update_finalized_height(block.header.slot);
        self.stats.record_block(block.extrinsics.len());
        Ok(receipts)
    }

    /// Everything a block changes, written into the open stage.
    async fn stage_block_effects(
        &mut self,
        block: &Block,
        proposer: Vec<u8>,
    ) -> Result<Vec<execution::evm::TransactionReceipt>, Box<dyn std::error::Error>> {
        // Vote rewards go to the validators whose precommits finalized the
        // parent, as listed in the block's last commit. The proposer picked
        // that certificate and the header commits to it, so every node pays
        // the same validators (a node's own collected precommits differ, which
        // once forked the state root).
        let parent = latest_block(&self.chain_store)?;
        let voters =
            node::block_producer::verify_last_commit(block, &parent, &self.parent_cert_set()?)?;
        // Record the set certifying *this* block (the set before it applies),
        // for checking the next block's last commit.
        let cert_set = serde_json::to_vec(&self.get_validator_list().await?)?;
        self.staged
            .put(storage::ColumnFamily::State, CERT_SET_KEY, &cert_set)?;

        let receipts = self.block_executor.lock().await.execute_and_commit(block)?;

        let total_fees = rewards::total_fees(&receipts, block.header.base_fee);

        if !proposer.is_empty() {
            if let Err(e) = node::governance_store::record_block_stats(
                &self.state_trie,
                block.header.slot,
                &proposer,
            )
            .await
            {
                warn!("Error recording block stats: {}", e);
            }
        }

        let rewards = {
            let mut rm = self.reward_manager.lock().await;
            rm.process_block(block, proposer, &voters, total_fees)
        };
        for (validator, amount) in rewards {
            if amount > 0 {
                self.apply_reward(&validator, amount)?;
            }
        }

        // Governance actions, from authenticated txs only
        let authenticated = node::block_producer::BlockExecutor::successful_txs(block, &receipts);
        if let Err(e) = node::governance_store::apply_extrinsics(
            &self.state_trie,
            &self.chain_store,
            &authenticated,
            block.header.slot,
        )
        .await
        {
            warn!("Error applying governance actions: {}", e);
        }

        if let Err(e) =
            node::runtime_upgrade_store::apply_at_height(&self.state_trie, block.header.slot).await
        {
            warn!("Error applying runtime upgrades: {}", e);
        }

        // Economic engine: burn 50% of fees; 10% to treasury
        if total_fees > 0 {
            info!(
                "🔥 Fee burn: {} tokens burned from block fees",
                total_fees / 2
            );
            if let Err(e) =
                node::governance_store::collect_treasury_fee(&self.state_trie, total_fees / 10)
                    .await
            {
                warn!("Treasury fee collection failed: {}", e);
            }
        }

        Ok(receipts)
    }

    /// Prune old blocks/receipts below the retention window (not state).
    async fn prune_after(&self, slot: u64) {
        match self.chain_store.maybe_prune(&self.prune_config, slot) {
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

    /// Periodically check whether we are behind peers or consensus has stalled
    /// and, if so, request a contiguous block sync from a connected peer so the
    /// node can catch up to the canonical chain.
    ///
    /// Triggered when:
    /// - the engine signals we are behind (BehindSync / NeedBlock),
    /// - the BFT engine height no longer matches the chain tip + 2 (desync), or
    /// - no block has been finalized for a while (stall).
    ///
    /// The engine is only ever advanced AFTER a sync has brought the chain
    /// store up to the peer's canonical chain (finish_sync). We never reset the
    /// engine forward to a stale local tip, which previously let a node re-propose
    /// an already-committed slot (an equivocation vector that forked the chain).
    async fn maybe_trigger_sync(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.syncing {
            // Watchdog: a request that never completes (peer stopped responding
            // and no OutboundFailure fires) must not wedge the node forever.
            // Clear the flag so the next tick retries against another peer.
            if self.last_sync_attempt_at.elapsed() >= Duration::from_secs(SYNC_STUCK_SECS) {
                warn!(
                    "Sync request in-flight for {}s without completing — clearing sync flag to retry",
                    SYNC_STUCK_SECS
                );
                self.syncing = false;
                // A request that neither completes nor fails means the
                // connection's request handling is wedged: drop the peer's
                // connections so the next attempt starts from fresh ones.
                if let Some(peer) = self.sync_peer.take() {
                    let _ = self
                        .network_cmd_sender
                        .send(NetworkCommand::DisconnectPeer(peer))
                        .await;
                }
            } else {
                return Ok(());
            }
        }

        let tip = self.chain_store.get_latest_height()?.unwrap_or(0);
        let engine_height = {
            let engine = self.bft_engine.lock().await;
            engine.height
        };
        let desync = tip > 0 && engine_height != tip + 2;
        let sync_requested = std::mem::take(&mut self.sync_requested);
        let urgent = sync_requested || desync;

        let stalled = self.last_finalize_at.elapsed() >= Duration::from_secs(SYNC_GRACE_SECS);

        // Non-urgent (stall) triggers are throttled; urgent ones retry quickly.
        if !urgent && !stalled {
            return Ok(());
        }
        if !urgent && self.last_sync_attempt_at.elapsed() < Duration::from_secs(SYNC_GRACE_SECS) {
            return Ok(());
        }
        if urgent && self.last_sync_attempt_at.elapsed() < Duration::from_secs(2) {
            return Ok(());
        }

        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.network_cmd_sender
            .send(NetworkCommand::ListConnectedPeers(reply_tx))
            .await?;
        // Bound the wait so a stalled reply can never wedge the main loop
        // (which would stop draining network events and deadlock the network).
        let peers = tokio::time::timeout(Duration::from_secs(5), reply_rx)
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or_default();
        // A different peer on each attempt: a peer whose request stream hangs
        // (no response and no failure event) once wedged a validator that
        // kept asking it forever.
        let peer = if peers.is_empty() {
            None
        } else {
            self.sync_peer_cursor = self.sync_peer_cursor.wrapping_add(1);
            peers.get(self.sync_peer_cursor % peers.len()).copied()
        };

        match peer {
            Some(peer) => {
                info!(
                    "⚠️ Behind peers at height {} (engine {}) — requesting sync from {}",
                    tip, engine_height, peer
                );
                self.syncing = true;
                self.last_sync_attempt_at = Instant::now();
                self.sync_peer = Some(peer);
                self.network_cmd_sender
                    .send(NetworkCommand::RequestBlock {
                        peer,
                        start_height: if tip == 0 { 0 } else { tip + 1 },
                        limit: SYNC_BATCH_SIZE,
                    })
                    .await?;
            }
            None => {
                if desync {
                    warn!(
                        "Consensus behind (engine {}, tip {}) but no peers connected — will retry",
                        engine_height, tip
                    );
                } else {
                    debug!("Consensus stalled but no peers connected — will retry");
                }
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
            // Keep the response under the sync codec's message limit.
            let mut bytes = 0usize;
            for height in start_height..=end {
                if let Some(data) = self.chain_store.get_block_by_height(height)? {
                    bytes = bytes.saturating_add(data.len());
                    if bytes > network::protocol::MAX_MESSAGE_BYTES / 2 && !blocks.is_empty() {
                        break;
                    }
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
        // A response can arrive after the watchdog stopped waiting for it.
        // Its blocks are verified like any other, so use them if they extend
        // our chain instead of dropping them.
        if !self.syncing {
            let tip = self.chain_store.get_latest_height()?.unwrap_or(0);
            if !blocks.iter().any(|b| b.header.slot > tip) {
                return Ok(());
            }
            self.syncing = true;
        }

        let total_received = blocks.len();
        let mut applied = 0usize;
        let mut advanced = 0usize;

        for block in blocks {
            let tip = self.chain_store.get_latest_height()?.unwrap_or(0);
            let slot = block.header.slot;

            // Replacement of our current tip — only for slot 0 (the local
            // genesis placeholder vs. the network's block 0). A finalized block
            // is never replaced.
            if slot == tip && tip == 0 {
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
            if slot <= tip {
                continue; // already have it
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
            // Leave last_sync_attempt_at untouched so the next sync tick retries
            // promptly (possibly against a different peer).
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
        self.last_sync_attempt_at = Instant::now();
        self.sync_peer = Some(peer);
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
    /// The validator set that certified our tip block (recorded when it was
    /// applied), against which the next block's last commit is checked.
    fn parent_cert_set(&self) -> Result<Vec<ValidatorInfo>, Box<dyn std::error::Error>> {
        match self
            .staged
            .get(storage::ColumnFamily::State, CERT_SET_KEY)?
        {
            Some(bytes) => Ok(serde_json::from_slice(&bytes)?),
            None => Ok(Vec::new()), // tip is genesis: no last commit to check
        }
    }

    async fn get_validator_list(&self) -> Result<Vec<ValidatorInfo>, Box<dyn std::error::Error>> {
        node::governance_store::load_consensus_validators(&self.state_trie)
            .await
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })
    }

    /// Reload BFT + finality validator sets from the state trie when changed.
    async fn sync_validator_set(&self) -> Result<(), Box<dyn std::error::Error>> {
        let validators =
            node::governance_store::load_consensus_validators(&self.state_trie).await?;
        if validators.is_empty() {
            return Ok(());
        }
        let mut engine = self.bft_engine.lock().await;
        let mut incoming: Vec<(Vec<u8>, u128)> = validators
            .iter()
            .filter(|v| !v.slashed)
            .map(|v| (v.public_key.clone(), v.stake))
            .collect();
        incoming.sort();
        // Compare keys and stakes, not just the count: a swapped validator or a
        // stake change alters what counts as a quorum.
        if engine.validator_set() != incoming {
            let count = incoming.len();
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
            .fold(0u128, |acc, s| acc.saturating_add(s));
        self.metrics
            .update_validator_set(validators.len(), stake_total as u64);

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

    /// Credit a validator's reward account (address derived from its key).
    fn apply_reward(
        &mut self,
        validator: &[u8],
        amount: u128,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let address = node::accounts::address_of_public_key(validator);
        node::accounts::credit(&self.chain_store, &address, amount)?;
        Ok(())
    }

    /// Persist BFT state to disk
    async fn persist_bft_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        let safety = self.bft_engine.lock().await.safety_state();
        let state = BftPersistedState {
            safety,
            last_saved: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        // Write-then-rename so a crash mid-write never leaves a torn file.
        let content = serde_json::to_vec(&state)?;
        let tmp = self.bft_state_path.with_extension("json.tmp");
        {
            use std::io::Write;
            let mut file = fs::File::create(&tmp)?;
            file.write_all(&content)?;
            file.sync_all()?;
        }
        fs::rename(&tmp, &self.bft_state_path)?;
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

/// The latest committed block (the local genesis placeholder on a new chain).
/// Node-local (outside the state root) record of the validator set that
/// certified the tip block.
const CERT_SET_KEY: &[u8] = b"local:consensus:cert_set";

fn latest_block(chain_store: &ChainStore) -> Result<Block, Box<dyn std::error::Error>> {
    match chain_store.get_latest_height()? {
        Some(height) => {
            let data = chain_store
                .get_block_by_height(height)?
                .ok_or_else(|| format!("Block not found at height {}", height))?;
            Ok(serde_json::from_slice(&data)?)
        }
        None => Ok(Block::genesis()),
    }
}

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
        .parse::<u128>()?;
    let cooldown = env::var("COOLDOWN_SECONDS")
        .unwrap_or_else(|_| "86400".to_string())
        .parse::<u64>()?;

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
    // RUST_LOG selects levels per target (e.g. `info,network=debug`).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
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
