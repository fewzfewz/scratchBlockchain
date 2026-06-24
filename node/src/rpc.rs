//! # JSON-RPC Server Module
//!
//! This module implements the HTTP/JSON-RPC API for the blockchain node.
//! It provides endpoints for:
//! - Submitting transactions
//! - Querying balances, blocks, and receipts
//! - Checking node status and mempool
//! - Estimating gas prices
//! - Connecting to peers
//! - Prometheus metrics
//!
//! ## Rate Limiting
//! Each IP is limited to 100 requests per second to prevent DoS attacks.
//!
//! ## CORS
//! CORS headers are enabled to allow web wallets and explorers to connect.

use common::types::Transaction;
use mempool::Mempool;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use storage::ChainStore;
use tokio::sync::Mutex;
use warp::{Filter, http::HeaderValue};
pub use network::NetworkCommand;
use tokio::sync::mpsc;
use execution::gas::calculate_next_base_fee;

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize)]
struct StatusResponse {
    height: u64,
    finalized_height: Option<u64>,
    mempool_size: usize,
    peer_count: usize,
}

#[derive(Debug, Serialize)]
struct MempoolResponse {
    size: usize,
    transactions: Vec<Transaction>,
}

#[derive(Debug, Serialize)]
struct SubmitTxResponse {
    status: String,
    hash: String,
}

#[derive(Debug, Serialize)]
struct BlockResponse {
    block: Option<serde_json::Value>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct BalanceResponse {
    address: String,
    balance: String,
    nonce: u64,
}

#[derive(Debug, Serialize)]
struct ReceiptResponse {
    receipt: Option<serde_json::Value>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConnectPeerRequest {
    multiaddr: String,
}

#[derive(Debug, Serialize)]
struct ConnectPeerResponse {
    status: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

// ============================================================================
// Economic/Phase 9 Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct GasPriceResponse {
    base_fee: String,
    suggested_priority_fee_low: String,
    suggested_priority_fee_medium: String,
    suggested_priority_fee_high: String,
    block_height: u64,
}

#[derive(Debug, Deserialize)]
struct EstimateGasRequest {
    from: String,
    to: String,
    data: String,
    value: Option<String>,
    max_fee_per_gas: Option<String>,
}

#[derive(Debug, Serialize)]
struct EstimateGasResponse {
    estimated_gas: u64,
    base_fee: String,
    total_cost_estimate: String,
    estimated_priority_fee: String,
}

#[derive(Debug, Serialize)]
struct FeeHistoryResponse {
    base_fee_per_gas: Vec<String>,
    gas_used_ratio: Vec<f64>,
    oldest_block: u64,
}

// ============================================================================
// CORS Helper
// ============================================================================

/// Add CORS headers to allow web clients to access the API
fn with_cors() -> warp::cors::Cors {
    warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST", "OPTIONS"])
        .allow_headers(vec!["Content-Type", "Authorization"])
        .max_age(3600)
        .build()
}

// ============================================================================
// RpcServer
// ============================================================================

pub struct RpcServer {
    mempool: Arc<Mempool>,
    chain_store: Arc<ChainStore>,
    metrics: Arc<crate::metrics::Metrics>,
    network_cmd_sender: mpsc::Sender<NetworkCommand>,
}

impl RpcServer {
    pub fn new(
        mempool: Arc<Mempool>,
        chain_store: Arc<ChainStore>,
        metrics: Arc<crate::metrics::Metrics>,
        network_cmd_sender: mpsc::Sender<NetworkCommand>,
    ) -> Self {
        Self {
            mempool,
            chain_store,
            metrics,
            network_cmd_sender,
        }
    }

    pub async fn run(&self, port: u16, enable_cors: bool) {
        use governor::{Quota, RateLimiter};
        use governor::clock::DefaultClock;
        use governor::state::keyed::DefaultKeyedStateStore;
        use std::num::NonZeroU32;
        use std::net::IpAddr;

        let mempool = self.mempool.clone();
        let chain_store = self.chain_store.clone();
        let metrics = self.metrics.clone();
        let network_cmd_sender = self.network_cmd_sender.clone();

        // Rate limiter: 100 requests/second per IP
        let rate_limiter = Arc::new(
            RateLimiter::<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>::keyed(
                Quota::per_second(NonZeroU32::new(100).unwrap()),
            ),
        );

        let with_rate_limit = warp::any()
            .map(move || rate_limiter.clone())
            .and(warp::addr::remote())
            .and_then(
                |limiter: Arc<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>,
                 addr: Option<std::net::SocketAddr>| async move {
                    if let Some(addr) = addr {
                        if limiter.check_key(&addr.ip()).is_err() {
                            return Err(warp::reject::reject());
                        }
                    }
                    Ok(())
                },
            );

        // 1MB request body limit
        let body_limit = warp::body::content_length_limit(1024 * 1024);

        // ====================================================================
        // Core Routes
        // ====================================================================

        // GET /status - Node health and height information
        let status = warp::path("status")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and(with_arc(mempool.clone()))
            .and_then(|_, chain_store, mempool| handle_status(chain_store, mempool));

        // GET /mempool - List pending transactions
        let mempool_route = warp::path("mempool")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(mempool.clone()))
            .and_then(|_, mempool| handle_mempool(mempool));

