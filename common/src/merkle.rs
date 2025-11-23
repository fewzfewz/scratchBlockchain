use sha2::{Digest, Sha256};

/// Simple Merkle tree implementation for state commitment
pub struct MerkleTree {
    leaves: Vec<[u8; 32]>,
}

impl MerkleTree {
    /// Create a new Merkle tree from leaf hashes
    pub fn new(leaves: Vec<[u8; 32]>) -> Self {
        Self { leaves }
    }

    /// Compute the Merkle root
    pub fn root(&self) -> [u8; 32] {
        if self.leaves.is_empty() {
            return [0u8; 32];
        }

        if self.leaves.len() == 1 {
            return self.leaves[0];
        }

        let mut current_level = self.leaves.clone();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1]
                } else {
                    // If odd number of nodes, duplicate the last one
                    current_level[i]
                };

                let combined = Self::hash_pair(&left, &right);
                next_level.push(combined);
            }

            current_level = next_level;
        }

        current_level[0]
    }

    /// Hash a pair of nodes
    fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let leaf1 = [1u8; 32];
        let leaf2 = [2u8; 32];
        let tree = MerkleTree::new(vec![leaf1, leaf2]);
        
        let expected = MerkleTree::hash_pair(&leaf1, &leaf2);
        assert_eq!(tree.root(), expected);
    }

    #[test]
    fn test_four_leaves() {
        let leaves = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];
        let tree = MerkleTree::new(leaves.clone());
        
        // Manually compute expected root
        let h01 = MerkleTree::hash_pair(&leaves[0], &leaves[1]);
        let h23 = MerkleTree::hash_pair(&leaves[2], &leaves[3]);
        let expected = MerkleTree::hash_pair(&h01, &h23);
        
        assert_eq!(tree.root(), expected);
    }

    #[test]
    fn test_odd_leaves() {
        let leaves = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let tree = MerkleTree::new(leaves.clone());
        
        // With odd number, last leaf is duplicated
        let h01 = MerkleTree::hash_pair(&leaves[0], &leaves[1]);
        let h22 = MerkleTree::hash_pair(&leaves[2], &leaves[2]);
        let expected = MerkleTree::hash_pair(&h01, &h22);
        
        assert_eq!(tree.root(), expected);
    }
}
