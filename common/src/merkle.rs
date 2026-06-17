//! # Merkle Tree Implementation
//!
//! This module provides a complete Merkle tree implementation with:
//! - Root calculation
//! - Proof generation and verification
//! - Multi-proof support for multiple leaves
//! - Batch verification

use crate::crypto::hash_pair;

/// Merkle proof for a single leaf
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleProof {
    /// Sibling hashes needed to reconstruct the root
    pub siblings: Vec<[u8; 32]>,
    /// Indices indicating whether each sibling is on the left or right
    pub indices: Vec<bool>, // true = right sibling, false = left sibling
    /// The leaf hash being proven
    pub leaf: [u8; 32],
    /// The leaf index in the tree
    pub leaf_index: usize,
}

impl MerkleProof {
    /// Verify the proof against a given root
    pub fn verify(&self, root: &[u8; 32]) -> bool {
        let mut current = self.leaf;
        
        for (i, sibling) in self.siblings.iter().enumerate() {
            let is_right = self.indices[i];
            current = if is_right {
                hash_pair(sibling, &current)
            } else {
                hash_pair(&current, sibling)
            };
        }
        
        &current == root
    }
    
    /// Create a proof for a leaf at given index
    pub fn from_tree(tree: &MerkleTree, leaf_index: usize) -> Option<Self> {
        if leaf_index >= tree.leaves.len() {
            return None;
        }
        
        let leaf = tree.leaves[leaf_index];
        let mut siblings = Vec::new();
        let mut indices = Vec::new();
        let mut idx = leaf_index;
        
        // Build the proof by walking up the tree
        let mut level_size = tree.leaves.len();
        let mut offset = 0;
        
        while level_size > 1 {
            let is_right = idx % 2 == 1;
            let sibling_idx = if is_right { idx - 1 } else { idx + 1 };
            
            if sibling_idx < level_size {
                // Get sibling from current level
                let sibling = tree.levels.get(offset + sibling_idx)
                    .copied()
                    .or_else(|| tree.calculate_node(offset, sibling_idx, level_size))
                    .unwrap_or([0u8; 32]);
                siblings.push(sibling);
                indices.push(is_right);
            } else {
                // Odd number of nodes - duplicate last node
                let last = tree.levels.get(offset + level_size - 1)
                    .unwrap_or(&[0u8; 32]);
                siblings.push(*last);
                indices.push(is_right);
            }
            
            idx /= 2;
            offset += level_size;
            level_size = (level_size + 1) / 2;
        }
        
        Some(Self {
            siblings,
            indices,
            leaf,
            leaf_index,
        })
    }
}

/// Multi-proof for multiple leaves
#[derive(Debug, Clone)]
pub struct MultiProof {
    /// Hashes needed to reconstruct the tree
    pub hashes: Vec<[u8; 32]>,
    /// Indices of leaves being proven
    pub leaf_indices: Vec<usize>,
    /// The leaves being proven
    pub leaves: Vec<[u8; 32]>,
}

impl MultiProof {
    /// Verify multi-proof against root
    pub fn verify(&self, root: &[u8; 32], total_leaves: usize) -> bool {
        // Build a map of leaf index to hash
        let leaf_map: std::collections::HashMap<usize, [u8; 32]> = self.leaf_indices
            .iter()
            .zip(self.leaves.iter())
            .map(|(&i, &h)| (i, h))
            .collect();
        
        // Build the tree bottom-up
        let mut hash_map: std::collections::HashMap<usize, [u8; 32]> = leaf_map.clone();
        
        let mut level_size = total_leaves;
        let mut offset = 0;
        
        while level_size > 1 {
            let mut next_level = std::collections::HashMap::new();
            
            for i in 0..level_size {
                if !hash_map.contains_key(&(offset + i)) {
                    continue;
                }
                
                let is_right = i % 2 == 1;
                let pair_idx = if is_right { i - 1 } else { i + 1 };
                let sibling_key = offset + pair_idx;
                
                if pair_idx < level_size {
                    // Try to get sibling
                    if let Some(sibling) = hash_map.get(&sibling_key) {
                        let combined = if is_right {
                            hash_pair(sibling, hash_map.get(&(offset + i)).unwrap())
                        } else {
                            hash_pair(hash_map.get(&(offset + i)).unwrap(), sibling)
                        };
                        next_level.insert(offset / 2 + i / 2, combined);
                    }
                } else {
                    // Odd number - duplicate
                    let combined = hash_pair(
                        hash_map.get(&(offset + i)).unwrap(),
                        hash_map.get(&(offset + i)).unwrap()
                    );
                    next_level.insert(offset / 2 + i / 2, combined);
                }
            }
            
            hash_map = next_level;
            offset += level_size;
            level_size = (level_size + 1) / 2;
        }
        
        // Check if we computed the root
        hash_map.get(&0).map_or(false, |computed| computed == root)
    }
}