        // POST /submit_tx - Submit a new transaction
        let submit_tx = warp::path("submit_tx")
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(mempool.clone()))
            .and(with_arc(network_cmd_sender.clone()))
            .and_then(handle_submit_tx);

        // GET /block/{height} - Get block by height
        let block_by_height = warp::path!("block" / u64)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and_then(|height, _, chain_store| handle_get_block_by_height(height, chain_store));

        // GET /block/hash/{hash} - Get block by hash
        let block_by_hash = warp::path!("block" / "hash" / String)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and_then(|hash, _, chain_store| handle_get_block_by_hash(hash, chain_store));

        // GET /balance/{address} - Get account balance and nonce
        let balance = warp::path!("balance" / String)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and_then(|address, _, chain_store| handle_get_balance(address, chain_store));

        // GET /tx/{hash} - Get transaction receipt
        let tx_receipt = warp::path!("tx" / String)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and_then(|hash, _, chain_store| handle_get_receipt(hash, chain_store));

        // ====================================================================
        // Economic Routes (Phase 9)
        // ====================================================================

        // GET /gas_price - Get current gas price suggestions (EIP-1559)
        let gas_price = warp::path("gas_price")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and_then(|_, chain_store| handle_gas_price(chain_store));

        // POST /estimate_gas - Estimate gas for a transaction
        let estimate_gas = warp::path("estimate_gas")
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(chain_store.clone()))
            .and_then(handle_estimate_gas);

        // GET /fee_history - Get historical fee data (EIP-1559)
        let fee_history = warp::path!("fee_history" / u64)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and_then(|block_count, _, chain_store| handle_fee_history(block_count, chain_store));

        // ====================================================================
        // Admin & Network Routes
        // ====================================================================

        // GET /metrics - Prometheus metrics endpoint
        let metrics_route = warp::path("metrics")
            .and(warp::get())
            .and(with_arc(metrics.clone()))
            .and_then(handle_metrics);

        // GET /health - Simple health check
        let health = warp::path("health")
            .and(warp::get())
            .and_then(handle_health);

        // POST /connect_peer - Connect to a peer by multiaddress
        let connect_peer = warp::path("connect_peer")
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(network_cmd_sender.clone()))
            .and_then(handle_connect_peer);

        // GET /peers - List connected peers
        let peers = warp::path("peers")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(network_cmd_sender.clone()))
            .and_then(|_, _network_cmd| handle_peers());

        // ====================================================================
        // Combine Routes with CORS
        // ====================================================================

        let routes = status
            .or(mempool_route)
            .or(submit_tx)
            .or(block_by_height)
            .or(block_by_hash)
            .or(balance)
            .or(tx_receipt)
            .or(gas_price)
            .or(estimate_gas)
            .or(fee_history)
            .or(metrics_route)
            .or(health)
            .or(connect_peer)
            .or(peers)
            .recover(handle_rejection);

        let cors = if enable_cors {
            with_cors()
        } else {
            warp::cors().allow_any_origin().build()
        };
        let routes_with_cors = routes.with(cors);

        tracing::info!("🚀 RPC server listening on 0.0.0.0:{}", port);
        warp::serve(routes_with_cors).run(([0, 0, 0, 0], port)).await;
    }
}

// ============================================================================
// Filter Helper
// ============================================================================

fn with_arc<T: Clone + Send + Sync>(
    val: T,
) -> impl Filter<Extract = (T,), Error = Infallible> + Clone {
    warp::any().map(move || val.clone())
}

// ============================================================================
// Core Handlers
// ============================================================================

async fn handle_status(
    chain_store: Arc<ChainStore>,
    mempool: Arc<Mempool>,
) -> Result<impl warp::Reply, Infallible> {
    let height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    let finalized_height = Some(height);
    let mempool_size = mempool.size();
    let peer_count = 0; // Would need network service connection for this

    Ok(warp::reply::json(&StatusResponse {
        height,
        finalized_height,
        mempool_size,
        peer_count,
    }))
}

