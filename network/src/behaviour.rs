use libp2p::{
    gossipsub,
    kad::{store::MemoryStore, Behaviour as Kademlia},
    swarm::NetworkBehaviour,
};

#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: Kademlia<MemoryStore>,
}
