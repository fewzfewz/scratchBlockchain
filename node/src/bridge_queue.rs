//! In-memory bridge unlock queue for relayers (Nebula lock → ETH unlock).

use serde::Serialize;
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize)]
pub struct PendingUnlock {
    pub nebula_tx_hash: String,
    pub eth_recipient: String,
    pub amount: String,
    pub sender: String,
    pub nonce: u64,
    pub message_id: u64,
    pub source_chain: u32,
    pub dest_chain: u32,
    pub block_height: u64,
}

pub struct BridgeUnlockQueue {
    processed: HashSet<String>,
    pending: Vec<PendingUnlock>,
}

impl BridgeUnlockQueue {
    pub fn new() -> Self {
        Self {
            processed: HashSet::new(),
            pending: Vec::new(),
        }
    }

    pub fn is_processed(&self, hash: &str) -> bool {
        self.processed.contains(hash)
    }

    pub fn is_pending(&self, hash: &str) -> bool {
        self.pending.iter().any(|p| p.nebula_tx_hash == hash)
    }

    pub fn enqueue(&mut self, entry: PendingUnlock) {
        if self.is_processed(&entry.nebula_tx_hash) || self.is_pending(&entry.nebula_tx_hash) {
            return;
        }
        self.pending.push(entry);
    }

    pub fn pending(&self) -> &[PendingUnlock] {
        &self.pending
    }

    pub fn processed_count(&self) -> usize {
        self.processed.len()
    }

    pub fn ack(&mut self, nebula_tx_hash: &str) -> bool {
        if let Some(pos) = self
            .pending
            .iter()
            .position(|p| p.nebula_tx_hash == nebula_tx_hash)
        {
            let entry = self.pending.remove(pos);
            self.processed.insert(entry.nebula_tx_hash);
            true
        } else {
            false
        }
    }

    /// Legacy path: mark processed without relayer ack (queue-only mode).
    pub fn mark_processed(&mut self, hash: String) {
        self.pending.retain(|p| p.nebula_tx_hash != hash);
        self.processed.insert(hash);
    }
}
