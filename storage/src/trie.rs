//! # Patricia Merkle Trie Implementation
//!
//! This module implements a Patricia Merkle Trie (also known as Merkle Patricia Trie)
//! for efficient state storage and verification.
//!
//! ## What is a Patricia Merkle Trie?
//! A hybrid data structure that combines:
//! - **Patricia Trie**: Space-optimized prefix tree for key-value storage
//! - **Merkle Tree**: Cryptographic hash tree for verification
//!
//! ## Use Cases
//! - Ethereum state storage (accounts, storage slots)
//! - Blockchain state root verification
//! - Merkle proofs for light clients
//!
//! ## Node Types
//! - **Empty**: Null node (hash of empty)
//! - **Leaf**: Terminal node with (path, value)
//! - **Extension**: Shared path prefix pointing to another node
//! - **Branch**: 16-way split node (for hex-nibble paths)

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, RwLock};
use tracing::debug;

use crate::db::ColumnFamily;
use crate::db::KeyValueStore;

/// Nibble (4-bit value) used for trie paths
/// Hex characters (0-15) represent half-bytes
type Nibble = u8;

/// Path in the trie as a sequence of nibbles
/// Example: 0xA, 0xB, 0xC = 0xABC
type NibblePath = Vec<Nibble>;

/// Hash type for node references (32 bytes, SHA256)
pub type NodeHash = [u8; 32];

/// Empty trie hash (SHA256 of empty string)
pub const EMPTY_TRIE_HASH: NodeHash = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14,
    0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c,
    0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
];

/// Convert bytes to nibble path (each byte becomes 2 nibbles)
fn bytes_to_nibbles(bytes: &[u8]) -> NibblePath {
    let mut nibbles = Vec::with_capacity(bytes.len() * 2);
    for byte in bytes {
        nibbles.push(byte >> 4);      // High nibble
        nibbles.push(byte & 0x0F);    // Low nibble
    }
    nibbles
}

/// Convert nibble path back to bytes (requires even length)
#[allow(dead_code)]
fn nibbles_to_bytes(nibbles: &[Nibble]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(nibbles.len() / 2);
    for chunk in nibbles.chunks(2) {
        if chunk.len() == 2 {
            bytes.push((chunk[0] << 4) | chunk[1]);
        }
    }
    bytes
}

/// Find common prefix length between two nibble paths
/// 
/// # Returns
/// Number of matching nibbles from the start
fn common_prefix_len(a: &[Nibble], b: &[Nibble]) -> usize {
    a.iter()
        .zip(b.iter())
        .take_while(|(x, y)| x == y)
        .count()
}

/// Trie node types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum TrieNode {
    /// Empty node (represents absence of data)
    Empty,
    
    /// Leaf node: contains a full path and a value
    /// Path is the remaining nibbles after following parent paths
    Leaf {
        path: NibblePath,
        value: Vec<u8>,
    },
    
    /// Extension node: shares a common path prefix with multiple children
    /// Points to a child node (usually a branch)
    Extension {
        path: NibblePath,
        child: NodeHash,
    },
    
    /// Branch node: 16-way split for the next nibble
    /// Each child corresponds to a hex digit (0-15)
    Branch {
        children: [Option<NodeHash>; 16],
        value: Option<Vec<u8>>,  // Optional value at this exact path
    },
}

impl TrieNode {
    /// Compute the hash of this node
    pub fn hash(&self) -> NodeHash {
        let encoded = self.encode();
        let mut hasher = Sha256::new();
        hasher.update(&encoded);
        hasher.finalize().into()
    }
    
    /// Encode node for hashing and storage
    /// Uses JSON serialization for simplicity (production could use RLP)
    fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
    
    /// Decode node from bytes
    fn decode(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        Ok(serde_json::from_slice(bytes)?)
    }
    
    /// Check if node is empty
    pub fn is_empty(&self) -> bool {
        matches!(self, TrieNode::Empty)
    }
}

/// Patricia Merkle Trie with persistent storage and caching
pub struct PatriciaTrie {
    /// Root node hash
    root: Arc<RwLock<NodeHash>>,
    
    /// In-memory node cache (hash -> node)
    /// Speeds up repeated access to the same nodes
    node_cache: Arc<RwLock<HashMap<NodeHash, TrieNode>>>,
    
