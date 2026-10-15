use common::types::Block;
use std::collections::HashMap;
use std::error::Error;
use storage::BlockStore;
use tracing::{info, warn};

/// Fork choice rule: longest finalized chain
pub struct ForkChoice {
    /// Track competing chain tips by their hash
    chain_tips: HashMap<[u8; 32], ChainTip>,
}

#[derive(Debug, Clone)]
struct ChainTip {
    block_hash: [u8; 32],
    height: u64,
    finalized_height: u64,
}

impl Default for ForkChoice {
    fn default() -> Self {
        Self::new()
    }
}

impl ForkChoice {
    pub fn new() -> Self {
        Self {
            chain_tips: HashMap::new(),
        }
    }

    /// Handle an incoming block and determine if we should switch chains
    pub fn handle_incoming_block(
        &mut self,
        block: &Block,
        block_store: &BlockStore,
    ) -> Result<ForkDecision, Box<dyn Error>> {
        let block_height = block.header.slot;

        // Get current finalized height from storage
        let _current_finalized = block_store.get_latest_finalized_height()?.unwrap_or(0);

        // Get our current tip
        let our_height = block_store.get_latest_height()?.unwrap_or(0);

        // Check if this block extends our current chain
        if block_height == our_height + 1 {
            // Normal case: block extends our chain
            info!("Block {} extends current chain", block_height);
            return Ok(ForkDecision::Accept);
        }

        // A higher block is never a reason to reorg: finalized blocks are
        // final, and a node that is behind catches up through sync, which
        // checks each block's commit certificate. (This used to accept any
        // higher chain, so one validator key could rewrite history.)
        if block_height > our_height {
            warn!(
                "Block {} is ahead of our tip {} — ignoring; sync will fetch certified blocks",
                block_height, our_height
            );
            return Ok(ForkDecision::Ignore);
        }

        // Block is old or duplicate
        if block_height <= our_height {
            info!(
                "Block {} is old or duplicate (our height: {})",
                block_height, our_height
            );
            return Ok(ForkDecision::Ignore);
        }

        Ok(ForkDecision::Ignore)
    }

    /// Select the best chain based on longest finalized chain rule
    pub fn select_best_chain(
        &self,
        _block_store: &BlockStore,
    ) -> Result<Option<[u8; 32]>, Box<dyn Error>> {
        let mut best_tip: Option<ChainTip> = None;

        for tip in self.chain_tips.values() {
            if let Some(ref current_best) = best_tip {
                // Prefer higher finalized height
                if tip.finalized_height > current_best.finalized_height {
                    best_tip = Some(tip.clone());
                } else if tip.finalized_height == current_best.finalized_height {
                    // If finalized heights are equal, prefer longer chain
                    if tip.height > current_best.height {
                        best_tip = Some(tip.clone());
                    }
                }
            } else {
                best_tip = Some(tip.clone());
            }
        }

        Ok(best_tip.map(|tip| tip.block_hash))
    }

    /// Register a new chain tip
    pub fn register_tip(&mut self, block_hash: [u8; 32], height: u64, finalized_height: u64) {
        self.chain_tips.insert(
            block_hash,
            ChainTip {
                block_hash,
                height,
                finalized_height,
            },
        );
    }
}

/// Decision on what to do with an incoming block
#[derive(Debug, PartialEq)]
pub enum ForkDecision {
    /// Accept the block as extending our current chain
    Accept,
    /// Reorganize to a new chain
    Reorg { new_tip: [u8; 32], new_height: u64 },
    /// Ignore the block (old or duplicate)
    Ignore,
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::types::Header;

    #[test]
    fn test_fork_choice_accept() {
        let mut fork_choice = ForkChoice::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_fork_db");
        let block_store = BlockStore::new(path.to_str().unwrap()).unwrap();

        // Store genesis block
        let genesis = Block::genesis();
        block_store.put_block(&genesis).unwrap();
        block_store.set_latest_height(0).unwrap();

        // Create block 1
        let header = Header::new(genesis.hash(), 1);
        let block1 = Block::new(header, vec![]);

        let decision = fork_choice
            .handle_incoming_block(&block1, &block_store)
            .unwrap();
        assert_eq!(decision, ForkDecision::Accept);
    }

    /// Regression: a higher block used to trigger a reorg with no finality
    /// check at all.
    #[test]
    fn test_higher_block_does_not_reorg() {
        let mut fork_choice = ForkChoice::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_reorg_db");
        let block_store = BlockStore::new(path.to_str().unwrap()).unwrap();

        // Set current height to 5
        block_store.set_latest_height(5).unwrap();

        // Incoming block at height 10 (fork with higher height)
        let header = Header::new([0u8; 32], 10);
        let block10 = Block::new(header, vec![]);

        let decision = fork_choice
            .handle_incoming_block(&block10, &block_store)
            .unwrap();
        assert!(matches!(decision, ForkDecision::Ignore));
    }

    #[test]
    fn test_fork_choice_ignore() {
        let mut fork_choice = ForkChoice::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_ignore_db");
        let block_store = BlockStore::new(path.to_str().unwrap()).unwrap();

        // Set current height to 10
        block_store.set_latest_height(10).unwrap();

        // Incoming block at height 5 (old block)
        let header = Header::new([0u8; 32], 5);
        let block5 = Block::new(header, vec![]);

        let decision = fork_choice
            .handle_incoming_block(&block5, &block_store)
            .unwrap();
        assert_eq!(decision, ForkDecision::Ignore);
    }
}
