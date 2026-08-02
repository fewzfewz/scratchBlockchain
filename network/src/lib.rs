//! # Network Module
//!
//! This module implements the peer-to-peer networking layer for the blockchain node.
//! It uses libp2p for all networking functionality including:
//!
//! - **Gossipsub**: Floodsub-style pub/sub for transactions, blocks, and consensus messages
//! - **Kademlia DHT**: Peer discovery and provider records
//! - **Request-Response**: Direct block requests for synchronization
//! - **Peer Reputation**: Track peer behavior and ban malicious peers
//! - **Rate Limiting**: Prevent DoS attacks by limiting message frequency
//! - **Peer Persistence**: Save and restore known peers across restarts
//!
//! ## Architecture
//! ```
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      NetworkService                          │
//! ├─────────────┬─────────────┬─────────────┬───────────────────┤
//! │ Gossipsub   │  Kademlia   │ Req-Resp    │ Connection Limits │
//! │ (Broadcast) │ (Discovery) │ (Sync)      │ (Security)        │
//! └─────────────┴─────────────┴─────────────┴───────────────────┘
//!        │              │              │              │
//!        └──────────────┴──────────────┴──────────────┘
//!                           │
//!                    ┌──────▼──────┐
//!                    │   Swarm     │
//!                    │  (libp2p)   │
//!                    └─────────────┘
//! ```

mod behaviour;
pub mod peer_store;
pub mod protocol;
pub mod rate_limiter;
pub mod reputation;
pub mod sync;
mod transport;

use behaviour::NodeBehaviour;
use common::consensus_types::ConsensusMessage;
use common::types::{Block, Transaction};
use futures::StreamExt;
use libp2p::{
    connection_limits,
    gossipsub::{self, IdentTopic, MessageAuthenticity, ValidationMode},
    identity,
    kad::{store::MemoryStore, Behaviour as Kademlia, Config as KademliaConfig},
    request_response::{self, ProtocolSupport, ResponseChannel},
    swarm::{Config, SwarmEvent},
    Multiaddr, PeerId, Swarm,
};
use protocol::{BlockExchangeProtocol, BlockRequest, BlockResponse};
use rate_limiter::{MessageType, RateLimitConfig, RateLimiter};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::iter;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex as TokioMutex;
use tracing::{debug, info, warn};

// ============================================================================
// Gossipsub Topics - Each message type gets its own topic for isolation
// ============================================================================

/// Topic for broadcasting new transactions
/// Format: /blockchain/tx/{version}
const TRANSACTION_TOPIC: &str = "/blockchain/tx/1.0.0";

/// Topic for broadcasting new blocks
/// Format: /blockchain/blocks/{version}
const BLOCK_TOPIC: &str = "/blockchain/blocks/1.0.0";

/// Topic for broadcasting consensus messages (votes, proposals)
/// Format: /blockchain/consensus/{version}
const CONSENSUS_TOPIC: &str = "/blockchain/consensus/1.0.0";

// ============================================================================
// Message Types (Serializable for wire transmission)
// ============================================================================

/// Wrapper for transaction messages sent over gossip network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMessage {
    /// The actual transaction
    pub transaction: Transaction,
}

/// Wrapper for block messages sent over gossip network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMessage {
    /// The actual block
    pub block: Block,
}

// ============================================================================
// Network Events (sent from network layer to node)
// ============================================================================

/// Events that the network layer sends to the main node event loop
#[derive(Debug)]
pub enum NetworkEvent {
    /// A new transaction was received from a peer
    TransactionReceived(Transaction),
    
    /// A new block was received from a peer
    BlockReceived {
        block: Block,
        source: PeerId,
    },
    
    /// A consensus message (vote/proposal) was received
    ConsensusMessageReceived(ConsensusMessage),
    
    /// Network is now listening on this address
    ListeningOn(Multiaddr),
    
    /// A block request was received from a peer
    BlockRequestReceived {
        peer: PeerId,
        request_id: request_response::RequestId,
        start_height: u64,
        limit: u32,
        channel: ResponseChannel<BlockResponse>,
    },
    
    /// A block response was received from a peer
    BlockResponseReceived {
        peer: PeerId,
        request_id: request_response::RequestId,
        blocks: Vec<Block>,
    },
}

