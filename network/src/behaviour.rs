use crate::protocol::BlockExchangeCodec;
use libp2p::{
    gossipsub,
    kad::{store::MemoryStore, Behaviour as Kademlia},
    request_response::Behaviour as RequestResponse,
    connection_limits,
    swarm::NetworkBehaviour,
};

#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: Kademlia<MemoryStore>,
    pub request_response: RequestResponse<BlockExchangeCodec>,
    pub connection_limits: connection_limits::Behaviour,
}
