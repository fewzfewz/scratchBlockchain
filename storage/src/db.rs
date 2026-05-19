//! # Database Layer for Blockchain Storage
//!
//! This module provides a high-performance, persistent storage layer for blockchain data.
//! It supports:
//! - Multiple column families for logical separation
//! - Atomic batch writes (critical for block commits)
//! - Iterator support for range scans
//! - In-memory caching for hot data
//! - Metrics for monitoring
//!
//! ## Architecture
//! - `KeyValueStore` trait abstracts the underlying database
//! - `MemDb` for testing (in-memory with RwLock)
//! - `RocksDb` for production (persistent with column families)
//! - `ChainStore` provides type-safe access to blockchain data
//!
//! ## Atomicity Guarantee
//! The `commit_block` method uses RocksDB's WriteBatch to ensure that
//! block data, state changes, and receipts are written atomically.
//! If the node crashes mid-write, the database recovers to a consistent state.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, warn, trace};

// ============================================================================
// Metrics for Database Monitoring
// ============================================================================

/// Database performance metrics
#[derive(Debug, Clone, Default)]
pub struct DbMetrics {
    /// Total number of read operations
    pub reads: u64,
    /// Total number of write operations
    pub writes: u64,
    /// Total number of delete operations
    pub deletes: u64,
    /// Total number of batch operations
    pub batches: u64,
    /// Cumulative read latency (microseconds)
    pub read_latency_us: u64,
    /// Cumulative write latency (microseconds)
    pub write_latency_us: u64,
    /// Cache hit count
    pub cache_hits: u64,
    /// Cache miss count
    pub cache_misses: u64,
}

impl DbMetrics {
    /// Calculate average read latency in microseconds
    pub fn avg_read_latency_us(&self) -> Option<u64> {
        if self.reads == 0 {
            None
        } else {
            Some(self.read_latency_us / self.reads)
        }
    }
    
    /// Calculate average write latency in microseconds
    pub fn avg_write_latency_us(&self) -> Option<u64> {
        if self.writes == 0 {
            None
        } else {
            Some(self.write_latency_us / self.writes)
        }
    }
    
    /// Cache hit rate (0.0 to 1.0)
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }
}

// ============================================================================
// Error Type
// ============================================================================

#[derive(Debug)]
pub struct DbError(String);

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DbError: {}", self.0)
    }
}

impl Error for DbError {}

impl DbError {
    pub fn new(msg: impl Into<String>) -> Self {
        DbError(msg.into())
    }
}

// ============================================================================
// Column Families
// ============================================================================

/// Column families (logical tables) for organizing blockchain data
/// 
/// Using column families instead of a flat keyspace prevents key collisions
/// and improves performance by keeping related data together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColumnFamily {
    /// Raw block data keyed by block hash (32 bytes)
    Blocks,
    
    /// Block height → block hash index (u64 → [u8; 32])
    BlockHeights,
    
    /// Account and contract storage state (key: address+slot)
    State,
    
    /// Transaction receipts keyed by transaction hash
    Receipts,
    
    /// Node metadata (latest height, genesis hash, chain ID)
    Meta,
}

impl ColumnFamily {
    /// Prefix byte used to namespace keys in the flat store
    /// Each CF gets a unique prefix so keys never collide in `MemDb`
    pub fn prefix(self) -> u8 {
        match self {
            ColumnFamily::Blocks       => 0x01,
            ColumnFamily::BlockHeights => 0x02,
            ColumnFamily::State        => 0x03,
            ColumnFamily::Receipts     => 0x04,
            ColumnFamily::Meta         => 0x05,
        }
    }
    
    /// Get column family name for RocksDB
    pub fn name(self) -> &'static str {
        match self {
            ColumnFamily::Blocks       => "blocks",
            ColumnFamily::BlockHeights => "block_heights",
            ColumnFamily::State        => "state",
            ColumnFamily::Receipts     => "receipts",
            ColumnFamily::Meta         => "meta",
        }
    }
}

// ============================================================================
// Batch Write - Atomic Multi-Key Operations
// ============================================================================

/// A batch of write operations that can be applied atomically
/// 
/// This is critical for blockchain commits: block data, state changes,
/// and receipts must all be written together. If the node crashes during
/// a commit, the batch is either fully applied or fully rolled back.
#[derive(Default)]
pub struct WriteBatch {
    ops: Vec<BatchOp>,
    size_hint: usize,  // Estimated byte size for RocksDB
}