    /// Persistent storage backend
    db: Arc<dyn KeyValueStore>,
    
    /// Statistics for monitoring
    stats: Arc<RwLock<TrieStats>>,
}

/// Trie statistics for monitoring
#[derive(Debug, Default, Clone)]
pub struct TrieStats {
    pub total_nodes: usize,
    pub cached_nodes: usize,
    pub persisted_nodes: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl PatriciaTrie {
    /// Create a new trie with the given database path
    /// 
    /// # Arguments
    /// * `db_path` - Path to the sled database directory
    /// 
    /// # Returns
    /// * `Result<Self>` - New trie instance
    pub fn new(db: Arc<dyn KeyValueStore>) -> Result<Self, Box<dyn Error>> {
        // Try to load existing root from database
        let (root, cache) =
            if let Some(root_bytes) = db.get(ColumnFamily::State, b"root")? {
                let mut root_hash = [0u8; 32];
                root_hash.copy_from_slice(&root_bytes);
                (root_hash, HashMap::new())
            } else {
                // Empty trie has empty root
                let empty_node = TrieNode::Empty;
                let root_hash = empty_node.hash();
                
                let mut c = HashMap::new();
                c.insert(root_hash, empty_node.clone());
                
                db.put(ColumnFamily::State, b"root", &root_hash)?;
                db.put(ColumnFamily::State, &root_hash, &empty_node.encode())?;
                db.flush()?;
                
                (root_hash, c)
            };
        
        Ok(Self {
            root: Arc::new(RwLock::new(root)),
            node_cache: Arc::new(RwLock::new(cache)),
            db,
            stats: Arc::new(RwLock::new(TrieStats::default())),
        })
    }
    
    /// Get the current root hash
    pub fn root_hash(&self) -> NodeHash {
        *self.root.read().unwrap()
    }
    
    /// Insert a key-value pair into the trie
    /// 
    /// # Arguments
    /// * `key` - Key bytes (will be converted to nibbles)
    /// * `value` - Value bytes to store
    pub fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), Box<dyn Error>> {
        let path = bytes_to_nibbles(key);
        let current_root = self.root_hash();
        
        let (new_root, changed_nodes) = self.insert_at(current_root, &path, value.to_vec())?;
        
        // Update root
        *self.root.write().unwrap() = new_root;
        
        // Persist the new root pointer so the trie survives restarts
        self.db.put(ColumnFamily::State, b"root", &new_root)?;
        
        // Persist changed nodes to disk
        for (hash, node) in changed_nodes {
            self.persist_node(hash, &node)?;
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.total_nodes += 1;
        }
        
        debug!("Inserted key: {:?} -> root: {:?}", hex::encode(key), hex::encode(&new_root));
        Ok(())
    }
    
