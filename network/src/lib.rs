mod behaviour;
pub mod protocol;
mod transport;
pub mod peer_store;
pub mod rate_limiter;
pub mod reputation;

use behaviour::NodeBehaviour;
use common::types::{Block, Transaction};
use common::consensus_types::ConsensusMessage;
use rate_limiter::{RateLimiter, RateLimitConfig, MessageType};
use futures::StreamExt;
use libp2p::{
    gossipsub::{self, IdentTopic},
    identity,
    kad::{store::MemoryStore, Behaviour as Kademlia},
    request_response::{self, ProtocolSupport, ResponseChannel},
    connection_limits,
    swarm::{Config, SwarmEvent},
    Multiaddr, PeerId, Swarm,
};
use protocol::{BlockExchangeProtocol, BlockRequest, BlockResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::iter;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

// Gossipsub topics
const TRANSACTION_TOPIC: &str = "/blockchain/tx/1.0.0";
const BLOCK_TOPIC: &str = "/blockchain/blocks/1.0.0";
const CONSENSUS_TOPIC: &str = "/blockchain/consensus/1.0.0";

// Network message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMessage {
    pub transaction: Transaction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMessage {
    pub block: Block,
}

// Events that the network layer sends to the node
#[derive(Debug)]
pub enum NetworkEvent {
    TransactionReceived(Transaction),
    BlockReceived {
        block: Block,
        source: PeerId,
    },
    ConsensusMessageReceived(ConsensusMessage),
    ListeningOn(Multiaddr),
    BlockRequestReceived {
        peer: PeerId,
        request_id: request_response::RequestId,
        start_height: u64,
        limit: u32,
        channel: ResponseChannel<BlockResponse>,
    },
    BlockResponseReceived {
        peer: PeerId,
        request_id: request_response::RequestId,
        blocks: Vec<Block>,
    },
}

pub struct NetworkService {
    pub swarm: Swarm<NodeBehaviour>,
    command_receiver: mpsc::Receiver<NetworkCommand>,
    event_sender: mpsc::Sender<NetworkEvent>,
    pending_requests: HashSet<request_response::RequestId>,
    rate_limiter: std::sync::Arc<std::sync::Mutex<RateLimiter>>,
    reputation: reputation::PeerReputation,
}

#[derive(Debug)]
pub enum NetworkCommand {
    StartListening(Multiaddr),
    Dial(Multiaddr),
    BroadcastTransaction(Transaction),
    BroadcastBlock(Block),
    BroadcastConsensusMessage(ConsensusMessage),
    RequestBlock {
        peer: PeerId,
        start_height: u64,
        limit: u32,
    },
    SendBlockResponse {
        channel: ResponseChannel<BlockResponse>,
        blocks: Vec<Block>,
    },
    SavePeers,
}

pub type NetworkServiceInit = (NetworkService, mpsc::Sender<NetworkCommand>, mpsc::Receiver<NetworkEvent>);

impl NetworkService {
    pub fn new() -> Result<NetworkServiceInit, Box<dyn Error>> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        info!("Local peer id: {:?}", local_peer_id);

        let transport = transport::build_transport(&local_key)?;

        // Gossipsub config
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(std::time::Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            // Peer scoring
            .max_transmit_size(65536)
            .build()
            .expect("Valid gossipsub config");
        
        let mut gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )?;

        // Subscribe to topics
        let tx_topic = IdentTopic::new(TRANSACTION_TOPIC);
        let block_topic = IdentTopic::new(BLOCK_TOPIC);
        let consensus_topic = IdentTopic::new(CONSENSUS_TOPIC);
        
        gossipsub.subscribe(&tx_topic)?;
        gossipsub.subscribe(&block_topic)?;
        gossipsub.subscribe(&consensus_topic)?;
        
        info!("Subscribed to transaction and block topics");

        // Kademlia config
        let kademlia_store = MemoryStore::new(local_peer_id);
        let kademlia = Kademlia::new(local_peer_id, kademlia_store);

        // RequestResponse config
        let request_response = request_response::Behaviour::new(
            iter::once((BlockExchangeProtocol(), ProtocolSupport::Full)),
            request_response::Config::default(),
        );

        let behaviour = NodeBehaviour {
            gossipsub,
            kademlia,
            request_response,
            connection_limits: connection_limits::Behaviour::new(
                connection_limits::ConnectionLimits::default()
                    .with_max_pending_incoming(Some(10))
                    .with_max_pending_outgoing(Some(10))
                    .with_max_established_incoming(Some(50))
                    .with_max_established_outgoing(Some(50))
                    .with_max_established_per_peer(Some(5)),
            ),
        };

        let swarm = Swarm::new(
            transport,
            behaviour,
            local_peer_id,
            Config::with_tokio_executor(),
        );

        let (command_sender, command_receiver) = mpsc::channel(32);
        let (event_sender, event_receiver) = mpsc::channel(100);

        // Initialize rate limiter
        let rate_limiter = std::sync::Arc::new(std::sync::Mutex::new(
            RateLimiter::new(RateLimitConfig::default())
        ));

        let mut service = Self {
            swarm,
            command_receiver,
            event_sender,
            pending_requests: HashSet::new(),
            rate_limiter,
            reputation: reputation::PeerReputation::new(),
        };

        // Load peers from disk
        service.load_peers();

        Ok((
            service,
            command_sender,
            event_receiver,
        ))
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await;
                },
                command = self.command_receiver.recv() => match command {
                    Some(NetworkCommand::StartListening(addr)) => {
                        if let Err(e) = self.swarm.listen_on(addr) {
                            warn!("Failed to listen on address: {}", e);
                        }
                    }
                    Some(NetworkCommand::Dial(addr)) => {
                        if let Err(e) = self.swarm.dial(addr) {
                            warn!("Failed to dial address: {}", e);
                        }
                    }
                    Some(NetworkCommand::BroadcastTransaction(tx)) => {
                        self.broadcast_transaction(tx);
                    }
                    Some(NetworkCommand::BroadcastBlock(block)) => {
                        self.broadcast_block(block);
                    }
                    Some(NetworkCommand::BroadcastConsensusMessage(msg)) => {
                        self.broadcast_consensus_message(msg);
                    }
                    Some(NetworkCommand::RequestBlock { peer, start_height, limit }) => {
                        let request_id = self.swarm.behaviour_mut().request_response.send_request(&peer, BlockRequest { start_height, limit });
                        self.pending_requests.insert(request_id);
                    }
                    Some(NetworkCommand::SendBlockResponse { channel, blocks }) => {
                        if let Err(e) = self.swarm.behaviour_mut().request_response.send_response(channel, BlockResponse { blocks }) {
                            warn!("Failed to send block response: {:?}", e);
                        }
                    }
                    Some(NetworkCommand::SavePeers) => {
                        self.save_peers();
                    }
                    None => return,
                }
            }
        }
    }

    async fn handle_swarm_event<E: std::fmt::Debug>(&mut self, event: SwarmEvent<behaviour::NodeBehaviourEvent, E>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {:?}", address);
                let _ = self.event_sender.send(NetworkEvent::ListeningOn(address)).await;
            }
            SwarmEvent::Behaviour(behaviour::NodeBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source,
                message_id,
                message,
            })) => {
                debug!("Received message from {:?}: {:?}", propagation_source, message_id);
                self.handle_gossipsub_message(message, propagation_source).await;
            }
            SwarmEvent::Behaviour(behaviour::NodeBehaviourEvent::Kademlia(event)) => {
                debug!("Kademlia event: {:?}", event);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connection established with peer: {:?}", peer_id);
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                info!("Connection closed with peer {:?}: {:?}", peer_id, cause);
            }
            SwarmEvent::Behaviour(behaviour::NodeBehaviourEvent::RequestResponse(request_response::Event::Message { peer, message })) => {
                match message {
                    request_response::Message::Request { request_id, request, channel, .. } => {
                        info!("Received block request from {:?}", peer);
                        if let Err(e) = self.event_sender.send(NetworkEvent::BlockRequestReceived { peer, request_id, start_height: request.start_height, limit: request.limit, channel }).await {
                             warn!("Failed to forward block request: {:?}", e);
                        }
                    }
                    request_response::Message::Response { request_id, response } => {
                        info!("Received block response from {:?}", peer);
                        self.pending_requests.remove(&request_id);
                        if let Err(e) = self.event_sender.send(NetworkEvent::BlockResponseReceived { peer, request_id, blocks: response.blocks }).await {
                             warn!("Failed to forward block response: {:?}", e);
                        }
                    }
                }
            }
            SwarmEvent::Behaviour(behaviour::NodeBehaviourEvent::RequestResponse(request_response::Event::OutboundFailure { request_id, error, .. })) => {
                warn!("Request {:?} failed: {:?}", request_id, error);
                self.pending_requests.remove(&request_id);
            }
            _ => {}
        }
    }

    async fn handle_gossipsub_message(&mut self, message: gossipsub::Message, source: PeerId) {
        let topic = message.topic.as_str();
        
        match topic {
            TRANSACTION_TOPIC => {
                // Check reputation
                if self.reputation.is_banned(&source) {
                    warn!("Ignoring transaction from banned peer: {:?}", source);
                    return;
                }

                // Check rate limit
                {
                    let mut limiter = self.rate_limiter.lock().unwrap();
                    if let Err(e) = limiter.check_and_consume(&source, MessageType::Transaction) {
                        warn!("Rate limit exceeded for transaction from {:?}: {}", source, e);
                        self.reputation.report_bad_behavior(source, 5); // Minor penalty for rate limit
                        return;
                    }
                }
                
                match serde_json::from_slice::<TransactionMessage>(&message.data) {
                    Ok(tx_msg) => {
                        info!("Received transaction from network");
                        self.reputation.report_good_behavior(source); // Reward valid message
                        if let Err(e) = self.event_sender.send(NetworkEvent::TransactionReceived(tx_msg.transaction)).await {
                            warn!("Failed to forward transaction to node: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to deserialize transaction message: {}", e);
                        self.reputation.report_bad_behavior(source, 10); // Penalty for invalid format
                    }
                }
            }
            BLOCK_TOPIC => {
                match serde_json::from_slice::<BlockMessage>(&message.data) {
                    Ok(block_msg) => {
                        info!("Received block from network");
                        if let Err(e) = self.event_sender.send(NetworkEvent::BlockReceived { block: block_msg.block, source }).await {
                            warn!("Failed to forward block to node: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to deserialize block message: {}", e);
                    }
                }
            }
            CONSENSUS_TOPIC => {
                // Check rate limit
                {
                    let mut limiter = self.rate_limiter.lock().unwrap();
                    if let Err(e) = limiter.check_and_consume(&source, MessageType::ConsensusMessage) {
                        warn!("Rate limit exceeded for consensus message from {:?}: {}", source, e);
                        return;
                    }
                }
                
                match serde_json::from_slice::<ConsensusMessage>(&message.data) {
                    Ok(msg) => {
                        debug!("Received consensus message");
                        if let Err(e) = self.event_sender.send(NetworkEvent::ConsensusMessageReceived(msg)).await {
                            warn!("Failed to forward consensus message: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to deserialize consensus message: {}", e);
                    }
                }
            }
            _ => {
                debug!("Received message on unknown topic: {}", topic);
            }
        }
    }

    fn broadcast_transaction(&mut self, tx: Transaction) {
        let msg = TransactionMessage { transaction: tx };
        let topic = IdentTopic::new(TRANSACTION_TOPIC);
        
        match serde_json::to_vec(&msg) {
            Ok(data) => {
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    warn!("Failed to broadcast transaction: {}", e);
                } else {
                    info!("Broadcasted transaction to network");
                }
            }
            Err(e) => {
                warn!("Failed to serialize transaction: {}", e);
            }
        }
    }

    fn broadcast_block(&mut self, block: Block) {
        let msg = BlockMessage { block };
        let topic = IdentTopic::new(BLOCK_TOPIC);
        
        match serde_json::to_vec(&msg) {
            Ok(data) => {
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    warn!("Failed to broadcast block: {}", e);
                } else {
                    info!("Broadcasted block to network");
                }
            }
            Err(e) => {
                warn!("Failed to serialize block: {}", e);
            }
        }
    }

    fn broadcast_consensus_message(&mut self, msg: ConsensusMessage) {
        let topic = IdentTopic::new(CONSENSUS_TOPIC);
        
        match serde_json::to_vec(&msg) {
            Ok(data) => {
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    warn!("Failed to broadcast consensus message: {}", e);
                } else {
                    debug!("Broadcasted consensus message to network");
                }
            }
            Err(e) => {
                warn!("Failed to serialize consensus message: {}", e);
            }
        }
    }

    pub fn save_peers(&mut self) {
        let mut peers = Vec::new();
        for bucket in self.swarm.behaviour_mut().kademlia.kbuckets() {
            for entry in bucket.iter() {
                peers.push(*entry.node.key.preimage());
            }
        }
        let peer_data = serde_json::to_string(&peers).unwrap_or_default();
        if let Err(e) = std::fs::write("peers.json", peer_data) {
            warn!("Failed to save peers: {}", e);
        } else {
            info!("Saved {} peers to disk", peers.len());
        }
    }

    pub fn load_peers(&mut self) {
        if let Ok(data) = std::fs::read_to_string("peers.json") {
            if let Ok(peers) = serde_json::from_str::<Vec<PeerId>>(&data) {
                info!("Loaded {} peers from disk", peers.len());
                for peer in peers {
                    self.swarm.behaviour_mut().kademlia.add_address(&peer, "/ip4/127.0.0.1/tcp/0".parse().unwrap()); // Placeholder address, in real world we'd save addrs too
                }
            }
        }
    }
}

pub fn init() {
    println!("Network initialized (use NetworkService::new)");
}
