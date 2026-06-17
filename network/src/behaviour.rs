use crate::protocol::BlockExchangeCodec;
use libp2p::{
    connection_limits, gossipsub,
    kad::{store::MemoryStore, Behaviour as Kademlia, KademliaConfig, KademliaEvent},
    request_response::{Behaviour as RequestResponse, RequestResponseConfig, RequestResponseEvent},
    swarm::NetworkBehaviour,
    PeerId,
};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

// Custom events for the behaviour
#[derive(Debug)]
pub enum BehaviourEvent {
    NewBlock(Block),
    NewTransaction(Transaction),
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    BlockRequest(BlockHash, PeerId),
    BlockResponse(Block, PeerId),
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourEvent")]
pub struct NodeBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: Kademlia<MemoryStore>,
    pub request_response: RequestResponse<BlockExchangeCodec>,
    pub connection_limits: connection_limits::Behaviour,
    
    // FIX: Add state tracking
    #[behaviour(ignore)]
    connected_peers: HashMap<PeerId, ConnectionInfo>,
    
    #[behaviour(ignore)]
    peer_scores: HashMap<PeerId, f64>,
    
    #[behaviour(ignore)]
    banned_peers: HashSet<PeerId>,
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub connected_at: std::time::Instant,
    pub user_agent: Option<String>,
    pub protocol_version: Option<String>,
    pub best_height: u64,
    pub messages_received: u64,
    pub messages_sent: u64,
}

impl NodeBehaviour {
    pub fn new(
        local_peer_id: PeerId,
        gossipsub_config: gossipsub::Config,
        kademlia_config: KademliaConfig,
        request_response_config: RequestResponseConfig,
        connection_limits_config: connection_limits::Config,
    ) -> Self {
        let gossipsub = gossipsub::Behaviour::new(gossipsub_config).unwrap();
        let kademlia = Kademlia::with_config(local_peer_id, MemoryStore::new(local_peer_id), kademlia_config);
        let request_response = RequestResponse::new(BlockExchangeCodec::default(), request_response_config);
        
        Self {
            gossipsub,
            kademlia,
            request_response,
            connection_limits: connection_limits::Behaviour::new(connection_limits_config),
            connected_peers: HashMap::new(),
            peer_scores: HashMap::new(),
            banned_peers: HashSet::new(),
        }
    }
    
    // Broadcast a block to all peers
    pub fn broadcast_block(&mut self, block: Block) -> Result<(), gossipsub::PublishError> {
        let block_data = bincode::serialize(&block).unwrap();
        self.gossipsub.publish(gossipsub::TopicHash::from_raw("blocks"), block_data)
    }
    
    // Broadcast a transaction
    pub fn broadcast_transaction(&mut self, tx: Transaction) -> Result<(), gossipsub::PublishError> {
        let tx_data = bincode::serialize(&tx).unwrap();
        self.gossipsub.publish(gossipsub::TopicHash::from_raw("transactions"), tx_data)
    }
    
    // Request a block from a specific peer
    pub fn request_block(&mut self, peer_id: PeerId, block_hash: BlockHash) {
        let request = BlockRequest { hash: block_hash };
        self.request_response.send_request(&peer_id, request);
    }
    
    // Add peer and track their info
    pub fn add_peer(&mut self, peer_id: PeerId, info: ConnectionInfo) {
        if self.banned_peers.contains(&peer_id) {
            tracing::warn!("Attempted to add banned peer: {}", peer_id);
            return;
        }
        self.connected_peers.insert(peer_id, info);
        self.peer_scores.insert(peer_id, 1.0); // Initial score
    }
    
    // Remove peer
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        self.connected_peers.remove(peer_id);
        self.peer_scores.remove(peer_id);
    }
    
    // Update peer score (for reputation system)
    pub fn update_peer_score(&mut self, peer_id: &PeerId, delta: f64) {
        if let Some(score) = self.peer_scores.get_mut(peer_id) {
            *score += delta;
            if *score < -10.0 {
                tracing::warn!("Peer {} banned due to low score", peer_id);
                self.banned_peers.insert(peer_id.clone());
                self.remove_peer(peer_id);
            }
        }
    }
    
    // Get connected peers
    pub fn get_peers(&self) -> Vec<PeerId> {
        self.connected_peers.keys().cloned().collect()
    }
    
    // Get peer count
    pub fn peer_count(&self) -> usize {
        self.connected_peers.len()
    }
    
    // Handle Kademlia events
    fn handle_kademlia_event(&mut self, event: KademliaEvent) -> Vec<BehaviourEvent> {
        match event {
            KademliaEvent::RoutingUpdated { peer, .. } => {
                tracing::debug!("Routing updated for peer: {}", peer);
                vec![BehaviourEvent::PeerConnected(peer)]
            }
            KademliaEvent::OutboundQueryProgressed { result, .. } => {
                // Handle discovery results
                tracing::debug!("Kademlia query progressed: {:?}", result);
                vec![]
            }
            _ => vec![],
        }
    }
    
    // Handle request-response events
    fn handle_request_response_event(&mut self, event: RequestResponseEvent<BlockRequest, BlockResponse>) -> Vec<BehaviourEvent> {
        match event {
            RequestResponseEvent::Message { peer, message } => {
                match message {
                    libp2p::request_response::RequestResponseMessage::Request { request, channel, .. } => {
                        // Handle incoming block request
                        tracing::info!("Block request from {} for hash {:?}", peer, request.hash);
                        // Would need to lookup block in DB and respond
                        vec![BehaviourEvent::BlockRequest(request.hash, peer)]
                    }
                    libp2p::request_response::RequestResponseMessage::Response { response, .. } => {
                        // Handle incoming block response
                        tracing::info!("Received block from {}: {:?}", peer, response.block.hash());
                        vec![BehaviourEvent::BlockResponse(response.block, peer)]
                    }
                }
            }
            RequestResponseEvent::ResponseSent { peer, .. } => {
                tracing::debug!("Response sent to {}", peer);
                vec![]
            }
        }
    }
}

// Helper types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlockRequest {
    pub hash: BlockHash,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlockResponse {
    pub block: Block,
}

// Re-export from common
use common::types::{Block, BlockHash, Transaction};