    /// Get a value by key
    /// 
    /// # Arguments
    /// * `key` - Key bytes to look up
    /// 
    /// # Returns
    /// * `Ok(Some(value))` if found
    /// * `Ok(None)` if not found
    /// * `Err` on error
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        let path = bytes_to_nibbles(key);
        let current_root = self.root_hash();
        self.get_at(current_root, &path)
    }
    
    /// Delete a key from the trie
    /// 
    /// # Arguments
    /// * `key` - Key bytes to delete
    pub fn delete(&mut self, key: &[u8]) -> Result<(), Box<dyn Error>> {
        let path = bytes_to_nibbles(key);
        let current_root = self.root_hash();
        
        let (new_root, changed_nodes) = self.delete_at(current_root, &path)?;
        
        // Update root
        *self.root.write().unwrap() = new_root;
        
        // Persist the new root pointer so the trie survives restarts
        self.db.put(ColumnFamily::State, b"root", &new_root)?;
        
        // Persist changed nodes
        for (hash, node) in changed_nodes {
            self.persist_node(hash, &node)?;
        }
        
        debug!("Deleted key: {:?} -> root: {:?}", hex::encode(key), hex::encode(&new_root));
        Ok(())
    }
    
    /// Generate a Merkle proof for a key
    /// 
    /// A Merkle proof allows verification that a key-value pair exists
    /// without having the entire trie.
    /// 
    /// # Returns
    /// * `Vec<Vec<u8>>` - Proof nodes (each is a serialized node)
    pub fn get_proof(&self, key: &[u8]) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        let path = bytes_to_nibbles(key);
        let mut proof = Vec::new();
        self.collect_proof(self.root_hash(), &path, &mut proof)?;
        Ok(proof)
    }
    
    /// Verify a Merkle proof against a root hash
    /// 
    /// # Arguments
    /// * `root_hash` - Trusted root hash
    /// * `key` - Key being verified
    /// * `value` - Value being verified
    /// * `proof` - Proof nodes from get_proof()
    /// 
    /// # Returns
    /// * `bool` - True if proof is valid
    pub fn verify_proof(
        root_hash: NodeHash,
        key: &[u8],
        value: &[u8],
        proof: &[Vec<u8>],
    ) -> Result<bool, Box<dyn Error>> {
        let path = bytes_to_nibbles(key);
        let mut current_hash = root_hash;
        
        // Iterate through proof nodes, recomputing hashes
        for node_bytes in proof {
            let node = TrieNode::decode(node_bytes)?;
            let computed_hash = node.hash();
            
            if computed_hash != current_hash {
                return Ok(false);
            }
            
            // Navigate to next node based on path
            match node {
                TrieNode::Leaf { path: leaf_path, value: leaf_value } => {
                    if path == leaf_path && leaf_value == value {
                        return Ok(true);
                    }
                    return Ok(false);
                }
                TrieNode::Extension { path: ext_path, child } => {
                    if path.starts_with(&ext_path) {
                        current_hash = child;
                    } else {
                        return Ok(false);
                    }
                }
                TrieNode::Branch { children, .. } => {
                    if let Some(next_idx) = path.first() {
                        if let Some(child_hash) = children[*next_idx as usize] {
                            current_hash = child_hash;
                        } else {
                            return Ok(false);
                        }
                    }
                }
                TrieNode::Empty => return Ok(false),
            }
        }
        
        Ok(false)
    }
    
    /// Get trie statistics
    pub fn get_stats(&self) -> TrieStats {
        self.stats.read().unwrap().clone()
    }
    
    // ========================================================================
    // Internal Methods
    // ========================================================================
    
    /// Get a node by hash (with caching)
    fn get_node(&self, hash: NodeHash) -> Result<TrieNode, Box<dyn Error>> {
        // Check in-memory cache first
        {
            let cache = self.node_cache.read().unwrap();
            if let Some(node) = cache.get(&hash) {
                let mut stats = self.stats.write().unwrap();
                stats.cache_hits += 1;
                return Ok(node.clone());
            }
        }
        
        // Cache miss - load from disk
        {
            let mut stats = self.stats.write().unwrap();
            stats.cache_misses += 1;
        }
        
        if let Some(data) = self.db.get(ColumnFamily::State, &hash)? {
            let node = TrieNode::decode(&data)?;
            
            // Cache for future use
            {
                let mut cache = self.node_cache.write().unwrap();
                cache.insert(hash, node.clone());
                
                let mut stats = self.stats.write().unwrap();
                stats.cached_nodes = cache.len();
                stats.persisted_nodes += 1;
            }
            
            Ok(node)
        } else {
            Err(format!("Node not found: {:?}", hex::encode(&hash)).into())
        }
    }
    
    /// Persist a node to disk and cache
    fn persist_node(&mut self, hash: NodeHash, node: &TrieNode) -> Result<(), Box<dyn Error>> {
        let encoded = node.encode();
        self.db.put(ColumnFamily::State, &hash, &encoded)?;
        self.db.flush()?;
        
        // Update cache
        {
            let mut cache = self.node_cache.write().unwrap();
            cache.insert(hash, node.clone());
        }
        
        Ok(())
    }
    
    /// Insert at a specific node (recursive)
    fn insert_at(
        &mut self,
        node_hash: NodeHash,
        path: &[Nibble],
        value: Vec<u8>,
    ) -> Result<(NodeHash, Vec<(NodeHash, TrieNode)>), Box<dyn Error>> {
        let node = self.get_node(node_hash)?;
        let mut changed = Vec::new();
        
        let new_node = match node {
            TrieNode::Empty => {
                // Insert as leaf
                TrieNode::Leaf {
                    path: path.to_vec(),
                    value,
                }
            }
            
            TrieNode::Leaf { path: leaf_path, value: leaf_value } => {
                if path == leaf_path.as_slice() {
                    // Update existing leaf
                    TrieNode::Leaf {
                        path: leaf_path,
                        value,
                    }
                } else {
                    // Split into branch
                    let common_len = common_prefix_len(path, &leaf_path);
                    
                    if common_len == 0 {
                        // Create branch at root
                        let mut children = [None; 16];
                        
                        // Insert old leaf
                        let old_leaf = TrieNode::Leaf {
                            path: leaf_path[1..].to_vec(),
                            value: leaf_value,
                        };
                        let old_hash = old_leaf.hash();
                        changed.push((old_hash, old_leaf));
                        children[leaf_path[0] as usize] = Some(old_hash);
                        
                        // Insert new leaf
                        let new_leaf = TrieNode::Leaf {
                            path: path[1..].to_vec(),
                            value,
                        };
                        let new_hash = new_leaf.hash();
                        changed.push((new_hash, new_leaf));
                        children[path[0] as usize] = Some(new_hash);
                        
                        TrieNode::Branch {
                            children,
                            value: None,
                        }
                    } else {
                        // Create extension + branch
                        let mut children = [None; 16];
                        
                        // Old leaf continuation
                        if common_len < leaf_path.len() {
                            let old_leaf = TrieNode::Leaf {
                                path: leaf_path[common_len + 1..].to_vec(),
                                value: leaf_value,
                            };
                            let old_hash = old_leaf.hash();
                            changed.push((old_hash, old_leaf));
                            children[leaf_path[common_len] as usize] = Some(old_hash);
                        }
                        
                        // New leaf continuation
                        if common_len < path.len() {
                            let new_leaf = TrieNode::Leaf {
                                path: path[common_len + 1..].to_vec(),
                                value,
                            };
                            let new_hash = new_leaf.hash();
                            changed.push((new_hash, new_leaf));
                            children[path[common_len] as usize] = Some(new_hash);
                        }
                        
                        let branch = TrieNode::Branch {
                            children,
                            value: None,
                        };
                        let branch_hash = branch.hash();
                        changed.push((branch_hash, branch.clone()));
                        
                        if common_len > 0 {
                            TrieNode::Extension {
                                path: path[..common_len].to_vec(),
                                child: branch_hash,
                            }
                        } else {
                            branch
                        }
                    }
                }
            }
            
            TrieNode::Extension { path: ext_path, child } => {
                let common_len = common_prefix_len(path, &ext_path);
                
                if common_len == ext_path.len() {
                    // Continue down the extension
                    let (new_child, child_changes) = self.insert_at(child, &path[common_len..], value)?;
                    changed.extend(child_changes);
                    
                    TrieNode::Extension {
                        path: ext_path,
                        child: new_child,
                    }
                } else {
                    // Split extension
                    let mut children = [None; 16];
                    
                    // Old extension continues
                    let old_ext = TrieNode::Extension {
                        path: ext_path[common_len + 1..].to_vec(),
                        child,
                    };
                    let old_hash = old_ext.hash();
                    changed.push((old_hash, old_ext));
                    children[ext_path[common_len] as usize] = Some(old_hash);
                    
                    // New path
                    let new_leaf = TrieNode::Leaf {
                        path: path[common_len + 1..].to_vec(),
                        value,
                    };
                    let new_hash = new_leaf.hash();
                    changed.push((new_hash, new_leaf));
                    children[path[common_len] as usize] = Some(new_hash);
                    
                    let branch = TrieNode::Branch {
                        children,
                        value: None,
                    };
                    let branch_hash = branch.hash();
                    changed.push((branch_hash, branch.clone()));
                    
                    if common_len > 0 {
                        TrieNode::Extension {
                            path: ext_path[..common_len].to_vec(),
                            child: branch_hash,
                        }
                    } else {
                        branch
                    }
                }
            }
            
            TrieNode::Branch { mut children, value: branch_value } => {
                if path.is_empty() {
                    // Update branch value
                    TrieNode::Branch {
                        children,
                        value: Some(value),
                    }
                } else {
                    // Insert into appropriate child
                    let idx = path[0] as usize;
                    let child_hash = children[idx].unwrap_or_else(|| TrieNode::Empty.hash());
                    let (new_child, child_changes) = self.insert_at(child_hash, &path[1..], value)?;
                    changed.extend(child_changes);
                    children[idx] = Some(new_child);
                    
                    TrieNode::Branch {
                        children,
                        value: branch_value,
                    }
                }
            }
        };
        
        let new_hash = new_node.hash();
        changed.push((new_hash, new_node.clone()));
        Ok((new_hash, changed))
    }
    
    /// Get at a specific node (recursive)
    fn get_at(&self, node_hash: NodeHash, path: &[Nibble]) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        let node = self.get_node(node_hash)?;
        
        match node {
            TrieNode::Empty => Ok(None),
            
            TrieNode::Leaf { path: leaf_path, value } => {
                if path == leaf_path.as_slice() {
                    Ok(Some(value))
                } else {
                    Ok(None)
                }
            }
            
            TrieNode::Extension { path: ext_path, child } => {
                if path.starts_with(&ext_path) {
                    self.get_at(child, &path[ext_path.len()..])
                } else {
                    Ok(None)
                }
            }
            
            TrieNode::Branch { children, value } => {
                if path.is_empty() {
                    Ok(value)
                } else {
                    let idx = path[0] as usize;
                    if let Some(child_hash) = children[idx] {
                        self.get_at(child_hash, &path[1..])
                    } else {
                        Ok(None)
                    }
                }
            }
        }
    }
    
    /// Delete at a specific node (recursive)
    fn delete_at(
        &mut self,
        node_hash: NodeHash,
        path: &[Nibble],
    ) -> Result<(NodeHash, Vec<(NodeHash, TrieNode)>), Box<dyn Error>> {
        let node = self.get_node(node_hash)?;
        let mut changed = Vec::new();
        
        let new_node = match node {
            TrieNode::Empty => TrieNode::Empty,
            
            TrieNode::Leaf { path: ref leaf_path, .. } => {
                if path == leaf_path.as_slice() {
                    TrieNode::Empty
                } else {
                    node.clone()
                }
            }
            
            TrieNode::Extension { path: ref ext_path, child } => {
                if path.starts_with(ext_path) {
                    let (new_child, child_changes) = self.delete_at(child, &path[ext_path.len()..])?;
                    changed.extend(child_changes);
                    
                    // If child became empty, this extension should be removed
                    if new_child == EMPTY_TRIE_HASH {
                        TrieNode::Empty
                    } else {
                        TrieNode::Extension {
                            path: ext_path.clone(),
                            child: new_child,
                        }
                    }
                } else {
                    node.clone()
                }
            }
            
            TrieNode::Branch { mut children, value } => {
                if path.is_empty() {
                    TrieNode::Branch {
                        children,
                        value: None,
                    }
                } else {
                    let idx = path[0] as usize;
                    if let Some(child_hash) = children[idx] {
                        let (new_child, child_changes) = self.delete_at(child_hash, &path[1..])?;
                        changed.extend(child_changes);
                        children[idx] = Some(new_child);
                    }
                    
                    TrieNode::Branch {
                        children,
                        value,
                    }
                }
            }
        };
        
        let new_hash = new_node.hash();
        changed.push((new_hash, new_node));
        Ok((new_hash, changed))
    }
    
    /// Collect proof nodes (recursive)
    fn collect_proof(
        &self,
        node_hash: NodeHash,
        path: &[Nibble],
        proof: &mut Vec<Vec<u8>>,
    ) -> Result<(), Box<dyn Error>> {
        let node = self.get_node(node_hash)?;
        proof.push(node.encode());
        
        match node {
            TrieNode::Empty | TrieNode::Leaf { .. } => Ok(()),
            
            TrieNode::Extension { path: ext_path, child } => {
                if path.starts_with(&ext_path) {
                    self.collect_proof(child, &path[ext_path.len()..], proof)
                } else {
                    Ok(())
                }
            }
            
            TrieNode::Branch { children, .. } => {
                if !path.is_empty() {
                    let idx = path[0] as usize;
                    if let Some(child_hash) = children[idx] {
                        self.collect_proof(child_hash, &path[1..], proof)?;
                    }
                }
                Ok(())
            }
        }
    }
    
    /// Flush all pending writes to disk
    pub fn flush(&self) -> Result<(), Box<dyn Error>> {
        // Store root hash
        let root = self.root_hash();
        self.db.put(ColumnFamily::State, b"root", &root)?;
        
        // Flush database
        self.db.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::MemDb;
    
    #[test]
    fn test_nibble_conversion() {
        let bytes = vec![0xAB, 0xCD];
        let nibbles = bytes_to_nibbles(&bytes);
        assert_eq!(nibbles, vec![0xA, 0xB, 0xC, 0xD]);
        
        let back = nibbles_to_bytes(&nibbles);
        assert_eq!(back, bytes);
    }
    
    #[test]
    fn test_trie_insert_get() {
        let db = MemDb::new();
        let mut trie = PatriciaTrie::new(Arc::new(db)).unwrap();
        
        let key = b"hello";
        let value = b"world";
        
        trie.insert(key, value).unwrap();
        let retrieved = trie.get(key).unwrap();
        
        assert_eq!(retrieved, Some(value.to_vec()));
    }
    
    #[test]
    fn test_trie_multiple_inserts() {
        let db = MemDb::new();
        let mut trie = PatriciaTrie::new(Arc::new(db)).unwrap();
        
        trie.insert(b"key1", b"value1").unwrap();
        trie.insert(b"key2", b"value2").unwrap();
        trie.insert(b"key3", b"value3").unwrap();
        
        assert_eq!(trie.get(b"key1").unwrap(), Some(b"value1".to_vec()));
        assert_eq!(trie.get(b"key2").unwrap(), Some(b"value2".to_vec()));
        assert_eq!(trie.get(b"key3").unwrap(), Some(b"value3".to_vec()));
        assert_eq!(trie.get(b"key4").unwrap(), None);
    }
    
    #[test]
    fn test_trie_delete() {
        let db = MemDb::new();
        let mut trie = PatriciaTrie::new(Arc::new(db)).unwrap();
        
        trie.insert(b"key1", b"value1").unwrap();
        trie.insert(b"key2", b"value2").unwrap();
        
        trie.delete(b"key1").unwrap();
        
        assert_eq!(trie.get(b"key1").unwrap(), None);
        assert_eq!(trie.get(b"key2").unwrap(), Some(b"value2".to_vec()));
    }
    
    #[test]
    fn test_root_hash_consistency() {
        let db1 = MemDb::new();
        let db2 = MemDb::new();
        
        let mut trie1 = PatriciaTrie::new(Arc::new(db1)).unwrap();
        let mut trie2 = PatriciaTrie::new(Arc::new(db2)).unwrap();
        
        trie1.insert(b"key1", b"value1").unwrap();
        trie1.insert(b"key2", b"value2").unwrap();
        
        trie2.insert(b"key2", b"value2").unwrap();
        trie2.insert(b"key1", b"value1").unwrap();
        
        assert_eq!(trie1.root_hash(), trie2.root_hash());
    }
    
    #[test]
    fn test_merkle_proof() {
        let db = MemDb::new();
        let mut trie = PatriciaTrie::new(Arc::new(db)).unwrap();
        
        let key = b"test_key";
        let value = b"test_value";
        
        trie.insert(key, value).unwrap();
        let root = trie.root_hash();
        
        let proof = trie.get_proof(key).unwrap();
        assert!(!proof.is_empty());
        
        let valid = PatriciaTrie::verify_proof(root, key, value, &proof).unwrap();
        assert!(valid);
    }
    
    #[test]
    fn test_persistence_across_restart() {
        let db = MemDb::new();
        
        // First session: insert data
        {
            let mut trie = PatriciaTrie::new(Arc::new(db.clone())).unwrap();
            trie.insert(b"persistent", b"data").unwrap();
            trie.flush().unwrap();
        }
        
        // Second session: reload and verify
        {
            let trie = PatriciaTrie::new(Arc::new(db)).unwrap();
            let value = trie.get(b"persistent").unwrap();
            assert_eq!(value, Some(b"data".to_vec()));
        }
    }
}