/// Single operation within a batch
enum BatchOp {
    Put {
        cf: ColumnFamily,
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Delete {
        cf: ColumnFamily,
        key: Vec<u8>,
    },
}

impl WriteBatch {
    /// Create a new empty write batch
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a put operation to the batch
    pub fn put(&mut self, cf: ColumnFamily, key: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) {
        let key = key.into();
        let value = value.into();
        self.size_hint += key.len() + value.len() + 1;
        self.ops.push(BatchOp::Put { cf, key, value });
    }
    
    /// Add a delete operation to the batch
    pub fn delete(&mut self, cf: ColumnFamily, key: impl Into<Vec<u8>>) {
        let key = key.into();
        self.size_hint += key.len() + 1;
        self.ops.push(BatchOp::Delete { cf, key });
    }
    
    /// Get the number of operations in the batch
    pub fn len(&self) -> usize {
        self.ops.len()
    }
    
    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
    
    /// Get estimated byte size of the batch
    pub fn estimated_size(&self) -> usize {
        self.size_hint
    }
}

// ============================================================================
// KeyValueStore Trait
// ============================================================================

/// Core database interface that all storage backends must implement
pub trait KeyValueStore: Send + Sync {
    /// Get a value by key from the specified column family
    fn get(&self, cf: ColumnFamily, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn Error>>;
    
    /// Put a key-value pair into the specified column family
    fn put(&self, cf: ColumnFamily, key: &[u8], value: &[u8]) -> Result<(), Box<dyn Error>>;
    
    /// Delete a key from the specified column family
    fn delete(&self, cf: ColumnFamily, key: &[u8]) -> Result<(), Box<dyn Error>>;
    
    /// Check if a key exists in the specified column family
    fn contains(&self, cf: ColumnFamily, key: &[u8]) -> Result<bool, Box<dyn Error>>;
    
    /// Apply a batch of operations atomically
    /// Either all operations succeed or none are applied
    fn write_batch(&self, batch: WriteBatch) -> Result<(), Box<dyn Error>>;
    
    /// Iterate over all key-value pairs in a column family
    /// Returns an iterator that yields (key, value) pairs
    fn iter(&self, cf: ColumnFamily) -> Result<Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + '_>, Box<dyn Error>>;
    
    /// Iterate over a key range in a column family
    fn scan(
        &self,
        cf: ColumnFamily,
        start_key: &[u8],
        end_key: &[u8],
    ) -> Result<Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + '_>, Box<dyn Error>>;
    
    /// Flush all pending writes to disk
    /// Ensures data is persisted before returning
    fn flush(&self) -> Result<(), Box<dyn Error>>;
    
    /// Get database metrics
    fn get_metrics(&self) -> DbMetrics;
}

// ============================================================================
// In-Memory Database (for Testing)
// ============================================================================

/// In-memory database implementation using RwLock
/// Used for testing and development (not production)
#[derive(Clone)]
pub struct MemDb {
    /// Actual key-value store (prefixed keys)
    store: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
    /// Metrics tracking
    metrics: Arc<RwLock<DbMetrics>>,
    /// Cache for hot data
    cache: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
    cache_size_limit: usize,
}

impl Default for MemDb {
    fn default() -> Self {
        Self::new()
    }
}

impl MemDb {
    /// Create a new in-memory database
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(DbMetrics::default())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_size_limit: 10000,  // Cache up to 10k items
        }
    }
    
    /// Prepend the column-family prefix byte to the key
    fn prefix_key(cf: ColumnFamily, key: &[u8]) -> Vec<u8> {
        let mut prefixed = Vec::with_capacity(1 + key.len());
        prefixed.push(cf.prefix());
        prefixed.extend_from_slice(key);
        prefixed
    }
    
    /// Update read metrics
    fn record_read(&self, latency_us: u64, hit: bool) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.reads += 1;
        metrics.read_latency_us += latency_us;
        if hit {
            metrics.cache_hits += 1;
        } else {
            metrics.cache_misses += 1;
        }
    }
    
    /// Update write metrics
    fn record_write(&self, latency_us: u64) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.writes += 1;
        metrics.write_latency_us += latency_us;
    }
    
    /// Try to get from cache
    fn get_cached(&self, prefixed_key: &[u8]) -> Option<Vec<u8>> {
        let cache = self.cache.read().unwrap();
        cache.get(prefixed_key).cloned()
    }
    
    /// Put into cache
    fn put_cache(&self, prefixed_key: Vec<u8>, value: Vec<u8>) {
        let mut cache = self.cache.write().unwrap();
        
        // Evict oldest if cache is too large (simple FIFO for now)
        if cache.len() >= self.cache_size_limit {
            if let Some(oldest) = cache.keys().next().cloned() {
                cache.remove(&oldest);
            }
        }
        
        cache.insert(prefixed_key, value);
    }
}

