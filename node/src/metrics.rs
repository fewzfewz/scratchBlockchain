use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Blockchain metrics for monitoring and observability
#[derive(Clone)]
pub struct Metrics {
    /// Total transactions processed
    pub total_transactions: Arc<AtomicU64>,
    /// Total blocks produced
    pub total_blocks: Arc<AtomicU64>,
    /// Current mempool size
    pub mempool_size: Arc<AtomicUsize>,
    /// Current peer count
    pub peer_count: Arc<AtomicUsize>,
    /// Last block time (unix timestamp)
    pub last_block_time: Arc<AtomicU64>,
    /// Finalized block height
    pub finalized_height: Arc<AtomicU64>,
    /// Total MEV protected transactions
    pub mev_protected_txs: Arc<AtomicU64>,
    /// Total account abstraction operations
    pub aa_operations: Arc<AtomicU64>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            total_transactions: Arc::new(AtomicU64::new(0)),
            total_blocks: Arc::new(AtomicU64::new(0)),
            mempool_size: Arc::new(AtomicUsize::new(0)),
            peer_count: Arc::new(AtomicUsize::new(0)),
            last_block_time: Arc::new(AtomicU64::new(0)),
            finalized_height: Arc::new(AtomicU64::new(0)),
            mev_protected_txs: Arc::new(AtomicU64::new(0)),
            aa_operations: Arc::new(AtomicU64::new(0)),
            consensus_round: Arc::new(AtomicU64::new(0)),
            network_bytes_rx: Arc::new(AtomicU64::new(0)),
            network_bytes_tx: Arc::new(AtomicU64::new(0)),
            block_latency: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn record_transaction(&self) {
        self.total_transactions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_block(&self) {
        self.total_blocks.fetch_add(1, Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_block_time.store(now, Ordering::Relaxed);
    }

    pub fn update_mempool_size(&self, size: usize) {
        self.mempool_size.store(size, Ordering::Relaxed);
    }

    pub fn update_peer_count(&self, count: usize) {
        self.peer_count.store(count, Ordering::Relaxed);
    }

    pub fn update_finalized_height(&self, height: u64) {
        self.finalized_height.store(height, Ordering::Relaxed);
    }

    pub fn record_mev_protected_tx(&self) {
        self.mev_protected_txs.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_aa_operation(&self) {
        self.aa_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current transactions per second (TPS)
    pub fn get_tps(&self) -> f64 {
        let total_txs = self.total_transactions.load(Ordering::Relaxed);
        let last_block = self.last_block_time.load(Ordering::Relaxed);
        
        if last_block == 0 {
            return 0.0;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let elapsed = now.saturating_sub(last_block).max(1);
        total_txs as f64 / elapsed as f64
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();
        
        output.push_str("# HELP blockchain_transactions_total Total number of transactions processed\n");
        output.push_str("# TYPE blockchain_transactions_total counter\n");
        output.push_str(&format!("blockchain_transactions_total {}\n", 
            self.total_transactions.load(Ordering::Relaxed)));
        
        output.push_str("# HELP blockchain_blocks_total Total number of blocks produced\n");
        output.push_str("# TYPE blockchain_blocks_total counter\n");
        output.push_str(&format!("blockchain_blocks_total {}\n", 
            self.total_blocks.load(Ordering::Relaxed)));
        
        output.push_str("# HELP blockchain_mempool_size Current mempool size\n");
        output.push_str("# TYPE blockchain_mempool_size gauge\n");
        output.push_str(&format!("blockchain_mempool_size {}\n", 
            self.mempool_size.load(Ordering::Relaxed)));
        
        output.push_str("# HELP blockchain_peer_count Current peer count\n");
        output.push_str("# TYPE blockchain_peer_count gauge\n");
        output.push_str(&format!("blockchain_peer_count {}\n", 
            self.peer_count.load(Ordering::Relaxed)));
        
        output.push_str("# HELP blockchain_finalized_height Finalized block height\n");
        output.push_str("# TYPE blockchain_finalized_height gauge\n");
        output.push_str(&format!("blockchain_finalized_height {}\n", 
            self.finalized_height.load(Ordering::Relaxed)));
        
        output.push_str("# HELP blockchain_tps Transactions per second\n");
        output.push_str("# TYPE blockchain_tps gauge\n");
        output.push_str(&format!("blockchain_tps {:.2}\n", self.get_tps()));
        
        output.push_str("# HELP blockchain_mev_protected_txs_total MEV protected transactions\n");
        output.push_str("# TYPE blockchain_mev_protected_txs_total counter\n");
        output.push_str(&format!("blockchain_mev_protected_txs_total {}\n", 
            self.mev_protected_txs.load(Ordering::Relaxed)));
        
        output.push_str("# HELP blockchain_aa_operations_total Account abstraction operations\n");
        output.push_str(&format!("blockchain_aa_operations_total {}\n", 
            self.aa_operations.load(Ordering::Relaxed)));

        output.push_str("# HELP blockchain_consensus_round Current consensus round\n");
        output.push_str("# TYPE blockchain_consensus_round gauge\n");
        output.push_str(&format!("blockchain_consensus_round {}\n", 
            self.consensus_round.load(Ordering::Relaxed)));

        output.push_str("# HELP blockchain_network_bytes_rx_total Total network bytes received\n");
        output.push_str("# TYPE blockchain_network_bytes_rx_total counter\n");
        output.push_str(&format!("blockchain_network_bytes_rx_total {}\n", 
            self.network_bytes_rx.load(Ordering::Relaxed)));

        output.push_str("# HELP blockchain_network_bytes_tx_total Total network bytes transmitted\n");
        output.push_str("# TYPE blockchain_network_bytes_tx_total counter\n");
        output.push_str(&format!("blockchain_network_bytes_tx_total {}\n", 
            self.network_bytes_tx.load(Ordering::Relaxed)));

        output.push_str("# HELP blockchain_block_latency_ms Block propagation latency in ms\n");
        output.push_str("# TYPE blockchain_block_latency_ms gauge\n");
        output.push_str(&format!("blockchain_block_latency_ms {}\n", 
            self.block_latency.load(Ordering::Relaxed)));
        
        output
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let metrics = Metrics::new();
        
        metrics.record_transaction();
        metrics.record_transaction();
        metrics.record_block();
        
        assert_eq!(metrics.total_transactions.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.total_blocks.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = Metrics::new();
        metrics.record_transaction();
        metrics.update_mempool_size(5);
        
        let output = metrics.export_prometheus();
        assert!(output.contains("blockchain_transactions_total 1"));
        assert!(output.contains("blockchain_mempool_size 5"));
    }
}
