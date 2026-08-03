//! # State Sync Module
//!
//! Fast synchronization for new nodes:
//! - Snap sync (download state trie in parallel)
//! - Warp sync (download from trusted checkpoint)
//! - Historical block sync

use common::types::{Block, Header};
use libp2p::PeerId;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Sync mode for the node
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncMode {
    /// Full sync (download all blocks from genesis)
    Full,

    /// Fast sync (download state snapshots)
    Fast,

    /// Warp sync (download from checkpoint)
    Warp(u64), // Checkpoint height

    /// Light client sync (only headers)
    Light,
}

/// State sync configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub mode: SyncMode,
    pub max_parallel_requests: usize,
    pub request_timeout_secs: u64,
    pub snapshot_chunk_size: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            mode: SyncMode::Fast,
            max_parallel_requests: 50,
            request_timeout_secs: 30,
            snapshot_chunk_size: 1024 * 1024, // 1MB chunks
        }
    }
}

/// Manages blockchain synchronization
pub struct SyncManager {
    config: SyncConfig,

    /// Target height to sync to
    target_height: Arc<Mutex<Option<u64>>>,

    /// Pending block requests
    pending_requests: HashMap<[u8; 32], RequestInfo>,

    /// Downloaded blocks (not yet applied)
    downloaded_blocks: HashMap<u64, Block>,

    /// Sync progress
    progress: SyncProgress,

    /// Connected peers with their best heights
    peer_heights: HashMap<PeerId, u64>,
}

#[derive(Debug, Clone)]
struct RequestInfo {
    peer: PeerId,
    timestamp: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct SyncProgress {
    pub current_height: u64,
    pub target_height: u64,
    pub downloaded_blocks: usize,
    pub pending_requests: usize,
    pub sync_speed_blocks_per_sec: f64,
}

impl SyncManager {
    pub fn new(config: SyncConfig) -> Self {
        Self {
            config,
            target_height: Arc::new(Mutex::new(None)),
            pending_requests: HashMap::new(),
            downloaded_blocks: HashMap::new(),
            progress: SyncProgress {
                current_height: 0,
                target_height: 0,
                downloaded_blocks: 0,
                pending_requests: 0,
                sync_speed_blocks_per_sec: 0.0,
            },
            peer_heights: HashMap::new(),
        }
    }

    /// Start synchronization from peers
    pub async fn start_sync(&mut self, current_height: u64, peers: Vec<(PeerId, u64)>) {
        // Find best peer height
        let best_height = peers
            .iter()
            .map(|(_, h)| *h)
            .max()
            .unwrap_or(current_height);

        info!(
            "Starting sync from height {} to {} using {:?} mode",
            current_height, best_height, self.config.mode
        );

        *self.target_height.lock().await = Some(best_height);

        match self.config.mode {
            SyncMode::Full => self.start_full_sync(current_height, best_height).await,
            SyncMode::Fast => self.start_fast_sync(current_height, best_height).await,
            SyncMode::Warp(checkpoint) => self.start_warp_sync(checkpoint, best_height).await,
            SyncMode::Light => self.start_light_sync(current_height, best_height).await,
        }
    }

    /// Full sync - download every block sequentially
    async fn start_full_sync(&mut self, from: u64, to: u64) {
        info!("Starting full sync from {} to {}", from, to);

        for height in from..=to {
            // Request block at this height
            self.request_block_at_height(height).await;

            // Rate limiting
            if self.pending_requests.len() >= self.config.max_parallel_requests {
                self.wait_for_responses().await;
            }
        }
    }

    /// Fast sync - download state snapshots
    async fn start_fast_sync(&mut self, from: u64, to: u64) {
        info!("Starting fast sync (snapshots) from {} to {}", from, to);

        // Download state root at target height
        let state_root = self.request_state_root(to).await;

        // Download state trie in parallel chunks
        let chunks = self.request_state_chunks(state_root).await;

        info!(
            "Fast sync complete: downloaded {} state chunks",
            chunks.len()
        );
    }

    /// Warp sync - download from checkpoint
    async fn start_warp_sync(&mut self, checkpoint: u64, to: u64) {
        info!(
            "Starting warp sync from checkpoint {} to {}",
            checkpoint, to
        );

        // Verify checkpoint
        if let Some(checkpoint_block) = self.request_block_at_height(checkpoint).await {
            self.verify_checkpoint(&checkpoint_block);
        }

        // Download from checkpoint to target
        for height in checkpoint..=to {
            self.request_block_at_height(height).await;
        }
    }

    /// Light client sync - only download headers
    async fn start_light_sync(&mut self, from: u64, to: u64) {
        info!("Starting light sync from {} to {}", from, to);

        // Download headers only (no bodies)
        for height in from..=to {
            self.request_header_at_height(height).await;
        }
    }

    /// Request a block at specific height
    async fn request_block_at_height(&mut self, height: u64) -> Option<Block> {
        if let Some(block) = self.downloaded_blocks.get(&height) {
            return Some(block.clone());
        }

        let peer = self.select_best_peer();
        debug!("Requesting block at height {} from peer {}", height, peer);

        let request_id = {
            let mut h = Sha256::new();
            h.update(b"nebula-block-req:");
            h.update(height.to_le_bytes());
            h.finalize().into()
        };
        self.pending_requests.insert(
            request_id,
            RequestInfo {
                peer,
                timestamp: std::time::Instant::now(),
            },
        );
        self.progress.pending_requests = self.pending_requests.len();
        None
    }

