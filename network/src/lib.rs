mod behaviour;
mod transport;

use behaviour::NodeBehaviour;
use futures::StreamExt;
use libp2p::{
    gossipsub, identity,
    kad::{store::MemoryStore, Behaviour as Kademlia},
    swarm::{Config, SwarmEvent},
    Multiaddr, PeerId, Swarm,
};
use std::error::Error;
use tokio::sync::mpsc;

pub struct NetworkService {
    pub swarm: Swarm<NodeBehaviour>,
    command_receiver: mpsc::Receiver<NetworkCommand>,
}

pub enum NetworkCommand {
    StartListening(Multiaddr),
    Dial(Multiaddr),
}

impl NetworkService {
    pub fn new() -> Result<(Self, mpsc::Sender<NetworkCommand>), Box<dyn Error>> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        println!("Local peer id: {:?}", local_peer_id);

        let transport = transport::build_transport(&local_key)?;

        // Gossipsub config
        let gossipsub_config = gossipsub::Config::default();
        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )?;

        // Kademlia config
        let kademlia_store = MemoryStore::new(local_peer_id);
        let kademlia = Kademlia::new(local_peer_id, kademlia_store);

        let behaviour = NodeBehaviour {
            gossipsub,
            kademlia,
        };

        let swarm = Swarm::new(
            transport,
            behaviour,
            local_peer_id,
            Config::with_tokio_executor(),
        );

        let (command_sender, command_receiver) = mpsc::channel(32);

        Ok((
            Self {
                swarm,
                command_receiver,
            },
            command_sender,
        ))
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Listening on {:?}", address);
                    }
                    _ => {}
                },
                command = self.command_receiver.recv() => match command {
                    Some(NetworkCommand::StartListening(addr)) => {
                        let _ = self.swarm.listen_on(addr);
                    }
                    Some(NetworkCommand::Dial(addr)) => {
                        let _ = self.swarm.dial(addr);
                    }
                    None => return,
                }
            }
        }
    }
}

pub fn init() {
    println!("Network initialized (use NetworkService::new)");
}
