use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::error::Error;

/// Nibble (4-bit value) used for trie paths
type Nibble = u8;

/// Path in the trie as a sequence of nibbles
type NibblePath = Vec<Nibble>;

/// Hash type for node references
pub type NodeHash = [u8; 32];

/// Convert bytes to nibble path
fn bytes_to_nibbles(bytes: &[u8]) -> NibblePath {
    let mut nibbles = Vec::with_capacity(bytes.len() * 2);
    for byte in bytes {
        nibbles.push(byte >> 4);
        nibbles.push(byte & 0x0F);
    }
    nibbles
}

/// Convert nibble path back to bytes (assumes even length)
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
fn common_prefix_len(a: &[Nibble], b: &[Nibble]) -> usize {
    a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count()
}

/// Trie node types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrieNode {
    /// Empty node
    Empty,
    
    /// Leaf node: (key_suffix, value)
    Leaf {
        path: NibblePath,
        value: Vec<u8>,
    },
    
    /// Extension node: (shared_path, child_hash)
    Extension {
        path: NibblePath,
        child: NodeHash,
    },
    
    /// Branch node: 16 children + optional value
    Branch {
        children: [Option<NodeHash>; 16],
        value: Option<Vec<u8>>,
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
    fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
    
    /// Decode node from bytes
    fn decode(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        Ok(serde_json::from_slice(bytes)?)
    }
}

/// Patricia Merkle Trie
pub struct PatriciaTrie {
    /// Root node hash
    root: NodeHash,
    
    /// Node storage (hash -> node)
    nodes: HashMap<NodeHash, TrieNode>,
    
    /// Persistent storage backend
    db: sled::Db,
}

impl PatriciaTrie {
    /// Create a new trie with the given database path
    pub fn new(db_path: &str) -> Result<Self, Box<dyn Error>> {
        let db = sled::open(db_path)?;
        
        // Empty trie has empty root
        let empty_node = TrieNode::Empty;
        let root = empty_node.hash();
        
        let mut nodes = HashMap::new();
        nodes.insert(root, empty_node);
        
        Ok(Self { root, nodes, db })
    }
    
    /// Get the current root hash
    pub fn root_hash(&self) -> NodeHash {
        self.root
    }
    
