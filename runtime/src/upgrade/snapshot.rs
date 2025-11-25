use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;

/// State snapshot for rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub id: u64,
    pub version: crate::upgrade::version::RuntimeVersion,
    pub block_number: u64,
    pub state_root: [u8; 32],
    pub timestamp: u64,
    pub compressed_state: Vec<u8>,
}

/// Snapshot manager for creating and restoring state snapshots
pub struct SnapshotManager {
    snapshots: HashMap<u64, StateSnapshot>,
    next_id: u64,
    max_snapshots: usize,
}

impl SnapshotManager {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: HashMap::new(),
            next_id: 1,
            max_snapshots,
        }
    }

    /// Create a snapshot of the current state
    pub fn create_snapshot(
        &mut self,
        version: crate::upgrade::version::RuntimeVersion,
        block_number: u64,
        state_data: &[u8],
    ) -> Result<u64, SnapshotError> {
        let id = self.next_id;
        self.next_id += 1;

        // Compute state root
        let mut hasher = Sha256::new();
        hasher.update(state_data);
        let state_root: [u8; 32] = hasher.finalize().into();

        // Compress state (simplified - in production use zstd or similar)
        let compressed_state = self.compress_state(state_data)?;

        let snapshot = StateSnapshot {
            id,
            version,
            block_number,
            state_root,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            compressed_state,
        };

        self.snapshots.insert(id, snapshot);

        // Prune old snapshots if needed
        if self.snapshots.len() > self.max_snapshots {
            self.prune_oldest();
        }

        Ok(id)
    }

    /// Get a snapshot by ID
    pub fn get_snapshot(&self, id: u64) -> Option<&StateSnapshot> {
        self.snapshots.get(&id)
    }

    /// Get the latest snapshot
    pub fn get_latest(&self) -> Option<&StateSnapshot> {
        self.snapshots.values()
            .max_by_key(|s| s.id)
    }

    /// Restore state from a snapshot
    pub fn restore_snapshot(&self, id: u64) -> Result<Vec<u8>, SnapshotError> {
        let snapshot = self.snapshots.get(&id)
            .ok_or(SnapshotError::SnapshotNotFound)?;

        // Decompress state
        let state_data = self.decompress_state(&snapshot.compressed_state)?;

        // Verify state root
        let mut hasher = Sha256::new();
        hasher.update(&state_data);
        let computed_root: [u8; 32] = hasher.finalize().into();

        if computed_root != snapshot.state_root {
            return Err(SnapshotError::StateRootMismatch);
        }

        Ok(state_data)
    }

    /// Prune the oldest snapshot
    fn prune_oldest(&mut self) {
        if let Some(oldest_id) = self.snapshots.keys()
            .min()
            .copied()
        {
            self.snapshots.remove(&oldest_id);
        }
    }

    /// Compress state data (simplified)
    fn compress_state(&self, data: &[u8]) -> Result<Vec<u8>, SnapshotError> {
        // In production, use zstd or similar
        Ok(data.to_vec())
    }

    /// Decompress state data (simplified)
    fn decompress_state(&self, data: &[u8]) -> Result<Vec<u8>, SnapshotError> {
        // In production, use zstd or similar
        Ok(data.to_vec())
    }

    /// Get all snapshot IDs
    pub fn list_snapshots(&self) -> Vec<u64> {
        let mut ids: Vec<u64> = self.snapshots.keys().copied().collect();
        ids.sort();
        ids
    }

    /// Delete a snapshot
    pub fn delete_snapshot(&mut self, id: u64) -> Result<(), SnapshotError> {
        self.snapshots.remove(&id)
            .ok_or(SnapshotError::SnapshotNotFound)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("Snapshot not found")]
    SnapshotNotFound,
    
    #[error("State root mismatch")]
    StateRootMismatch,
    
    #[error("Compression failed")]
    CompressionFailed,
    
    #[error("Decompression failed")]
    DecompressionFailed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upgrade::version::RuntimeVersion;

    #[test]
    fn test_snapshot_creation() {
        let mut manager = SnapshotManager::new(5);
        let version = RuntimeVersion::new(1, 0, 0);
        let state_data = b"test state data";

        let id = manager.create_snapshot(version, 100, state_data).unwrap();
        assert_eq!(id, 1);

        let snapshot = manager.get_snapshot(id).unwrap();
        assert_eq!(snapshot.block_number, 100);
    }

    #[test]
    fn test_snapshot_restore() {
        let mut manager = SnapshotManager::new(5);
        let version = RuntimeVersion::new(1, 0, 0);
        let state_data = b"test state data";

        let id = manager.create_snapshot(version, 100, state_data).unwrap();
        let restored = manager.restore_snapshot(id).unwrap();

        assert_eq!(restored, state_data);
    }

    #[test]
    fn test_snapshot_pruning() {
        let mut manager = SnapshotManager::new(3);
        let version = RuntimeVersion::new(1, 0, 0);

        // Create 5 snapshots (max is 3)
        for i in 0..5 {
            manager.create_snapshot(version.clone(), i, b"data").unwrap();
        }

        // Should only have 3 snapshots
        assert_eq!(manager.list_snapshots().len(), 3);
    }
}