async fn handle_mempool(mempool: Arc<Mempool>) -> Result<impl warp::Reply, Infallible> {
    let transactions = mempool.get_transactions(100);
    Ok(warp::reply::json(&MempoolResponse {
        size: mempool.size(),
        transactions,
    }))
}

async fn handle_submit_tx(
    tx: Transaction,
    mempool: Arc<Mempool>,
    network_cmd_sender: mpsc::Sender<NetworkCommand>,
) -> Result<impl warp::Reply, Infallible> {
    // Validate chain ID if present
    if let Some(tx_chain_id) = tx.chain_id {
        // In production, check against configured chain ID
        if tx_chain_id != 1 {
            return Ok(warp::reply::json(&SubmitTxResponse {
                status: format!("error: invalid chain_id (expected 1, got {})", tx_chain_id),
                hash: String::new(),
            }));
        }
    }

    match mempool.add_transaction(tx.clone()) {
        Ok(_) => {
            // Broadcast to peers
            let _ = network_cmd_sender
                .send(NetworkCommand::BroadcastTransaction(tx.clone()))
                .await;
            Ok(warp::reply::json(&SubmitTxResponse {
                status: "success".into(),
                hash: hex::encode(tx.hash()),
            }))
        }
        Err(e) => Ok(warp::reply::json(&SubmitTxResponse {
            status: format!("error: {}", e),
            hash: String::new(),
        })),
    }
}

