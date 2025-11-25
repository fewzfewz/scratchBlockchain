use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::Path;
use tracing::info;

/// Persistent storage for peer multiaddrs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerStore {
    /// Set of known peer multiaddrs
    peers: HashSet<String>,
    /// Path to the peers file
    #[serde(skip)]
    file_path: String,
}

impl PeerStore {
    /// Create a new peer store
    pub fn new(file_path: &str) -> Result<Self, Box<dyn Error>> {
        let mut store = Self {
            peers: HashSet::new(),
            file_path: file_path.to_string(),
        };

        // Try to load existing peers
        if Path::new(file_path).exists() {
            store.load()?;
        }

        Ok(store)
    }

    /// Add a peer to the store
    pub fn add_peer(&mut self, addr: &Multiaddr) {
        let addr_str = addr.to_string();
        if self.peers.insert(addr_str.clone()) {
            info!("Added peer to store: {}", addr_str);
        }
    }

    /// Get all known peers
    pub fn get_peers(&self) -> Vec<Multiaddr> {
        self.peers
            .iter()
            .filter_map(|addr_str| addr_str.parse().ok())
            .collect()
    }

    /// Save peers to disk
    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string_pretty(&self.peers)?;
        fs::write(&self.file_path, json)?;
        info!("Saved {} peers to {}", self.peers.len(), self.file_path);
        Ok(())
    }

    /// Load peers from disk
    fn load(&mut self) -> Result<(), Box<dyn Error>> {
        let contents = fs::read_to_string(&self.file_path)?;
        self.peers = serde_json::from_str(&contents)?;
        info!("Loaded {} peers from {}", self.peers.len(), self.file_path);
        Ok(())
    }

    /// Remove a peer from the store
    pub fn remove_peer(&mut self, addr: &Multiaddr) {
        let addr_str = addr.to_string();
        if self.peers.remove(&addr_str) {
            info!("Removed peer from store: {}", addr_str);
        }
    }

    /// Get the number of stored peers
    pub fn len(&self) -> usize {
        self.peers.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_peer_store_add_and_get() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("peers.json");
        let mut store = PeerStore::new(path.to_str().unwrap()).unwrap();

        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        store.add_peer(&addr);

        assert_eq!(store.len(), 1);
        let peers = store.get_peers();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0], addr);
    }

    #[test]
    fn test_peer_store_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("peers.json");

        // Create store and add peers
        {
            let mut store = PeerStore::new(path.to_str().unwrap()).unwrap();
            let addr1: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
            let addr2: Multiaddr = "/ip4/127.0.0.1/tcp/4002".parse().unwrap();

            store.add_peer(&addr1);
            store.add_peer(&addr2);
            store.save().unwrap();

            assert_eq!(store.len(), 2);
        }

        // Load peers in new store
        {
            let store = PeerStore::new(path.to_str().unwrap()).unwrap();
            assert_eq!(store.len(), 2);

            let peers = store.get_peers();
            assert_eq!(peers.len(), 2);
        }
    }

    #[test]
    fn test_peer_store_remove() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("peers.json");
        let mut store = PeerStore::new(path.to_str().unwrap()).unwrap();

        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        store.add_peer(&addr);
        assert_eq!(store.len(), 1);

        store.remove_peer(&addr);
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn test_peer_store_duplicates() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("peers.json");
        let mut store = PeerStore::new(path.to_str().unwrap()).unwrap();

        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        store.add_peer(&addr);
        store.add_peer(&addr); // Add same peer again

        assert_eq!(store.len(), 1); // Should only have one entry
    }
}