impl KeyValueStore for MemDb {
    fn get(&self, cf: ColumnFamily, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        let start = Instant::now();
        let prefixed = Self::prefix_key(cf, key);
        
        // Check cache first
        if let Some(value) = self.get_cached(&prefixed) {
            let latency = start.elapsed().as_micros() as u64;
            self.record_read(latency, true);
            return Ok(Some(value));
        }
        
        // Cache miss - read from store
        let guard = self
            .store
            .read()
            .map_err(|_| DbError::new("RwLock poisoned on read"))?;
        
        let result = guard.get(&prefixed).cloned();
        
        // Cache if found
        if let Some(ref value) = result {
            self.put_cache(prefixed, value.clone());
        }
        
        let latency = start.elapsed().as_micros() as u64;
        self.record_read(latency, false);
        Ok(result)
    }
    
    fn put(&self, cf: ColumnFamily, key: &[u8], value: &[u8]) -> Result<(), Box<dyn Error>> {
        let start = Instant::now();
        let prefixed = Self::prefix_key(cf, key);
        
        let mut guard = self
            .store
            .write()
            .map_err(|_| DbError::new("RwLock poisoned on write"))?;
        
        guard.insert(prefixed.clone(), value.to_vec());
        
        // Update cache
        self.put_cache(prefixed, value.to_vec());
        
        let latency = start.elapsed().as_micros() as u64;
        self.record_write(latency);
        Ok(())
    }
    
    fn delete(&self, cf: ColumnFamily, key: &[u8]) -> Result<(), Box<dyn Error>> {
        let start = Instant::now();
        let prefixed = Self::prefix_key(cf, key);
        
        let mut guard = self
            .store
            .write()
            .map_err(|_| DbError::new("RwLock poisoned on delete"))?;
        
        guard.remove(&prefixed);
        
        // Remove from cache
        let mut cache = self.cache.write().unwrap();
        cache.remove(&prefixed);
        
        let mut metrics = self.metrics.write().unwrap();
        metrics.deletes += 1;
        metrics.write_latency_us += start.elapsed().as_micros() as u64;
        
        Ok(())
    }
    
    fn contains(&self, cf: ColumnFamily, key: &[u8]) -> Result<bool, Box<dyn Error>> {
        Ok(self.get(cf, key)?.is_some())
    }
    
    fn write_batch(&self, batch: WriteBatch) -> Result<(), Box<dyn Error>> {
        let start = Instant::now();
        let mut guard = self
            .store
            .write()
            .map_err(|_| DbError::new("RwLock poisoned on write_batch"))?;
        
        for op in batch.ops {
            match op {
                BatchOp::Put { cf, key, value } => {
                    let prefixed = Self::prefix_key(cf, &key);
                    guard.insert(prefixed.clone(), value);
                    self.put_cache(prefixed, value);
                }
                BatchOp::Delete { cf, key } => {
                    let prefixed = Self::prefix_key(cf, &key);
                    guard.remove(&prefixed);
                    
                    let mut cache = self.cache.write().unwrap();
                    cache.remove(&prefixed);
                }
            }
        }
        
        let mut metrics = self.metrics.write().unwrap();
        metrics.batches += 1;
        metrics.write_latency_us += start.elapsed().as_micros() as u64;
        
        Ok(())
    }
    
    fn iter(&self, cf: ColumnFamily) -> Result<Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + '_>, Box<dyn Error>> {
        let guard = self
            .store
            .read()
            .map_err(|_| DbError::new("RwLock poisoned on iter"))?;
        
        let prefix = &[cf.prefix()];
        let mut results = Vec::new();
        
        for (key, value) in guard.iter() {
            if key.starts_with(prefix) {
                // Strip prefix before returning
                let stripped_key = key[1..].to_vec();
                results.push((stripped_key, value.clone()));
            }
        }
        