async fn handle_get_block_by_height(
    height: u64,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    match chain_store.get_block_hash_by_height(height) {
        Ok(Some(hash_bytes)) if hash_bytes.len() == 32 => {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&hash_bytes);
            match chain_store.get_block(&hash) {
                Ok(Some(encoded)) => {
                    let val: serde_json::Value =
                        serde_json::from_slice(&encoded).unwrap_or(serde_json::Value::Null);
                    Ok(warp::reply::json(&BlockResponse { block: Some(val), error: None }))
                }
                Ok(None) => Ok(warp::reply::json(&BlockResponse {
                    block: None,
                    error: Some(format!("Block data missing for height {}", height)),
                })),
                Err(e) => Ok(warp::reply::json(&BlockResponse {
                    block: None,
                    error: Some(e.to_string()),
                })),
            }
        }
        Ok(_) => Ok(warp::reply::json(&BlockResponse {
            block: None,
            error: Some(format!("No block at height {}", height)),
        })),
        Err(e) => Ok(warp::reply::json(&BlockResponse {
            block: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_get_block_by_hash(
    hash_str: String,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let hash_bytes = match hex::decode(hash_str.trim_start_matches("0x")) {
        Ok(b) if b.len() == 32 => {
            let mut h = [0u8; 32];
            h.copy_from_slice(&b);
            h
        }
        _ => {
            return Ok(warp::reply::json(&BlockResponse {
                block: None,
                error: Some("Invalid hash: must be 32-byte hex string".into()),
            }))
        }
    };

    match chain_store.get_block(&hash_bytes) {
        Ok(Some(encoded)) => {
            let val: serde_json::Value =
                serde_json::from_slice(&encoded).unwrap_or(serde_json::Value::Null);
            Ok(warp::reply::json(&BlockResponse { block: Some(val), error: None }))
        }
        Ok(None) => Ok(warp::reply::json(&BlockResponse {
            block: None,
            error: Some(format!("Block not found: {}", hash_str)),
        })),
        Err(e) => Ok(warp::reply::json(&BlockResponse {
            block: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_get_balance(
    address_str: String,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let address_bytes = match hex::decode(address_str.trim_start_matches("0x")) {
        Ok(b) if b.len() == 20 => b,
        _ => {
            return Ok(warp::reply::json(&BalanceResponse {
                address: address_str,
                balance: "0".into(),
                nonce: 0,
            }))
        }
    };

    match chain_store.get_state(&address_bytes) {
        Ok(Some(encoded)) => {
            #[derive(serde::Deserialize)]
            struct AccountEncoded { balance: String, nonce: u64 }
            if let Ok(acc) = serde_json::from_slice::<AccountEncoded>(&encoded) {
                Ok(warp::reply::json(&BalanceResponse {
                    address: address_str,
                    balance: acc.balance,
                    nonce: acc.nonce,
                }))
            } else {
                Ok(warp::reply::json(&BalanceResponse {
                    address: address_str,
                    balance: "0".into(),
                    nonce: 0,
                }))
            }
        }
        _ => Ok(warp::reply::json(&BalanceResponse {
            address: address_str,
            balance: "0".into(),
            nonce: 0,
        })),
    }
}

async fn handle_get_receipt(
    tx_hash_str: String,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let hash_bytes = match hex::decode(tx_hash_str.trim_start_matches("0x")) {
        Ok(b) if b.len() == 32 => b,
        _ => {
            return Ok(warp::reply::json(&ReceiptResponse {
                receipt: None,
                error: Some("Invalid tx hash".into()),
            }))
        }
    };

    match chain_store.get_receipt(&hash_bytes) {
        Ok(Some(encoded)) => {
            let val: serde_json::Value =
                serde_json::from_slice(&encoded).unwrap_or(serde_json::Value::Null);
            Ok(warp::reply::json(&ReceiptResponse { receipt: Some(val), error: None }))
        }
        Ok(None) => Ok(warp::reply::json(&ReceiptResponse {
            receipt: None,
            error: Some(format!("Receipt not found: {}", tx_hash_str)),
        })),
        Err(e) => Ok(warp::reply::json(&ReceiptResponse {
            receipt: None,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Economic Handlers (Phase 9)
// ============================================================================

async fn handle_gas_price(
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let latest_height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    
    // Get the latest block to read actual base fee
    let (base_fee, low_priority, medium_priority, high_priority) = 
        if let Ok(Some(hash_bytes)) = chain_store.get_block_hash_by_height(latest_height) {
            if hash_bytes.len() == 32 {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&hash_bytes);
                if let Ok(Some(encoded)) = chain_store.get_block(&hash) {
                    if let Ok(block_val) = serde_json::from_slice::<serde_json::Value>(&encoded) {
                        let base_fee_val = block_val.get("header")
                            .and_then(|h| h.get("base_fee"))
                            .and_then(|b| b.as_u64())
                            .unwrap_or(1_000_000_000);
                        
                        // Priority fees based on recent blocks (simplified)
                        let low = base_fee_val;
                        let medium = base_fee_val * 2;
                        let high = base_fee_val * 5;
                        
                        (base_fee_val, low, medium, high)
                    } else {
                        (1_000_000_000u64, 1_000_000_000, 2_000_000_000, 5_000_000_000)
                    }
                } else {
                    (1_000_000_000u64, 1_000_000_000, 2_000_000_000, 5_000_000_000)
                }
            } else {
                (1_000_000_000u64, 1_000_000_000, 2_000_000_000, 5_000_000_000)
            }
        } else {
            (1_000_000_000u64, 1_000_000_000, 2_000_000_000, 5_000_000_000)
        };

    let response = GasPriceResponse {
        base_fee: base_fee.to_string(),
        suggested_priority_fee_low: low_priority.to_string(),
        suggested_priority_fee_medium: medium_priority.to_string(),
        suggested_priority_fee_high: high_priority.to_string(),
        block_height: latest_height,
    };
    
    Ok(warp::reply::json(&response))
}

async fn handle_estimate_gas(
    request: EstimateGasRequest,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    // Base transaction cost (21,000 gas)
    let base_cost = 21_000u64;
    
    // Calculate data cost (EIP-2028: 16 gas per non-zero byte, 4 gas per zero byte)
    let data_bytes = hex::decode(request.data.trim_start_matches("0x")).unwrap_or_default();
    let data_cost: u64 = data_bytes.iter()
        .map(|&byte| if byte == 0 { 4 } else { 16 })
        .sum();
    
    // Contract creation cost (if 'to' is empty or zero)
    let is_contract_creation = request.to == "0x" || request.to == "0x0000000000000000000000000000000000000000";
    let contract_cost = if is_contract_creation {
        32_000 // Contract creation base cost
    } else {
        0
    };
    
    // Estimated gas = base + data + contract cost + buffer (10% for safety)
    let estimated_gas = (base_cost + data_cost + contract_cost) * 110 / 100;
    
    // Get current base fee from latest block
    let latest_height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    let base_fee = if let Ok(Some(hash_bytes)) = chain_store.get_block_hash_by_height(latest_height) {
        if hash_bytes.len() == 32 {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&hash_bytes);
            if let Ok(Some(encoded)) = chain_store.get_block(&hash) {
                if let Ok(block_val) = serde_json::from_slice::<serde_json::Value>(&encoded) {
                    block_val.get("header")
                        .and_then(|h| h.get("base_fee"))
                        .and_then(|b| b.as_u64())
                        .unwrap_or(1_000_000_000)
                } else {
                    1_000_000_000
                }
            } else {
                1_000_000_000
            }
        } else {
            1_000_000_000
        }
    } else {
        1_000_000_000
    };
    
    // Parse max fee from request or use default
    let max_fee = request.max_fee_per_gas
        .as_ref()
        .and_then(|f| f.parse::<u128>().ok())
        .unwrap_or(1_000_000_000);
    
    let total_cost = estimated_gas as u128 * max_fee;
    let estimated_priority = (max_fee / 10) as u64; // 10% of max fee as priority
    
    let response = EstimateGasResponse {
        estimated_gas,
        base_fee: base_fee.to_string(),
        total_cost_estimate: total_cost.to_string(),
        estimated_priority_fee: estimated_priority.to_string(),
    };
    
    Ok(warp::reply::json(&response))
}

async fn handle_fee_history(
    block_count: u64,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let latest_height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    let oldest_block = latest_height.saturating_sub(block_count);
    
    let mut base_fees = Vec::new();
    let mut gas_used_ratio = Vec::new();
    
    for height in oldest_block..=latest_height {
        if let Ok(Some(hash_bytes)) = chain_store.get_block_hash_by_height(height) {
            if hash_bytes.len() == 32 {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&hash_bytes);
                if let Ok(Some(encoded)) = chain_store.get_block(&hash) {
                    if let Ok(block_val) = serde_json::from_slice::<serde_json::Value>(&encoded) {
                        let base_fee = block_val.get("header")
                            .and_then(|h| h.get("base_fee"))
                            .and_then(|b| b.as_u64())
                            .unwrap_or(1_000_000_000);
                        base_fees.push(base_fee.to_string());
                        
                        let gas_used = block_val.get("header")
                            .and_then(|h| h.get("gas_used"))
                            .and_then(|g| g.as_u64())
                            .unwrap_or(0);
                        let gas_limit = 30_000_000u64;
                        gas_used_ratio.push(gas_used as f64 / gas_limit as f64);
                    }
                }
            }
        }
    }
    
    let response = FeeHistoryResponse {
        base_fee_per_gas: base_fees,
        gas_used_ratio,
        oldest_block,
    };
    
    Ok(warp::reply::json(&response))
}

// ============================================================================
// Admin & Network Handlers
// ============================================================================

async fn handle_metrics(
    metrics: Arc<crate::metrics::Metrics>,
) -> Result<impl warp::Reply, Infallible> {
    Ok(warp::reply::with_header(
        metrics.export_prometheus(),
        "Content-Type",
        "text/plain; version=0.0.4",
    ))
}

async fn handle_health() -> Result<impl warp::Reply, Infallible> {
    #[derive(Serialize)]
    struct HealthResponse { 
        status: String,
        version: &'static str,
    }
    Ok(warp::reply::json(&HealthResponse { 
        status: "healthy".into(),
        version: env!("CARGO_PKG_VERSION"),
    }))
}

async fn handle_connect_peer(
    request: ConnectPeerRequest,
    network_cmd_sender: mpsc::Sender<NetworkCommand>,
) -> Result<impl warp::Reply, Infallible> {
    match request.multiaddr.parse::<libp2p::Multiaddr>() {
        Ok(addr) => {
            let _ = network_cmd_sender
                .send(NetworkCommand::Dial(addr))
                .await;
            Ok(warp::reply::json(&ConnectPeerResponse { 
                status: "success".into() 
            }))
        }
        Err(e) => Ok(warp::reply::json(&ConnectPeerResponse {
            status: format!("error: invalid multiaddr: {}", e),
        })),
    }
}

async fn handle_peers() -> Result<impl warp::Reply, Infallible> {
    // This would need access to the network service's peer list
    // For now, return an empty list
    #[derive(Serialize)]
    struct PeersResponse {
        peers: Vec<String>,
        count: usize,
    }
    Ok(warp::reply::json(&PeersResponse {
        peers: vec![],
        count: 0,
    }))
}

// ============================================================================
// Error Handling
// ============================================================================

async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, Infallible> {
    let (status, message) = if err.is_not_found() {
        (warp::http::StatusCode::NOT_FOUND, "Endpoint not found".to_string())
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        (warp::http::StatusCode::METHOD_NOT_ALLOWED, "Method not allowed".to_string())
    } else if err.find::<warp::reject::PayloadTooLarge>().is_some() {
        (warp::http::StatusCode::PAYLOAD_TOO_LARGE, "Request body too large (max 1MB)".to_string())
    } else if err.find::<warp::reject::InvalidQuery>().is_some() {
        (warp::http::StatusCode::BAD_REQUEST, "Invalid query parameters".to_string())
    } else {
        (warp::http::StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded".to_string())
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&ErrorResponse { error: message }),
        status,
    ))
}