use crate::protocol::BlockExchangeCodec;
use libp2p::{
    connection_limits, gossipsub,
    kad::{store::MemoryStore, Behaviour as Kademlia},
    ping,
    request_response::Behaviour as RequestResponse,
    swarm::NetworkBehaviour,
};

#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: Kademlia<MemoryStore>,
    pub request_response: RequestResponse<BlockExchangeCodec>,
    pub connection_limits: connection_limits::Behaviour,
    pub ping: ping::Behaviour,
}