        Ok(Box::new(results.into_iter()))
    }
    
    fn scan(
        &self,
        cf: ColumnFamily,
        start_key: &[u8],
        end_key: &[u8],
    ) -> Result<Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + '_>, Box<dyn Error>> {
        let guard = self
            .store
            .read()
            .map_err(|_| DbError::new("RwLock poisoned on scan"))?;
        
        let prefix = &[cf.prefix()];
        let start_prefixed = [&[cf.prefix()], start_key].concat();
        let end_prefixed = [&[cf.prefix()], end_key].concat();
        
        let mut results = Vec::new();
        for (key, value) in guard.iter() {
            if key >= &start_prefixed && key <= &end_prefixed && key.starts_with(prefix) {
                let stripped_key = key[1..].to_vec();
                results.push((stripped_key, value.clone()));
            }
        }
        
        Ok(Box::new(results.into_iter()))
    }
    
    fn flush(&self) -> Result<(), Box<dyn Error>> {
        // In-memory database doesn't need flushing
        Ok(())
    }
    
    fn get_metrics(&self) -> DbMetrics {
        self.metrics.read().unwrap().clone()
    }
}

// ============================================================================
// RocksDB Persistent Implementation
// ============================================================================

#[cfg(feature = "rocksdb")]
pub mod rocks {
    use super::*;
    use rocksdb::{
        ColumnFamilyDescriptor, Options, WriteBatch as RocksWriteBatch,
        DB, IteratorMode, Direction,
    };
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};
    
    /// Production database using RocksDB
    /// 
    /// RocksDB is a high-performance embedded key-value store from Facebook.
    /// It supports:
    /// - Column families for logical separation
    /// - Write-ahead logging (WAL) for crash recovery
    /// - Compression (Snappy, LZ4, Zstd)
    /// - Bloom filters for fast lookups
    pub struct RocksDb {
        db: Arc<DB>,
        metrics: Arc<RwLock<DbMetrics>>,
        // Atomic counters for lock-free metrics
        reads: AtomicU64,
        writes: AtomicU64,
        deletes: AtomicU64,
        batches: AtomicU64,
    }
    
    /// Configuration for RocksDB
    #[derive(Debug, Clone)]
    pub struct RocksDbConfig {
        /// Maximum database size in MB (default: 1024)
        pub max_db_size_mb: usize,
        /// Enable compression (default: true)
        pub enable_compression: bool,
        /// Number of threads for background compaction (default: 4)
        pub compaction_threads: usize,
        /// Write buffer size in MB (default: 64)
        pub write_buffer_size_mb: usize,
        /// Enable bloom filters (default: true)
        pub enable_bloom_filters: bool,
    }
    
    impl Default for RocksDbConfig {
        fn default() -> Self {
            Self {
                max_db_size_mb: 1024,
                enable_compression: true,
                compaction_threads: 4,
                write_buffer_size_mb: 64,
                enable_bloom_filters: true,
            }
        }
    }
    
    impl RocksDb {
        /// Open a RocksDB database at the given path
        pub fn open(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
            Self::open_with_config(path, RocksDbConfig::default())
        }
        
        /// Open a RocksDB database with custom configuration
        pub fn open_with_config(path: impl AsRef<Path>, config: RocksDbConfig) -> Result<Self, Box<dyn Error>> {
            let mut opts = Options::default();
            
            // Basic configuration
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);
            
            // Performance tuning
            opts.set_max_total_wal_size(config.max_db_size_mb as u64 * 1024 * 1024);
            opts.set_write_buffer_size(config.write_buffer_size_mb * 1024 * 1024);
            opts.set_max_background_jobs(config.compaction_threads as i32);
            
            // Compression
            if config.enable_compression {
                opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
                opts.set_bottommost_compression_type(rocksdb::DBCompressionType::Zstd);
            }
            
            // Bloom filters for fast point lookups
            let mut cf_opts = Options::default();
            if config.enable_bloom_filters {
                let mut bloom_opts = rocksdb::BloomFilterOptions::default();
                bloom_opts.set_bits_per_key(10.0);
                cf_opts.set_bloom_filter(&bloom_opts);
            }
            cf_opts.set_write_buffer_size(config.write_buffer_size_mb * 1024 * 1024);
            
            // Column families
            let cf_names: Vec<&str> = vec![
                ColumnFamily::Blocks.name(),
                ColumnFamily::BlockHeights.name(),
                ColumnFamily::State.name(),
                ColumnFamily::Receipts.name(),
                ColumnFamily::Meta.name(),
            ];
            
            let cfs: Vec<ColumnFamilyDescriptor> = cf_names
                .iter()
                .map(|name| ColumnFamilyDescriptor::new(*name, cf_opts.clone()))
                .collect();
            
            let db = DB::open_cf_descriptors(&opts, path, cfs)
                .map_err(|e| DbError::new(format!("Failed to open RocksDB: {}", e)))?;
            
            Ok(Self {
                db: Arc::new(db),
                metrics: Arc::new(RwLock::new(DbMetrics::default())),
                reads: AtomicU64::new(0),
                writes: AtomicU64::new(0),
                deletes: AtomicU64::new(0),
                batches: AtomicU64::new(0),
            })
        }
        
        /// Get column family handle
        fn cf_handle(&self, cf: ColumnFamily) -> Result<&rocksdb::ColumnFamily, Box<dyn Error>> {
            self.db
                .cf_handle(cf.name())
                .ok_or_else(|| DbError::new(format!("Unknown column family: {}", cf.name())).into())
        }
        
        /// Record metrics for a read operation
        fn record_read(&self, latency_us: u64) {
            self.reads.fetch_add(1, Ordering::Relaxed);
            let mut metrics = self.metrics.write().unwrap();
            metrics.reads += 1;
            metrics.read_latency_us += latency_us;
        }
        
        /// Record metrics for a write operation
        fn record_write(&self, latency_us: u64, is_batch: bool) {
            if is_batch {
                self.batches.fetch_add(1, Ordering::Relaxed);
                let mut metrics = self.metrics.write().unwrap();
                metrics.batches += 1;
            } else {
                self.writes.fetch_add(1, Ordering::Relaxed);
                let mut metrics = self.metrics.write().unwrap();
                metrics.writes += 1;
            }
            
            let mut metrics = self.metrics.write().unwrap();
            metrics.write_latency_us += latency_us;
        }
    }
    
    impl KeyValueStore for RocksDb {
        fn get(&self, cf: ColumnFamily, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
            let start = Instant::now();
            let handle = self.cf_handle(cf)?;
            let result = self
                .db
                .get_cf(handle, key)
                .map_err(|e| DbError::new(format!("RocksDB get error: {}", e)))?;
            
            let latency = start.elapsed().as_micros() as u64;
            self.record_read(latency);
            Ok(result)
        }
        
        fn put(&self, cf: ColumnFamily, key: &[u8], value: &[u8]) -> Result<(), Box<dyn Error>> {
            let start = Instant::now();
            let handle = self.cf_handle(cf)?;
            self.db
                .put_cf(handle, key, value)
                .map_err(|e| DbError::new(format!("RocksDB put error: {}", e)))?;
            
            let latency = start.elapsed().as_micros() as u64;
            self.record_write(latency, false);
            Ok(())
        }
        
        fn delete(&self, cf: ColumnFamily, key: &[u8]) -> Result<(), Box<dyn Error>> {
            let start = Instant::now();
            let handle = self.cf_handle(cf)?;
            self.db
                .delete_cf(handle, key)
                .map_err(|e| DbError::new(format!("RocksDB delete error: {}", e)))?;
            
            let latency = start.elapsed().as_micros() as u64;
            self.record_write(latency, false);
            self.deletes.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        
        fn contains(&self, cf: ColumnFamily, key: &[u8]) -> Result<bool, Box<dyn Error>> {
            Ok(self.get(cf, key)?.is_some())
        }
        
        fn write_batch(&self, batch: WriteBatch) -> Result<(), Box<dyn Error>> {
            let start = Instant::now();
            let mut rocks_batch = RocksWriteBatch::default();
            
            for op in batch.ops {
                match op {
                    BatchOp::Put { cf, key, value } => {
                        let handle = self.cf_handle(cf)?;
                        rocks_batch.put_cf(handle, &key, &value);
                    }
                    BatchOp::Delete { cf, key } => {
                        let handle = self.cf_handle(cf)?;
                        rocks_batch.delete_cf(handle, &key);
                    }
                }
            }
            
            self.db
                .write(rocks_batch)
                .map_err(|e| DbError::new(format!("RocksDB write_batch error: {}", e)))?;
            
            let latency = start.elapsed().as_micros() as u64;
            self.record_write(latency, true);
            Ok(())
        }
        
        fn iter(&self, cf: ColumnFamily) -> Result<Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + '_>, Box<dyn Error>> {
            let handle = self.cf_handle(cf)?;
            let iter = self.db.iterator_cf(handle, IteratorMode::Start);
            
            // Convert RocksDB iterator to our iterator type
            let mapped = iter.map(|item| {
                let (key, value) = item.unwrap();
                (key.to_vec(), value.to_vec())
            });
            
            Ok(Box::new(mapped))
        }
        
        fn scan(
            &self,
            cf: ColumnFamily,
            start_key: &[u8],
            end_key: &[u8],
        ) -> Result<Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + '_>, Box<dyn Error>> {
            let handle = self.cf_handle(cf)?;
            let iter = self.db.iterator_cf(handle, IteratorMode::From(start_key, Direction::Forward));
            
            let mapped = iter
                .take_while(|item| {
                    if let Ok((key, _)) = item {
                        key.as_ref() <= end_key
                    } else {
                        false
                    }
                })
                .map(|item| {
                    let (key, value) = item.unwrap();
                    (key.to_vec(), value.to_vec())
                });
            
            Ok(Box::new(mapped))
        }
        
        fn flush(&self) -> Result<(), Box<dyn Error>> {
            self.db
                .flush_wal()
                .map_err(|e| DbError::new(format!("RocksDB flush error: {}", e)))?;
            Ok(())
        }
        
        fn get_metrics(&self) -> DbMetrics {
            let mut metrics = self.metrics.read().unwrap().clone();
            // Update atomic counters
            metrics.reads = self.reads.load(Ordering::Relaxed);
            metrics.writes = self.writes.load(Ordering::Relaxed);
            metrics.deletes = self.deletes.load(Ordering::Relaxed);
            metrics.batches = self.batches.load(Ordering::Relaxed);
            metrics
        }
    }
}