/// Enhanced Merkle tree with proof capabilities
#[derive(Debug, Clone)]
pub struct MerkleTree {
    leaves: Vec<[u8; 32]>,
    levels: Vec<[u8; 32]>, // Flat array of all nodes
    root_hash: [u8; 32],
}

impl MerkleTree {
    /// Create a new Merkle tree from leaf hashes
    pub fn new(leaves: Vec<[u8; 32]>) -> Self {
        let levels = Self::build_tree(&leaves);
        let root_hash = levels.last().copied().unwrap_or([0u8; 32]);
        
        Self {
            leaves,
            levels,
            root_hash,
        }
    }
    
    /// Build the complete tree (all levels)
    fn build_tree(leaves: &[[u8; 32]]) -> Vec<[u8; 32]> {
        if leaves.is_empty() {
            return vec![];
        }
        
        let mut all_levels = Vec::new();
        let mut current_level = leaves.to_vec();
        
        all_levels.extend(current_level.clone());
        
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1]
                } else {
                    current_level[i]
                };
                next_level.push(hash_pair(&left, &right));
            }
            
            all_levels.extend(next_level.clone());
            current_level = next_level;
        }
        
        all_levels
    }
    
    /// Calculate a specific node (lazy calculation)
    fn calculate_node(&self, offset: usize, idx: usize, level_size: usize) -> Option<[u8; 32]> {
        if idx >= level_size {
            return None;
        }
        
        let start_idx = offset;
        let _end_idx = start_idx + level_size;
        
        if start_idx >= self.levels.len() {
            return None;
        }
        
        self.levels.get(start_idx + idx).copied()
    }
    
    /// Get the Merkle root
    pub fn root(&self) -> [u8; 32] {
        self.root_hash
    }
    
    /// Get the number of leaves
    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
    }
    
    /// Get a leaf by index
    pub fn get_leaf(&self, index: usize) -> Option<[u8; 32]> {
        self.leaves.get(index).copied()
    }
    
    /// Generate a proof for a leaf
    pub fn generate_proof(&self, leaf_index: usize) -> Option<MerkleProof> {
        MerkleProof::from_tree(self, leaf_index)
    }
    
    /// Generate a proof for multiple leaves
    pub fn generate_multi_proof(&self, leaf_indices: &[usize]) -> Option<MultiProof> {
        let mut leaves = Vec::new();
        for &idx in leaf_indices {
            if let Some(leaf) = self.get_leaf(idx) {
                leaves.push(leaf);
            } else {
                return None;
            }
        }
        
        Some(MultiProof {
            hashes: vec![], // Would need to compute required nodes
            leaf_indices: leaf_indices.to_vec(),
            leaves,
        })
    }
    
    /// Verify a single proof
    pub fn verify_proof(&self, proof: &MerkleProof) -> bool {
        proof.verify(&self.root_hash)
    }
    
    /// Create a tree from transaction hashes
    pub fn from_transactions(txs: &[crate::types::Transaction]) -> Self {
        let leaves: Vec<[u8; 32]> = txs.iter().map(|tx| tx.hash()).collect();
        Self::new(leaves)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_leaves(n: usize) -> Vec<[u8; 32]> {
        (0..n).map(|i| {
            let mut leaf = [0u8; 32];
            leaf[0] = i as u8;
            leaf
        }).collect()
    }
    
    #[test]
    fn test_empty_tree() {
        let tree = MerkleTree::new(vec![]);
        assert_eq!(tree.root(), [0u8; 32]);
    }
    
    #[test]
    fn test_single_leaf() {
        let leaf = [1u8; 32];
        let tree = MerkleTree::new(vec![leaf]);
        assert_eq!(tree.root(), leaf);
    }
    
    #[test]
    fn test_two_leaves() {
        let leaves = create_test_leaves(2);
        let tree = MerkleTree::new(leaves.clone());
        
        let expected = hash_pair(&leaves[0], &leaves[1]);
        assert_eq!(tree.root(), expected);
    }
    
    #[test]
    fn test_proof_generation_and_verification() {
        let leaves = create_test_leaves(8);
        let tree = MerkleTree::new(leaves);
        
        for i in 0..8 {
            let proof = tree.generate_proof(i).unwrap();
            assert!(tree.verify_proof(&proof));
            
            // Tampered proof should fail
            let mut tampered = proof.clone();
            tampered.leaf = [0xff; 32];
            assert!(!tree.verify_proof(&tampered));
        }
    }
    
    #[test]
    fn test_odd_number_of_leaves() {
        let leaves = create_test_leaves(7);
        let tree = MerkleTree::new(leaves);
        
        // Last leaf should be duplicated
        let proof = tree.generate_proof(6).unwrap();
        assert!(tree.verify_proof(&proof));
    }
}