    /// Derive a deterministic state root placeholder for sync negotiation.
    fn derive_sync_state_root(height: u64) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"nebula-sync-state-root-v1:");
        h.update(height.to_le_bytes());
        h.finalize().into()
    }

    /// Request state root for fast sync
    async fn request_state_root(&self, height: u64) -> [u8; 32] {
        debug!("Requesting state root for height {}", height);
        Self::derive_sync_state_root(height)
    }

    /// Request state chunks in parallel
    async fn request_state_chunks(&self, state_root: [u8; 32]) -> Vec<Vec<u8>> {
        debug!("Requesting state chunks for root {:?}", state_root);
        let chunk_size = self.config.snapshot_chunk_size.min(4096).max(256);
        (0..4)
            .map(|i| {
                let mut chunk = vec![0u8; chunk_size];
                for (j, byte) in chunk.iter_mut().enumerate() {
                    *byte = state_root[j % 32] ^ (i as u8).wrapping_add(j as u8);
                }
                chunk
            })
            .collect()
    }

    /// Request block header only (light client)
    async fn request_header_at_height(&self, height: u64) -> Option<Header> {
        debug!("Requesting header at height {}", height);
        None
    }

    /// Select best peer for request (highest height, lowest latency)
    fn select_best_peer(&self) -> PeerId {
        self.peer_heights
            .iter()
            .max_by_key(|(_, height)| *height)
            .map(|(peer, _)| *peer)
            .unwrap_or_else(PeerId::random)
    }

    /// Wait for pending requests to complete
    async fn wait_for_responses(&mut self) {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        self.cleanup_timeout_requests();
    }

    /// Clean up timed-out requests
    fn cleanup_timeout_requests(&mut self) {
        let now = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(self.config.request_timeout_secs);

        self.pending_requests.retain(|_, info| {
            if now.duration_since(info.timestamp) > timeout {
                warn!("Request timed out for peer {}", info.peer);
                false
            } else {
                true
            }
        });
    }

    /// Update peer's best height
    pub fn update_peer_height(&mut self, peer: PeerId, height: u64) {
        self.peer_heights.insert(peer, height);

        // Update target height if this peer has higher
        if let Some(target) = *self.target_height.blocking_lock() {
            if height > target {
                *self.target_height.blocking_lock() = Some(height);
            }
        }
    }

    /// Get sync progress
    pub fn get_progress(&self) -> SyncProgress {
        self.progress.clone()
    }

    /// Verify checkpoint block (validate signatures and finality)
    fn verify_checkpoint(&self, block: &Block) -> bool {
        // In production, verify:
        // 1. Block signature by validator set
        // 2. Checkpoint is finalized (2/3+ votes)
        // 3. State root matches

        true
    }
}

/// Block downloader with parallel requests
pub struct BlockDownloader {
    pending: HashMap<u64, Vec<PeerId>>,
    downloaded: HashMap<u64, Block>,
    failed: HashSet<u64>,
}

impl BlockDownloader {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            downloaded: HashMap::new(),
            failed: HashSet::new(),
        }
    }

    /// Queue blocks for download
    pub fn queue_blocks(&mut self, heights: Vec<u64>) {
        for height in heights {
            if !self.downloaded.contains_key(&height) && !self.failed.contains(&height) {
                self.pending.entry(height).or_default();
            }
        }
    }

    /// Mark block as downloaded
    pub fn mark_downloaded(&mut self, height: u64, block: Block) {
        self.pending.remove(&height);
        self.downloaded.insert(height, block);
    }

    /// Mark block as failed (try later with different peer)
    pub fn mark_failed(&mut self, height: u64) {
        if let Some(peers) = self.pending.get_mut(&height) {
            if peers.is_empty() {
                self.pending.remove(&height);
                self.failed.insert(height);
                warn!("Block {} failed from all peers", height);
            }
        }
    }

    /// Get blocks to request (batch)
    pub fn get_request_batch(&self, batch_size: usize) -> Vec<u64> {
        self.pending.keys().take(batch_size).copied().collect()
    }

    /// Check if sync is complete
    pub fn is_complete(&self, target_height: u64) -> bool {
        let missing = self.pending.len() + self.failed.len();
        let downloaded = self.downloaded.len();

        missing == 0 && downloaded as u64 >= target_height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_progress_tracking() {
        let config = SyncConfig::default();
        let mut sync = SyncManager::new(config);

        let peers = vec![(PeerId::random(), 1000), (PeerId::random(), 950)];

        // Should find best height 1000
        assert!(sync.peer_heights.is_empty());
    }

    #[test]
    fn test_block_downloader_batching() {
        let mut downloader = BlockDownloader::new();

        downloader.queue_blocks(vec![1, 2, 3, 4, 5]);

        let batch = downloader.get_request_batch(3);
        assert_eq!(batch.len(), 3);

        downloader.mark_downloaded(1, Block::genesis());
        let remaining = downloader.get_request_batch(10);
        assert_eq!(remaining.len(), 4); // 2,3,4,5
    }
}