// ============================================================================
// Network Commands (sent from node to network layer)
// ============================================================================

/// Commands that the node can send to control the network layer
#[derive(Debug)]
pub enum NetworkCommand {
    /// Start listening on a specific multiaddress
    StartListening(Multiaddr),
    
    /// Dial (connect to) a specific peer address
    Dial(Multiaddr),
    
    /// Broadcast a transaction to all peers
    BroadcastTransaction(Transaction),
    
    /// Broadcast a block to all peers
    BroadcastBlock(Block),
    
    /// Broadcast a consensus message to all peers
    BroadcastConsensusMessage(ConsensusMessage),
    
    /// Request blocks from a specific peer (for synchronization)
    RequestBlock {
        peer: PeerId,
        start_height: u64,
        limit: u32,
    },
    
    /// Send a block response back to a requesting peer
    SendBlockResponse {
        channel: ResponseChannel<BlockResponse>,
        blocks: Vec<Block>,
    },
    
    /// Return the list of currently connected peers (used to pick a sync source)
    ListConnectedPeers(tokio::sync::oneshot::Sender<Vec<PeerId>>),
    
    /// Save current peer list to disk
    SavePeers,
    
    /// Query current network statistics (peers, connections, bytes)
    GetStats(tokio::sync::oneshot::Sender<NetworkStats>),
}

// ============================================================================
// Network Stats (queried by the node for metrics)
// ============================================================================

/// Snapshot of current network statistics
#[derive(Debug, Clone, Default)]
pub struct NetworkStats {
    /// Number of connected peers
    pub peer_count: usize,
    /// Total established connections (incoming + outgoing)
    pub connection_count: usize,
    /// Connections currently being established
    pub pending_connections: usize,
    /// Application-layer bytes received via gossip
    pub bytes_rx: u64,
    /// Application-layer bytes sent via gossip
    pub bytes_tx: u64,
}

// ============================================================================
// NetworkService - Main Network Component
// ============================================================================

/// Main network service that manages all P2P communication
pub struct NetworkService {
    /// libp2p swarm that manages all protocols
    swarm: Swarm<NodeBehaviour>,

    /// Local peer ID (stored separately because Swarm::local_peer_id is private)
    local_peer_id: PeerId,
    
    /// Channel for receiving commands from the node
    command_receiver: mpsc::Receiver<NetworkCommand>,
    
    /// Channel for sending events to the node
    event_sender: mpsc::Sender<NetworkEvent>,
    
    /// Track pending request IDs for timeout handling
    pending_requests: HashSet<request_response::RequestId>,
    
    /// Rate limiter to prevent DoS attacks
    rate_limiter: Arc<TokioMutex<RateLimiter>>,
    
    /// Peer reputation system (track good/bad behavior)
    reputation: reputation::PeerReputation,
    
    /// Bootstrap nodes to connect to on startup
    bootstrap_addresses: Vec<Multiaddr>,
    
    /// Persistent peer store (saves known peers to disk)
    peer_store: peer_store::PeerStore,
    
    /// Path for reputation file (for persistence)
    reputation_file: String,
    
    /// Reconnection interval for maintaining peer connections
    reconnect_interval: tokio::time::Interval,
    
    /// Flag indicating if shutdown has been initiated
    shutting_down: bool,
    
    /// Cumulative gossip bytes received (application layer)
    bytes_rx: u64,
    
    /// Cumulative gossip bytes sent (application layer)
    bytes_tx: u64,
}

/// Return type for network service initialization
pub type NetworkServiceInit = (
    NetworkService,
    mpsc::Sender<NetworkCommand>,
    mpsc::Receiver<NetworkEvent>,
);