// ============================================================================
// High-Level ChainStore with Type-Safe Access
// ============================================================================

/// High-level store that knows how to read/write blockchain data
/// 
/// This provides type-safe methods for blockchain-specific operations
/// while using the underlying KeyValueStore for actual storage.
pub struct ChainStore {
    inner: Arc<dyn KeyValueStore>,
}

impl ChainStore {
    /// Create a new ChainStore wrapping a KeyValueStore
    pub fn new(inner: Arc<dyn KeyValueStore>) -> Self {
        Self { inner }
    }
    
    /// Get a reference to the underlying store
    pub fn inner(&self) -> &Arc<dyn KeyValueStore> {
        &self.inner
    }
    
    // ========================================================================
    // Block Operations
    // ========================================================================
    
    /// Store a block by its hash
    pub fn put_block(&self, hash: &[u8; 32], encoded: &[u8]) -> Result<(), Box<dyn Error>> {
        self.inner.put(ColumnFamily::Blocks, hash, encoded)
    }
    
    /// Retrieve a block by its hash
    pub fn get_block(&self, hash: &[u8; 32]) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        self.inner.get(ColumnFamily::Blocks, hash)
    }
    
    /// Store the mapping from height to block hash
    pub fn put_block_height(&self, height: u64, hash: &[u8; 32]) -> Result<(), Box<dyn Error>> {
        self.inner.put(ColumnFamily::BlockHeights, &height.to_le_bytes(), hash)
    }
    
    /// Get block hash by height
    pub fn get_block_hash_by_height(&self, height: u64) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        self.inner.get(ColumnFamily::BlockHeights, &height.to_le_bytes())
    }
    
    /// Get block by height (convenience method)
    pub fn get_block_by_height(&self, height: u64) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        if let Some(hash) = self.get_block_hash_by_height(height)? {
            let mut hash_array = [0u8; 32];
            hash_array.copy_from_slice(&hash);
            self.get_block(&hash_array)
        } else {
            Ok(None)
        }
    }
    
    // ========================================================================
    // State Operations
    // ========================================================================
    
    /// Store a state key-value pair
    pub fn put_state(&self, key: &[u8], value: &[u8]) -> Result<(), Box<dyn Error>> {
        self.inner.put(ColumnFamily::State, key, value)
    }
    
    /// Retrieve a state value by key
    pub fn get_state(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        self.inner.get(ColumnFamily::State, key)
    }
    
    /// Delete a state key
    pub fn delete_state(&self, key: &[u8]) -> Result<(), Box<dyn Error>> {
        self.inner.delete(ColumnFamily::State, key)
    }
    
    /// Iterate over all state keys
    pub fn iter_state(&self) -> Result<Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + '_>, Box<dyn Error>> {
        self.inner.iter(ColumnFamily::State)
    }
    
    // ========================================================================
    // Receipt Operations
    // ========================================================================
    
    /// Store a transaction receipt
    pub fn put_receipt(&self, tx_hash: &[u8], encoded: &[u8]) -> Result<(), Box<dyn Error>> {
        self.inner.put(ColumnFamily::Receipts, tx_hash, encoded)
    }
    
    /// Retrieve a transaction receipt by hash
    pub fn get_receipt(&self, tx_hash: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        self.inner.get(ColumnFamily::Receipts, tx_hash)
    }
    
    // ========================================================================
    // Metadata Operations
    // ========================================================================
    
    /// Set the latest block height
    pub fn set_latest_height(&self, height: u64) -> Result<(), Box<dyn Error>> {
        self.inner.put(ColumnFamily::Meta, b"latest_height", &height.to_le_bytes())
    }
    
    /// Get the latest block height
    pub fn get_latest_height(&self) -> Result<Option<u64>, Box<dyn Error>> {
        let raw = self.inner.get(ColumnFamily::Meta, b"latest_height")?;
        Ok(raw.map(|b| {
            let arr: [u8; 8] = b.try_into().unwrap_or([0u8; 8]);
            u64::from_le_bytes(arr)
        }))
    }
    
    /// Set the genesis block hash
    pub fn set_genesis_hash(&self, hash: &[u8; 32]) -> Result<(), Box<dyn Error>> {
        self.inner.put(ColumnFamily::Meta, b"genesis_hash", hash)
    }
    
    /// Get the genesis block hash
    pub fn get_genesis_hash(&self) -> Result<Option<[u8; 32]>, Box<dyn Error>> {
        let raw = self.inner.get(ColumnFamily::Meta, b"genesis_hash")?;
        Ok(raw.map(|b| {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }))
    }
    
    /// Set the chain ID
    pub fn set_chain_id(&self, chain_id: &str) -> Result<(), Box<dyn Error>> {
        self.inner.put(ColumnFamily::Meta, b"chain_id", chain_id.as_bytes())
    }
    
    /// Get the chain ID
    pub fn get_chain_id(&self) -> Result<Option<String>, Box<dyn Error>> {
        let raw = self.inner.get(ColumnFamily::Meta, b"chain_id")?;
        Ok(raw.map(|b| String::from_utf8_lossy(&b).to_string()))
    }
    
    // ========================================================================
    // Atomic Block Commit
    // ========================================================================
    
    /// Commit a block atomically with its state changes and receipts
    /// 
    /// This is the most important method - it ensures all block-related data
    /// is written together atomically. If the node crashes during commit,
    /// the batch is either fully applied or fully rolled back.
    /// 
    /// # Arguments
    /// * `height` - Block height (number)
    /// * `hash` - Block hash (32 bytes)
    /// * `block_encoded` - Serialized block data
    /// * `state_diffs` - State changes: (key, Some(value)) for writes, (key, None) for deletes
    /// * `receipt_pairs` - Transaction receipts: (tx_hash, receipt_encoded)
    /// * `transactions` - Transactions included in the block (for mempool removal)
    pub fn commit_block(
        &self,
        height: u64,
        hash: &[u8; 32],
        block_encoded: &[u8],
        state_diffs: Vec<(Vec<u8>, Option<Vec<u8>>)>,
        receipt_pairs: Vec<(Vec<u8>, Vec<u8>)>,
    ) -> Result<(), Box<dyn Error>> {
        let mut batch = WriteBatch::new();
        
        // 1. Store block data
        batch.put(ColumnFamily::Blocks, hash.as_slice(), block_encoded);
        batch.put(ColumnFamily::BlockHeights, &height.to_le_bytes(), hash.as_slice());
        
        // 2. Apply state changes (inserts and deletes)
        for (key, value) in state_diffs {
            match value {
                Some(v) => batch.put(ColumnFamily::State, key, v),
                None    => batch.delete(ColumnFamily::State, key),
            }
        }
        
        // 3. Store receipts
        for (tx_hash, receipt) in receipt_pairs {
            batch.put(ColumnFamily::Receipts, tx_hash, receipt);
        }
        
        // 4. Update metadata (last!)
        batch.put(ColumnFamily::Meta, b"latest_height", &height.to_le_bytes());
        
        debug!(
            "Committing block {} with {} state changes, {} receipts",
            height,
            state_diffs.len(),
            receipt_pairs.len()
        );
        
        let start = Instant::now();
        self.inner.write_batch(batch)?;
        let elapsed = start.elapsed();
        
        trace!("Block commit completed in {:?}", elapsed);
        Ok(())
    }
    
    /// Check if the database is empty (no blocks)
    pub fn is_empty(&self) -> Result<bool, Box<dyn Error>> {
        Ok(self.get_latest_height()?.is_none())
    }
    
    /// Get database metrics
    pub fn get_metrics(&self) -> DbMetrics {
        self.inner.get_metrics()
    }
    
    /// Flush all pending writes
    pub fn flush(&self) -> Result<(), Box<dyn Error>> {
        self.inner.flush()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memdb_put_get_delete() {
        let db = MemDb::new();
        let key = b"hello";
        let val = b"world";
        
        db.put(ColumnFamily::State, key, val).unwrap();
        assert_eq!(db.get(ColumnFamily::State, key).unwrap(), Some(val.to_vec()));
        assert!(db.contains(ColumnFamily::State, key).unwrap());
        
        db.delete(ColumnFamily::State, key).unwrap();
        assert_eq!(db.get(ColumnFamily::State, key).unwrap(), None);
        assert!(!db.contains(ColumnFamily::State, key).unwrap());
    }
    
    #[test]
    fn test_column_family_isolation() {
        let db = MemDb::new();
        // Same key in different column families must not collide
        db.put(ColumnFamily::Blocks, b"key", b"block_data").unwrap();
        db.put(ColumnFamily::State,  b"key", b"state_data").unwrap();
        
        assert_eq!(
            db.get(ColumnFamily::Blocks, b"key").unwrap(),
            Some(b"block_data".to_vec())
        );
        assert_eq!(
            db.get(ColumnFamily::State, b"key").unwrap(),
            Some(b"state_data".to_vec())
        );
    }
    
    #[test]
    fn test_write_batch_atomic() {
        let db = MemDb::new();
        let mut batch = WriteBatch::new();
        batch.put(ColumnFamily::Blocks, b"block1", b"data1");
        batch.put(ColumnFamily::Receipts, b"tx1", b"receipt1");
        batch.put(ColumnFamily::Meta, b"height", &42u64.to_le_bytes());
        
        db.write_batch(batch).unwrap();
        
        assert_eq!(db.get(ColumnFamily::Blocks, b"block1").unwrap(), Some(b"data1".to_vec()));
        assert_eq!(db.get(ColumnFamily::Receipts, b"tx1").unwrap(), Some(b"receipt1".to_vec()));
        assert_eq!(db.get(ColumnFamily::Meta, b"height").unwrap(), Some(42u64.to_le_bytes().to_vec()));
    }
    
    #[test]
    fn test_batch_delete() {
        let db = MemDb::new();
        db.put(ColumnFamily::State, b"acc1", b"old_data").unwrap();
        
        let mut batch = WriteBatch::new();
        batch.put(ColumnFamily::State, b"acc2", b"new_data");
        batch.delete(ColumnFamily::State, b"acc1");
        db.write_batch(batch).unwrap();
        
        assert_eq!(db.get(ColumnFamily::State, b"acc1").unwrap(), None);
        assert_eq!(db.get(ColumnFamily::State, b"acc2").unwrap(), Some(b"new_data".to_vec()));
    }
    
    #[test]
    fn test_chain_store_commit_block() {
        let db = Arc::new(MemDb::new());
        let store = ChainStore::new(db);
        
        let hash = [0xABu8; 32];
        let state_diffs = vec![
            (b"account_alice".to_vec(), Some(b"balance:500".to_vec())),
            (b"account_dead".to_vec(),  None),
        ];
        let receipts = vec![(b"tx_hash_1".to_vec(), b"receipt_data".to_vec())];
        
        store
            .commit_block(1, &hash, b"block_encoded", state_diffs, receipts)
            .unwrap();
        
        assert_eq!(store.get_latest_height().unwrap(), Some(1));
        assert_eq!(store.get_block(&hash).unwrap(), Some(b"block_encoded".to_vec()));
        assert_eq!(store.get_state(b"account_alice").unwrap(), Some(b"balance:500".to_vec()));
        assert_eq!(store.get_state(b"account_dead").unwrap(), None);
        assert_eq!(store.get_receipt(b"tx_hash_1").unwrap(), Some(b"receipt_data".to_vec()));
    }
    
    #[test]
    fn test_iter_state() {
        let db = Arc::new(MemDb::new());
        let store = ChainStore::new(db);
        
        store.put_state(b"key1", b"value1").unwrap();
        store.put_state(b"key2", b"value2").unwrap();
        store.put_state(b"key3", b"value3").unwrap();
        
        let items: Vec<_> = store.iter_state().unwrap().collect();
        assert_eq!(items.len(), 3);
    }
    
    #[test]
    fn test_metrics() {
        let db = MemDb::new();
        
        db.put(ColumnFamily::State, b"key", b"value").unwrap();
        db.get(ColumnFamily::State, b"key").unwrap();
        db.get(ColumnFamily::State, b"key").unwrap(); // Should hit cache
        
        let metrics = db.get_metrics();
        assert_eq!(metrics.reads, 2);
        assert_eq!(metrics.writes, 1);
        assert!(metrics.cache_hits > 0);
    }
}