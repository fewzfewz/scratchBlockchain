use prometheus::{
    Counter, Gauge, Histogram, HistogramOpts, IntCounter, IntGauge, Opts, Registry,
};
use std::sync::Arc;

/// Blockchain metrics for Prometheus
pub struct BlockchainMetrics {
    registry: Registry,
    
    // Block metrics
    pub block_height: IntGauge,
    pub block_time: Histogram,
    pub blocks_produced: IntCounter,
    
    // Transaction metrics
    pub tx_total: IntCounter,
    pub tx_pending: IntGauge,
    pub tx_processing_time: Histogram,
    
    // Validator metrics
    pub validator_count: IntGauge,
    pub validator_stake_total: Gauge,
    pub missed_blocks: IntCounter,
    
    // Network metrics
    pub peer_count: IntGauge,
    pub network_bytes_sent: IntCounter,
    pub network_bytes_received: IntCounter,
    
    // Consensus metrics
    pub consensus_rounds: IntCounter,
    pub consensus_time: Histogram,
    pub finality_time: Histogram,
    
    // Storage metrics
    pub state_size_bytes: IntGauge,
    pub db_read_time: Histogram,
    pub db_write_time: Histogram,
}

impl BlockchainMetrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        
        // Block metrics
        let block_height = IntGauge::with_opts(
            Opts::new("chain_block_height", "Current block height")
        )?;
        
        let block_time = Histogram::with_opts(
            HistogramOpts::new("chain_block_time_seconds", "Block production time")
                .buckets(vec![0.5, 1.0, 2.0, 3.0, 5.0, 10.0])
        )?;
        
        let blocks_produced = IntCounter::with_opts(
            Opts::new("chain_blocks_produced_total", "Total blocks produced")
        )?;
        
        // Transaction metrics
        let tx_total = IntCounter::with_opts(
            Opts::new("chain_transactions_total", "Total transactions processed")
        )?;
        
        let tx_pending = IntGauge::with_opts(
            Opts::new("chain_transactions_pending", "Pending transactions in mempool")
        )?;
        
        let tx_processing_time = Histogram::with_opts(
            HistogramOpts::new("chain_tx_processing_seconds", "Transaction processing time")
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5])
        )?;
        
        // Validator metrics
        let validator_count = IntGauge::with_opts(
            Opts::new("chain_validator_count", "Number of active validators")
        )?;
        
        let validator_stake_total = Gauge::with_opts(
            Opts::new("chain_validator_stake_total", "Total staked amount")
        )?;
        
        let missed_blocks = IntCounter::with_opts(
            Opts::new("chain_missed_blocks_total", "Total missed blocks")
        )?;
        
        // Network metrics
        let peer_count = IntGauge::with_opts(
            Opts::new("chain_peer_count", "Number of connected peers")
        )?;
        
        let network_bytes_sent = IntCounter::with_opts(
            Opts::new("chain_network_bytes_sent_total", "Total bytes sent")
        )?;
        
        let network_bytes_received = IntCounter::with_opts(
            Opts::new("chain_network_bytes_received_total", "Total bytes received")
        )?;
        
        // Consensus metrics
        let consensus_rounds = IntCounter::with_opts(
            Opts::new("chain_consensus_rounds_total", "Total consensus rounds")
        )?;
        
        let consensus_time = Histogram::with_opts(
            HistogramOpts::new("chain_consensus_time_seconds", "Consensus round time")
                .buckets(vec![1.0, 2.0, 3.0, 5.0, 10.0, 30.0])
        )?;
        
        let finality_time = Histogram::with_opts(
            HistogramOpts::new("chain_finality_time_seconds", "Block finality time")
                .buckets(vec![3.0, 6.0, 9.0, 12.0, 15.0, 30.0])
        )?;
        
        // Storage metrics
        let state_size_bytes = IntGauge::with_opts(
            Opts::new("chain_state_size_bytes", "Total state size in bytes")
        )?;
        
        let db_read_time = Histogram::with_opts(
            HistogramOpts::new("chain_db_read_seconds", "Database read time")
                .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05])
        )?;
        
        let db_write_time = Histogram::with_opts(
            HistogramOpts::new("chain_db_write_seconds", "Database write time")
                .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05])
        )?;
        
        // Register all metrics
        registry.register(Box::new(block_height.clone()))?;
        registry.register(Box::new(block_time.clone()))?;
        registry.register(Box::new(blocks_produced.clone()))?;
        registry.register(Box::new(tx_total.clone()))?;
        registry.register(Box::new(tx_pending.clone()))?;
        registry.register(Box::new(tx_processing_time.clone()))?;
        registry.register(Box::new(validator_count.clone()))?;
        registry.register(Box::new(validator_stake_total.clone()))?;
        registry.register(Box::new(missed_blocks.clone()))?;
        registry.register(Box::new(peer_count.clone()))?;
        registry.register(Box::new(network_bytes_sent.clone()))?;
        registry.register(Box::new(network_bytes_received.clone()))?;
        registry.register(Box::new(consensus_rounds.clone()))?;
        registry.register(Box::new(consensus_time.clone()))?;
        registry.register(Box::new(finality_time.clone()))?;
        registry.register(Box::new(state_size_bytes.clone()))?;
        registry.register(Box::new(db_read_time.clone()))?;
        registry.register(Box::new(db_write_time.clone()))?;
        
        Ok(Self {
            registry,
            block_height,
            block_time,
            blocks_produced,
            tx_total,
            tx_pending,
            tx_processing_time,
            validator_count,
            validator_stake_total,
            missed_blocks,
            peer_count,
            network_bytes_sent,
            network_bytes_received,
            consensus_rounds,
            consensus_time,
            finality_time,
            state_size_bytes,
            db_read_time,
            db_write_time,
        })
    }
    
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

impl Default for BlockchainMetrics {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics")
    }
}