impl NetworkService {
    /// Create a new network service
    ///
    /// # Arguments
    /// * `bootstrap_nodes` - List of bootstrap node addresses (multiaddr strings)
    /// * `peer_store_path` - Path to save/load peer list (JSON file)
    ///
    /// # Returns
    /// * `Ok((service, command_sender, event_receiver))` - Network service and channels
    /// * `Err` - If initialization fails
    pub fn new(
        bootstrap_nodes: Vec<String>,
        peer_store_path: &str,
    ) -> Result<NetworkServiceInit, Box<dyn Error>> {
        // Generate or load local key pair
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        info!("🆔 Local peer id: {}", local_peer_id);

        // Build transport layer with noise encryption and mplex multiplexing
        let transport = transport::build_transport(&local_key)?;

        // ====================================================================
        // Gossipsub Configuration - For broadcasting messages
        // ====================================================================
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))      // Check peers every second
            .validation_mode(ValidationMode::Strict)         // Validate all messages
            .max_transmit_size(64 * 1024)                    // 64KB max message size
            .build()
            .map_err(|e| format!("Failed to build gossipsub config: {}", e))?;

        let mut gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )?;

        // Subscribe to all required topics
        let tx_topic = IdentTopic::new(TRANSACTION_TOPIC);
        let block_topic = IdentTopic::new(BLOCK_TOPIC);
        let consensus_topic = IdentTopic::new(CONSENSUS_TOPIC);

        gossipsub.subscribe(&tx_topic)?;
        gossipsub.subscribe(&block_topic)?;
        gossipsub.subscribe(&consensus_topic)?;

        info!("📡 Subscribed to topics: tx, block, consensus");

        // ====================================================================
        // Kademlia Configuration - For peer discovery
        // ====================================================================
        let kademlia_config = KademliaConfig::default();
        let kademlia_store = MemoryStore::new(local_peer_id);
        let kademlia = Kademlia::with_config(local_peer_id, kademlia_store, kademlia_config);

        // ====================================================================
        // Request-Response Configuration - For block synchronization
        // ====================================================================
        let mut request_response_config = request_response::Config::default();
        request_response_config.set_request_timeout(Duration::from_secs(30));  // 30 second timeout for block requests

        let request_response = request_response::Behaviour::new(
            iter::once((BlockExchangeProtocol(), ProtocolSupport::Full)),
            request_response_config,
        );

        // ====================================================================
        // Connection Limits - For DoS protection
        // ====================================================================
        let connection_limits_config = connection_limits::ConnectionLimits::default()
            .with_max_pending_incoming(Some(10))           // Max 10 pending incoming connections
            .with_max_pending_outgoing(Some(10))           // Max 10 pending outgoing connections
            .with_max_established_incoming(Some(50))       // Max 50 established incoming
            .with_max_established_outgoing(Some(50))       // Max 50 established outgoing
            .with_max_established_per_peer(Some(5));       // Max 5 connections per peer

        // ====================================================================
        // Build the complete behaviour
        // ====================================================================
        let behaviour = NodeBehaviour {
            gossipsub,
            kademlia,
            request_response,
            connection_limits: connection_limits::Behaviour::new(connection_limits_config),
        };

        // Create the swarm
        let swarm = Swarm::new(
            transport,
            behaviour,
            local_peer_id,
            Config::with_tokio_executor(),
        );

        // Create communication channels
        let (command_sender, command_receiver) = mpsc::channel(64);
        let (event_sender, event_receiver) = mpsc::channel(256);

        // Initialize rate limiter
        let rate_limiter = Arc::new(TokioMutex::new(RateLimiter::new(RateLimitConfig::default())));

        // Load peer store
        let peer_store = peer_store::PeerStore::new(peer_store_path)?;
        
        // Setup reputation persistence path
        let reputation_file = format!("{}/reputation.json", 
            peer_store_path.trim_end_matches("/peers.json"));

        // Parse bootstrap addresses
        let bootstrap_addresses: Vec<Multiaddr> = bootstrap_nodes
            .into_iter()
            .filter_map(|addr| Self::parse_bootstrap_addr(&addr))
            .collect();

        // Create the service
        let mut service = Self {
            swarm,
            local_peer_id,
            command_receiver,
            event_sender,
            pending_requests: HashSet::new(),
            rate_limiter,
            reputation: reputation::PeerReputation::new(),
            bootstrap_addresses,
            peer_store,
            reputation_file,
            reconnect_interval: tokio::time::interval(Duration::from_secs(15)),
            shutting_down: false,
            bytes_rx: 0,
            bytes_tx: 0,
        };

        // Attempt to connect to known peers immediately
        service.reconnect_known_peers();

        Ok((service, command_sender, event_receiver))
    }

    /// Run the network service main loop
    /// 
    /// This method runs indefinitely, processing swarm events and commands.
    /// It should be spawned in a separate tokio task.
    pub async fn run(mut self) {
        info!("🌐 Network service started");
        
        loop {
            // Check if we should shut down
            if self.shutting_down {
                info!("Network service shutting down");
                break;
            }
            
            tokio::select! {
                // Process swarm events (incoming messages, connections, etc.)
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await;
                },
                
                // Process commands from the node
                command = self.command_receiver.recv() => {
                    match command {
                        Some(cmd) => self.handle_network_command(cmd).await,
                        None => {
                            info!("Command channel closed, shutting down");
                            break;
                        }
                    }
                },
                
                // Periodic reconnection to known peers
                _ = self.reconnect_interval.tick() => {
                    if !self.shutting_down {
                        self.reconnect_known_peers();
                    }
                },
            }
        }
        
        // Save peers and reputation on shutdown
        self.save_peers();
        let _ = self.save_reputation();
        info!("👋 Network service stopped");
    }
    
    /// Gracefully shut down the network service
    pub async fn shutdown(&mut self) -> Result<(), Box<dyn Error>> {
        info!("Shutting down network service...");
        self.shutting_down = true;
        self.save_peers();
        self.save_reputation()?;
        Ok(())
    }

    // ========================================================================
    // Event Handlers
    // ========================================================================

    /// Handle incoming swarm events from libp2p
    async fn handle_swarm_event<E: std::fmt::Debug>(
        &mut self,
        event: SwarmEvent<behaviour::NodeBehaviourEvent, E>,
    ) {
        match event {
            // New listening address - we can now accept connections
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("🔊 Listening on {}", address);
                let _ = self.event_sender
                    .send(NetworkEvent::ListeningOn(address))
                    .await;
            }
            
            // ================================================================
            // Gossipsub Messages (broadcast messages)
            // ================================================================
            SwarmEvent::Behaviour(behaviour::NodeBehaviourEvent::Gossipsub(
                gossipsub::Event::Message {
                    propagation_source: source,
                    message_id,
                    message,
                },
            )) => {
                debug!("📨 Received message from {}: {}", source, message_id);
                self.handle_gossipsub_message(message, source).await;
            }
            
            // ================================================================
            // Kademlia Events (peer discovery)
            // ================================================================
            SwarmEvent::Behaviour(behaviour::NodeBehaviourEvent::Kademlia(event)) => {
                debug!("🔍 Kademlia event: {:?}", event);
            }
            
            // ================================================================
            // Request-Response Events (block synchronization)
            // ================================================================
            SwarmEvent::Behaviour(behaviour::NodeBehaviourEvent::RequestResponse(
                request_response::Event::Message { peer, message },
            )) => match message {
                // Received a block request from a peer
                request_response::Message::Request {
                    request_id,
                    request,
                    channel,
                    ..
                } => {
                    debug!("📥 Block request from {}: height {} limit {}", 
                           peer, request.start_height, request.limit);
                    
                    if let Err(e) = self.event_sender
                        .send(NetworkEvent::BlockRequestReceived {
                            peer,
                            request_id,
                            start_height: request.start_height,
                            limit: request.limit,
                            channel,
                        })
                        .await
                    {
                        warn!("Failed to forward block request: {}", e);
                    }
                }
                // Received a block response from a peer
                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    debug!("📦 Block response from {}: {} blocks", peer, response.blocks.len());
                    self.pending_requests.remove(&request_id);
                    
                    if let Err(e) = self.event_sender
                        .send(NetworkEvent::BlockResponseReceived {
                            peer,
                            request_id,
                            blocks: response.blocks,
                        })
                        .await
                    {
                        warn!("Failed to forward block response: {}", e);
                    }
                }
            },
            
            // Request-response failures
            SwarmEvent::Behaviour(behaviour::NodeBehaviourEvent::RequestResponse(
                request_response::Event::OutboundFailure {
                    request_id, error, ..
                },
            )) => {
                warn!("❌ Request {} failed: {:?}", request_id, error);
                self.pending_requests.remove(&request_id);
            }
            
            // ================================================================
            // Connection Events (tracking peer connections)
            // ================================================================
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                info!("🔗 Connection established with peer: {}", peer_id);
                
                // Add to peer store for future reconnection
                let remote_addr = endpoint.get_remote_address();
                self.peer_store.add_peer(remote_addr);
                if let Err(e) = self.peer_store.save() {
                    warn!("Failed to save peer store: {}", e);
                }
                
                // Add address to Kademlia for discovery
                self.swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id, remote_addr.clone());
            }
            
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                info!("🔌 Connection closed with peer {}: {:?}", peer_id, cause);
            }
            
            // Connection errors
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                warn!("❌ Outgoing connection error to {:?}: {}", peer_id, error);
            }
            
            SwarmEvent::IncomingConnectionError {
                local_addr,
                send_back_addr,
                error,
                ..
            } => {
                debug!("⚠️ Incoming connection error from {} to {}: {}", 
                       send_back_addr, local_addr, error);
            }
            
            _ => {}
        }
    }
    
    /// Handle incoming gossip messages (transactions, blocks, consensus)
    async fn handle_gossipsub_message(&mut self, message: gossipsub::Message, source: PeerId) {
        let topic = message.topic.as_str();
        
        // Check if peer is banned
        if self.reputation.is_banned(&source) {
            warn!("🚫 Ignoring message from banned peer: {}", source);
            return;
        }
        
        self.bytes_rx += message.data.len() as u64;

        match topic {
            TRANSACTION_TOPIC => {
                // Rate limit check
                {
                    let mut limiter = self.rate_limiter.lock().await;
                    if let Err(e) = limiter.check_and_consume(&source, MessageType::Transaction) {
                        warn!("⏱️ Rate limit exceeded for transaction from {}: {}", source, e);
                        self.reputation.report_bad_behavior(source, 5);
                        return;
                    }
                }

                match serde_json::from_slice::<TransactionMessage>(&message.data) {
                    Ok(tx_msg) => {
                        debug!("💰 Received transaction from {}", source);
                        self.reputation.report_good_behavior(source);
                        
                        if let Err(e) = self.event_sender
                            .send(NetworkEvent::TransactionReceived(tx_msg.transaction))
                            .await
                        {
                            warn!("Failed to forward transaction: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to deserialize transaction: {}", e);
                        self.reputation.report_bad_behavior(source, 10);
                    }
                }
            }
            
            BLOCK_TOPIC => {
                match serde_json::from_slice::<BlockMessage>(&message.data) {
                    Ok(block_msg) => {
                        info!("📦 Received block from {} at height {}", 
                              source, block_msg.block.header.slot);
                        
                        if let Err(e) = self.event_sender
                            .send(NetworkEvent::BlockReceived {
                                block: block_msg.block,
                                source,
                            })
                            .await
                        {
                            warn!("Failed to forward block: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to deserialize block: {}", e);
                        self.reputation.report_bad_behavior(source, 10);
                    }
                }
            }
            
            CONSENSUS_TOPIC => {
                // Rate limit check
                {
                    let mut limiter = self.rate_limiter.lock().await;
                    if let Err(e) = limiter.check_and_consume(&source, MessageType::ConsensusMessage) {
                        warn!("⏱️ Rate limit exceeded for consensus message from {}: {}", source, e);
                        return;
                    }
                }

                match serde_json::from_slice::<ConsensusMessage>(&message.data) {
                    Ok(msg) => {
                        debug!("🗳️ Received consensus message from {}", source);
                        
                        if let Err(e) = self.event_sender
                            .send(NetworkEvent::ConsensusMessageReceived(msg))
                            .await
                        {
                            warn!("Failed to forward consensus message: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to deserialize consensus message: {}", e);
                        self.reputation.report_bad_behavior(source, 10);
                    }
                }
            }
            
            _ => {
                debug!("Unknown topic: {}", topic);
            }
        }
    }
    
    /// Handle network commands from the node
    async fn handle_network_command(&mut self, command: NetworkCommand) {
        match command {
            NetworkCommand::StartListening(addr) => {
                if let Err(e) = self.swarm.listen_on(addr) {
                    warn!("Failed to listen on address: {}", e);
                }
            }
            
            NetworkCommand::Dial(addr) => {
                if let Err(e) = self.swarm.dial(addr) {
                    warn!("Failed to dial address: {}", e);
                }
            }
            
            NetworkCommand::BroadcastTransaction(tx) => {
                self.broadcast_transaction(tx);
            }
            
            NetworkCommand::BroadcastBlock(block) => {
                self.broadcast_block(block);
            }
            
            NetworkCommand::BroadcastConsensusMessage(msg) => {
                self.broadcast_consensus_message(msg);
            }
            
            NetworkCommand::RequestBlock { peer, start_height, limit } => {
                let request_id = self.swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&peer, BlockRequest { start_height, limit });
                self.pending_requests.insert(request_id);
                debug!("📤 Sent block request to {} (height {}+{})", peer, start_height, limit);
            }
            
            NetworkCommand::SendBlockResponse { channel, blocks } => {
                if let Err(e) = self.swarm
                    .behaviour_mut()
                    .request_response
                    .send_response(channel, BlockResponse { blocks })
                {
                    warn!("Failed to send block response: {:?}", e);
                }
            }
            
            NetworkCommand::ListConnectedPeers(reply) => {
                let peers: Vec<PeerId> = self.swarm.connected_peers().cloned().collect();
                let _ = reply.send(peers);
            }
            
            NetworkCommand::SavePeers => {
                self.save_peers();
                let _ = self.save_reputation();
            }
            
            NetworkCommand::GetStats(reply) => {
                let stats = self.stats();
                let _ = reply.send(stats);
            }
        }
    }
    
    // ========================================================================
    // Broadcasting Methods
    // ========================================================================
    
    /// Broadcast a transaction to all peers via gossipsub
    fn broadcast_transaction(&mut self, tx: Transaction) {
        let msg = TransactionMessage { transaction: tx };
        let topic = IdentTopic::new(TRANSACTION_TOPIC);
        
        match serde_json::to_vec(&msg) {
            Ok(data) => {
                self.bytes_tx += data.len() as u64;
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    warn!("Failed to broadcast transaction: {}", e);
                } else {
                    debug!("📡 Broadcasted transaction to network");
                }
            }
            Err(e) => {
                warn!("Failed to serialize transaction: {}", e);
            }
        }
    }
    
    /// Broadcast a block to all peers via gossipsub
    fn broadcast_block(&mut self, block: Block) {
        let msg = BlockMessage { block };
        let topic = IdentTopic::new(BLOCK_TOPIC);
        
        match serde_json::to_vec(&msg) {
            Ok(data) => {
                self.bytes_tx += data.len() as u64;
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    warn!("Failed to broadcast block: {}", e);
                } else {
                    info!("📡 Broadcasted block to network");
                }
            }
            Err(e) => {
                warn!("Failed to serialize block: {}", e);
            }
        }
    }
    
    /// Broadcast a consensus message to all peers via gossipsub
    fn broadcast_consensus_message(&mut self, msg: ConsensusMessage) {
        let topic = IdentTopic::new(CONSENSUS_TOPIC);
        
        match serde_json::to_vec(&msg) {
            Ok(data) => {
                self.bytes_tx += data.len() as u64;
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    warn!("Failed to broadcast consensus message: {}", e);
                } else {
                    debug!("📡 Broadcasted consensus message to network");
                }
            }
            Err(e) => {
                warn!("Failed to serialize consensus message: {}", e);
            }
        }
    }
    
    // ========================================================================
    // Peer Management
    // ========================================================================
    
    /// Save current peers to disk
    pub fn save_peers(&mut self) {
        if let Err(e) = self.peer_store.save() {
            warn!("Failed to save peers: {}", e);
        } else {
            debug!("💾 Saved {} peers to disk", self.peer_store.len());
        }
    }
    
    /// Save reputation data to disk
    fn save_reputation(&self) -> Result<(), Box<dyn Error>> {
        let reputation_data = self.reputation.export()?;
        std::fs::write(&self.reputation_file, serde_json::to_string_pretty(&reputation_data)?)?;
        debug!("💾 Saved reputation data to {}", self.reputation_file);
        Ok(())
    }
    
    /// Load reputation data from disk
    #[allow(dead_code)]
    fn load_reputation(&mut self) -> Result<(), Box<dyn Error>> {
        if let Ok(data) = std::fs::read_to_string(&self.reputation_file) {
            let reputation_data: serde_json::Value = serde_json::from_str(&data)?;
            self.reputation.import(reputation_data)?;
            debug!("📂 Loaded reputation data from {}", self.reputation_file);
        }
        Ok(())
    }
    
    /// Attempt to reconnect to known peers (bootstrap + saved)
    fn reconnect_known_peers(&mut self) {
        let mut seen = HashSet::new();
        
        // Clone to avoid borrow conflict with try_dial_known_addr
        let bootstrap_addresses = self.bootstrap_addresses.clone();
        for addr in &bootstrap_addresses {
            if seen.insert(addr.to_string()) {
                self.try_dial_known_addr(addr.clone());
            }
        }
        
        // Connect to saved peers
        let peer_addresses = self.peer_store.get_peers();
        for addr in &peer_addresses {
            let addr_str = addr.to_string();
            if seen.insert(addr_str) {
                self.try_dial_known_addr(addr.clone());
            }
        }
    }
    
    /// Attempt to dial a known address (non-blocking)
    fn try_dial_known_addr(&mut self, addr: Multiaddr) {
        debug!("🔄 Attempting to connect to known peer: {}", addr);
        if let Err(e) = self.swarm.dial(addr.clone()) {
            debug!("Failed to dial {}: {}", addr, e);
        }
    }
    
    // ========================================================================
    // Utility Methods
    // ========================================================================
    
    /// Parse a bootstrap address string into a Multiaddr
    /// Supports formats: "/ip4/1.2.3.4/tcp/9000" or "1.2.3.4:9000"
    fn parse_bootstrap_addr(addr: &str) -> Option<Multiaddr> {
        // Try parsing as Multiaddr directly
        if let Ok(parsed) = addr.parse::<Multiaddr>() {
            return Some(parsed);
        }

        // Try parsing as host:port format
        let (host, port) = addr.rsplit_once(':')?;
        
        let multiaddr = if host.parse::<Ipv4Addr>().is_ok() {
            format!("/ip4/{}/tcp/{}", host, port)
        } else if host.parse::<std::net::Ipv6Addr>().is_ok() {
            format!("/ip6/{}/tcp/{}", host, port)
        } else {
            format!("/dns4/{}/tcp/{}", host, port)
        };

        match multiaddr.parse::<Multiaddr>() {
            Ok(parsed) => Some(parsed),
            Err(error) => {
                warn!("Invalid bootstrap node address '{}': {}", addr, error);
                None
            }
        }
    }
    
    /// Get the local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }
    
    /// Get number of connected peers
    pub fn connected_peers(&self) -> usize {
        self.swarm.connected_peers().count()
    }
    
    /// Get a snapshot of current network statistics
    pub fn stats(&self) -> NetworkStats {
        let info = self.swarm.network_info();
        let counters = info.connection_counters();
        NetworkStats {
            peer_count: info.num_peers(),
            connection_count: counters.num_established() as usize,
            pending_connections: counters.num_pending() as usize,
            bytes_rx: self.bytes_rx,
            bytes_tx: self.bytes_tx,
        }
    }
}

// ============================================================================
// Module Initialization
// ============================================================================

pub fn init() {
    println!("🌐 Network module initialized");
    println!("   - Gossipsub (pub/sub)");
    println!("   - Kademlia (DHT discovery)");
    println!("   - Request-Response (block sync)");
    println!("   - Rate limiting & reputation");
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_bootstrap_addr() {
        // IPv4 with port
        let addr = NetworkService::parse_bootstrap_addr("192.168.1.1:9000");
        assert!(addr.is_some());
        assert_eq!(addr.unwrap().to_string(), "/ip4/192.168.1.1/tcp/9000");
        
        // Full multiaddr format
        let addr = NetworkService::parse_bootstrap_addr("/ip4/10.0.0.1/tcp/9000");
        assert!(addr.is_some());
        
        // DNS format
        let addr = NetworkService::parse_bootstrap_addr("bootstrap.example.com:9000");
        assert!(addr.is_some());
        assert!(addr.unwrap().to_string().contains("dns4"));
        
        // Invalid address
        let addr = NetworkService::parse_bootstrap_addr("not-an-address");
        assert!(addr.is_none());
    }
}