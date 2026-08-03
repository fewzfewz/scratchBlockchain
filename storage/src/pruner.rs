//! Block and receipt pruning for `ChainStore`.
//!
//! Modes (via `PruneConfig::from_mode`):
//! - `archive` — no pruning
//! - `full` — keep last 10_000 blocks (default)
//! - `minimal` — keep last 256 blocks

use crate::db::{ChainStore, ColumnFamily, WriteBatch};
use common::types::Block;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tracing::{debug, info};

const META_PRUNED_HEIGHT: &[u8] = b"pruned_height";

/// Pruning configuration derived from node storage settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneConfig {
    pub mode: String,
    pub blocks_to_keep: u64,
    pub prune_every_n_blocks: u64,
    pub min_blocks_below_finalized: u64,
}

impl PruneConfig {
    pub fn archive() -> Self {
        Self {
            mode: "archive".into(),
            blocks_to_keep: u64::MAX,
            prune_every_n_blocks: u64::MAX,
            min_blocks_below_finalized: 0,
        }
    }

    pub fn from_mode(mode: &str, blocks_to_keep: Option<u64>, prune_every: Option<u64>) -> Self {
        match mode {
            "archive" => Self::archive(),
            "minimal" => Self {
                mode: "minimal".into(),
                blocks_to_keep: blocks_to_keep.unwrap_or(256),
                prune_every_n_blocks: prune_every.unwrap_or(50),
                min_blocks_below_finalized: 32,
            },
            _ => Self {
                mode: "full".into(),
                blocks_to_keep: blocks_to_keep.unwrap_or(10_000),
                prune_every_n_blocks: prune_every.unwrap_or(100),
                min_blocks_below_finalized: 128,
            },
        }
    }

    pub fn enabled(&self) -> bool {
        self.mode != "archive"
    }

    pub fn should_run_at(&self, finalized_height: u64) -> bool {
        if !self.enabled() || finalized_height == 0 {
            return false;
        }
        finalized_height % self.prune_every_n_blocks == 0
    }

    pub fn cutoff_height(&self, finalized_height: u64) -> u64 {
        finalized_height.saturating_sub(self.blocks_to_keep + self.min_blocks_below_finalized)
    }
}

#[derive(Debug, Clone, Default)]
pub struct PruneStats {
    pub blocks_pruned: u64,
    pub receipts_pruned: u64,
    pub from_height: u64,
    pub to_height: u64,
}

impl ChainStore {
    pub fn get_pruned_height(&self) -> Result<u64, Box<dyn Error>> {
        Ok(self.get_meta_u64(META_PRUNED_HEIGHT)?.unwrap_or(0))
    }

    pub fn get_meta_u64(&self, key: &[u8]) -> Result<Option<u64>, Box<dyn Error>> {
        match self.inner().get(ColumnFamily::Meta, key)? {
            Some(raw) if raw.len() >= 8 => {
                let arr: [u8; 8] = raw[..8].try_into().unwrap_or([0u8; 8]);
                Ok(Some(u64::from_le_bytes(arr)))
            }
            _ => Ok(None),
        }
    }

    pub fn set_meta_u64(&self, key: &[u8], value: u64) -> Result<(), Box<dyn Error>> {
        self.inner()
            .put(ColumnFamily::Meta, key, &value.to_le_bytes())
    }

    /// Delete blocks, height index entries, and receipts below `cutoff_height`.
    pub fn prune_blocks_below(&self, cutoff_height: u64) -> Result<PruneStats, Box<dyn Error>> {
        let mut stats = PruneStats::default();
        if cutoff_height == 0 {
            return Ok(stats);
        }

        let already = self.get_pruned_height()?;
        if cutoff_height <= already {
            return Ok(stats);
        }

        stats.from_height = already;
        stats.to_height = cutoff_height;

        let mut batch = WriteBatch::new();

        for height in already..cutoff_height {
            let Some(hash_bytes) = self.get_block_hash_by_height(height)? else {
                continue;
            };
            if hash_bytes.len() != 32 {
                continue;
            }
            let hash: [u8; 32] = hash_bytes.as_slice().try_into().unwrap();

            if let Some(encoded) = self.get_block(&hash)? {
                if let Ok(block) = serde_json::from_slice::<Block>(&encoded) {
                    for tx in &block.extrinsics {
                        let tx_hash = tx.hash();
                        batch.delete(ColumnFamily::Receipts, tx_hash.as_slice());
                        stats.receipts_pruned += 1;
                    }
                }
            }

            batch.delete(ColumnFamily::Blocks, hash.as_slice());
            batch.delete(ColumnFamily::BlockHeights, &height.to_le_bytes());
            stats.blocks_pruned += 1;
        }

        batch.put(
            ColumnFamily::Meta,
            META_PRUNED_HEIGHT,
            cutoff_height.to_le_bytes().to_vec(),
        );
        self.inner().write_batch(batch)?;

        if stats.blocks_pruned > 0 {
            info!(
                "Pruned {} blocks and {} receipts (heights {}..{})",
                stats.blocks_pruned, stats.receipts_pruned, already, cutoff_height
            );
        } else {
            debug!(
                "Prune cycle complete — nothing to delete below height {}",
                cutoff_height
            );
        }

        Ok(stats)
    }

    /// Run pruning if the config says we should at this finalized height.
    pub fn maybe_prune(
        &self,
        config: &PruneConfig,
        finalized_height: u64,
    ) -> Result<PruneStats, Box<dyn Error>> {
        if !config.should_run_at(finalized_height) {
            return Ok(PruneStats::default());
        }
        let cutoff = config.cutoff_height(finalized_height);
        self.prune_blocks_below(cutoff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemDb;
    use common::types::{Block, Header};
    use std::sync::Arc;

    fn seed_blocks(store: &ChainStore, count: u64) {
        for height in 0..count {
            let block = Block::new(Header::new([height as u8; 32], height), vec![]);
            let hash = block.hash();
            let encoded = serde_json::to_vec(&block).unwrap();
            store
                .commit_block(height, &hash, &encoded, vec![], vec![])
                .unwrap();
        }
    }

    #[test]
    fn test_prune_old_blocks() {
        let inner: Arc<dyn crate::db::KeyValueStore> = Arc::new(MemDb::new());
        let store = ChainStore::new(inner);
        seed_blocks(&store, 500);

        let cfg = PruneConfig::from_mode("minimal", Some(50), Some(1));
        let stats = store.maybe_prune(&cfg, 400).unwrap();
        assert!(stats.blocks_pruned > 0);

        assert!(store.get_block_by_height(0).unwrap().is_none());
        assert!(store.get_block_by_height(399).unwrap().is_some());
    }

    #[test]
    fn test_archive_mode_skips_pruning() {
        let inner: Arc<dyn crate::db::KeyValueStore> = Arc::new(MemDb::new());
        let store = ChainStore::new(inner);
        seed_blocks(&store, 20);

        let cfg = PruneConfig::archive();
        let stats = store.maybe_prune(&cfg, 19).unwrap();
        assert_eq!(stats.blocks_pruned, 0);
        assert!(store.get_block_by_height(0).unwrap().is_some());
    }
}