    /// Insert a key-value pair
    pub fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), Box<dyn Error>> {
        let path = bytes_to_nibbles(key);
        let new_root = self.insert_at(self.root, &path, value.to_vec())?;
        self.root = new_root;
        self.persist_node(new_root)?;
        Ok(())
    }
    
    /// Get a value by key
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        let path = bytes_to_nibbles(key);
        self.get_at(self.root, &path)
    }
    
    /// Delete a key
    pub fn delete(&mut self, key: &[u8]) -> Result<(), Box<dyn Error>> {
        let path = bytes_to_nibbles(key);
        let new_root = self.delete_at(self.root, &path)?;
        self.root = new_root;
        self.persist_node(new_root)?;
        Ok(())
    }
    
    /// Generate a Merkle proof for a key
    pub fn get_proof(&self, key: &[u8]) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        let path = bytes_to_nibbles(key);
        let mut proof = Vec::new();
        self.collect_proof(self.root, &path, &mut proof)?;
        Ok(proof)
    }
    
    // Internal methods
    
    fn get_node(&self, hash: NodeHash) -> Result<TrieNode, Box<dyn Error>> {
        // Check in-memory cache first
        if let Some(node) = self.nodes.get(&hash) {
            return Ok(node.clone());
        }
        
        // Load from persistent storage
        if let Some(data) = self.db.get(&hash)? {
            let node = TrieNode::decode(&data)?;
            return Ok(node);
        }
        
        Err("Node not found".into())
    }
    
    fn persist_node(&mut self, hash: NodeHash) -> Result<(), Box<dyn Error>> {
        if let Some(node) = self.nodes.get(&hash) {
            let encoded = node.encode();
            self.db.insert(&hash, encoded.as_slice())?;
            self.db.flush()?;
        }
        Ok(())
    }
    
    fn insert_at(&mut self, node_hash: NodeHash, path: &[Nibble], value: Vec<u8>) -> Result<NodeHash, Box<dyn Error>> {
        let node = self.get_node(node_hash)?;
        
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
                        self.nodes.insert(old_hash, old_leaf);
                        children[leaf_path[0] as usize] = Some(old_hash);
                        
                        // Insert new leaf
                        let new_leaf = TrieNode::Leaf {
                            path: path[1..].to_vec(),
                            value,
                        };
                        let new_hash = new_leaf.hash();
                        self.nodes.insert(new_hash, new_leaf);
                        children[path[0] as usize] = Some(new_hash);
                        
                        TrieNode::Branch {
                            children,
                            value: None,
                        }
                    } else {
                        // Create extension + branch
                        let mut children = [None; 16];
                        
                        // Old leaf
                        if common_len < leaf_path.len() {
                            let old_leaf = TrieNode::Leaf {
                                path: leaf_path[common_len + 1..].to_vec(),
                                value: leaf_value,
                            };
                            let old_hash = old_leaf.hash();
                            self.nodes.insert(old_hash, old_leaf);
                            children[leaf_path[common_len] as usize] = Some(old_hash);
                        }
                        
                        // New leaf
                        if common_len < path.len() {
                            let new_leaf = TrieNode::Leaf {
                                path: path[common_len + 1..].to_vec(),
                                value,
                            };
                            let new_hash = new_leaf.hash();
                            self.nodes.insert(new_hash, new_leaf);
                            children[path[common_len] as usize] = Some(new_hash);
                        }
                        
                        let branch = TrieNode::Branch {
                            children,
                            value: None,
                        };
                        let branch_hash = branch.hash();
                        self.nodes.insert(branch_hash, branch);
                        
                        TrieNode::Extension {
                            path: path[..common_len].to_vec(),
                            child: branch_hash,
                        }
                    }
                }
            }
            
            TrieNode::Extension { path: ext_path, child } => {
                let common_len = common_prefix_len(path, &ext_path);
                
                if common_len == ext_path.len() {
                    // Continue down the extension
                    let new_child = self.insert_at(child, &path[common_len..], value)?;
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
                    self.nodes.insert(old_hash, old_ext);
                    children[ext_path[common_len] as usize] = Some(old_hash);
                    
                    // New path
                    let new_leaf = TrieNode::Leaf {
                        path: path[common_len + 1..].to_vec(),
                        value,
                    };
                    let new_hash = new_leaf.hash();
                    self.nodes.insert(new_hash, new_leaf);
                    children[path[common_len] as usize] = Some(new_hash);
                    
                    let branch = TrieNode::Branch {
                        children,
                        value: None,
                    };
                    let branch_hash = branch.hash();
                    self.nodes.insert(branch_hash, branch.clone());
                    
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
                    let new_child = self.insert_at(child_hash, &path[1..], value)?;
                    children[idx] = Some(new_child);
                    
                    TrieNode::Branch {
                        children,
                        value: branch_value,
                    }
                }
            }
        };
        
        let new_hash = new_node.hash();
        self.nodes.insert(new_hash, new_node);
        Ok(new_hash)
    }
    
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
    
    fn delete_at(&mut self, node_hash: NodeHash, path: &[Nibble]) -> Result<NodeHash, Box<dyn Error>> {
        let node = self.get_node(node_hash)?;
        
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
                    let new_child = self.delete_at(child, &path[ext_path.len()..])?;
                    TrieNode::Extension {
                        path: ext_path.clone(),
                        child: new_child,
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
                        let new_child = self.delete_at(child_hash, &path[1..])?;
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
        self.nodes.insert(new_hash, new_node);
        Ok(new_hash)
    }
    
    fn collect_proof(&self, node_hash: NodeHash, path: &[Nibble], proof: &mut Vec<Vec<u8>>) -> Result<(), Box<dyn Error>> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
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
        let dir = tempdir().unwrap();
        let mut trie = PatriciaTrie::new(dir.path().to_str().unwrap()).unwrap();
        
        let key = b"hello";
        let value = b"world";
        
        trie.insert(key, value).unwrap();
        let retrieved = trie.get(key).unwrap();
        
        assert_eq!(retrieved, Some(value.to_vec()));
    }
    
    #[test]
    fn test_trie_multiple_inserts() {
        let dir = tempdir().unwrap();
        let mut trie = PatriciaTrie::new(dir.path().to_str().unwrap()).unwrap();
        
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
        let dir = tempdir().unwrap();
        let mut trie = PatriciaTrie::new(dir.path().to_str().unwrap()).unwrap();
        
        trie.insert(b"key1", b"value1").unwrap();
        trie.insert(b"key2", b"value2").unwrap();
        
        trie.delete(b"key1").unwrap();
        
        assert_eq!(trie.get(b"key1").unwrap(), None);
        assert_eq!(trie.get(b"key2").unwrap(), Some(b"value2".to_vec()));
    }
    
    #[test]
    fn test_root_hash_consistency() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();
        
        let mut trie1 = PatriciaTrie::new(dir1.path().to_str().unwrap()).unwrap();
        let mut trie2 = PatriciaTrie::new(dir2.path().to_str().unwrap()).unwrap();
        
        trie1.insert(b"key1", b"value1").unwrap();
        trie1.insert(b"key2", b"value2").unwrap();
        
        trie2.insert(b"key2", b"value2").unwrap();
        trie2.insert(b"key1", b"value1").unwrap();
        
        assert_eq!(trie1.root_hash(), trie2.root_hash());
    }
}
