use common::types::Transaction;
use consensus::FinalityGadget;
use mempool::Mempool;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use storage::BlockStore;
use tokio::sync::Mutex;
use warp::Filter;

#[derive(Debug, Serialize)]
struct StatusResponse {
    height: u64,
    finalized_height: Option<u64>,
    mempool_size: usize,
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

use network::NetworkCommand;
use tokio::sync::mpsc;

#[derive(Debug, Serialize)]
struct BlockResponse {
    block: Option<common::types::Block>,
}

#[derive(Debug, Serialize)]
struct BalanceResponse {
    address: String,
    balance: String,
    nonce: u64,
}

#[derive(Debug, Serialize)]
struct ReceiptResponse {
    receipt: Option<common::types::TransactionReceipt>,
}

pub struct RpcServer {
    mempool: Arc<Mempool>,
    block_store: Arc<BlockStore>,
    state_store: Arc<storage::StateStore>,
    receipt_store: Arc<storage::receipt_store::ReceiptStore>,
    finality_gadget: Arc<Mutex<FinalityGadget>>,
    metrics: Arc<crate::metrics::Metrics>,
    network_cmd_sender: mpsc::Sender<NetworkCommand>,
}

impl RpcServer {
    pub fn new(
        mempool: Arc<Mempool>,
        block_store: Arc<BlockStore>,
        state_store: Arc<storage::StateStore>,
        receipt_store: Arc<storage::receipt_store::ReceiptStore>,
        finality_gadget: Arc<Mutex<FinalityGadget>>,
        metrics: Arc<crate::metrics::Metrics>,
        network_cmd_sender: mpsc::Sender<NetworkCommand>,
    ) -> Self {
        Self {
            mempool,
            block_store,
            state_store,
            receipt_store,
            finality_gadget,
            metrics,
            network_cmd_sender,
        }
    }

    pub async fn run(&self, port: u16, tls_config: Option<(String, String)>) {
        use governor::{Quota, RateLimiter};
        use governor::clock::DefaultClock;
        use governor::state::keyed::DefaultKeyedStateStore;
        use std::num::NonZeroU32;
        use std::net::IpAddr;

        let mempool = self.mempool.clone();
        let block_store = self.block_store.clone();
        let state_store = self.state_store.clone();
        let receipt_store = self.receipt_store.clone();
        let finality_gadget = self.finality_gadget.clone();
        let metrics = self.metrics.clone();
        let network_cmd_sender = self.network_cmd_sender.clone();

        // Rate limiter: 100 requests per second per IP
        let rate_limiter = Arc::new(RateLimiter::<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>::keyed(
            Quota::per_second(NonZeroU32::new(100).unwrap()),
        ));

        let with_rate_limit = warp::any()
            .map(move || rate_limiter.clone())
            .and(warp::addr::remote())
            .and_then(|limiter: Arc<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>, addr: Option<std::net::SocketAddr>| async move {
                if let Some(addr) = addr {
                    if let Err(_) = limiter.check_key(&addr.ip()) {
                        return Err(warp::reject::reject());
                    }
                }
                Ok(())
            });

        // Request validation middleware
        let with_validation = warp::body::content_length_limit(1024 * 1024); // 1MB limit


        // GET /status
        let status = warp::path("status")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_state(block_store.clone()))
            .and(with_state(mempool.clone()))
            .and(with_state(finality_gadget.clone()))
            .and_then(|_, block_store, mempool, finality_gadget| handle_status(block_store, mempool, finality_gadget));

        // GET /mempool
        let mempool_route = warp::path("mempool")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_state(mempool.clone()))
            .and_then(|_, mempool| handle_mempool(mempool));

        // POST /submit_tx
        let submit_tx = warp::path("submit_tx")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_state(mempool.clone()))
            .and(with_state(network_cmd_sender.clone()))
            .and_then(|tx, mempool, sender| handle_submit_tx(tx, mempool, sender));

        // GET /block/:height
        let block_by_height = warp::path!("block" / u64)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_state(block_store.clone()))
            .and_then(|height, _, block_store| handle_get_block_by_height(height, block_store));

        // GET /block/hash/:hash
        let block_by_hash = warp::path!("block" / "hash" / String)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_state(block_store.clone()))
            .and_then(|hash, _, block_store| handle_get_block_by_hash(hash, block_store));

        // GET /balance/:address
        let balance = warp::path!("balance" / String)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_state(state_store.clone()))
            .and_then(|address, _, state_store| handle_get_balance(address, state_store));

        // GET /tx/:hash - Get transaction receipt
        let tx_receipt = warp::path!("tx" / String)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_state(receipt_store.clone()))
            .and_then(|hash, _, receipt_store| handle_get_receipt(hash, receipt_store));

        // GET /metrics (Prometheus format)
        let metrics_route = warp::path("metrics")
            .and(warp::get())
            .and(with_state(metrics.clone()))
            .and_then(handle_metrics);

        // GET /health
        let health = warp::path("health")
            .and(warp::get())
            .and_then(handle_health);

        let routes = status
            .or(mempool_route)
            .or(submit_tx)
            .or(block_by_height)
            .or(block_by_hash)
            .or(balance)
            .or(tx_receipt)
            .or(metrics_route)
            .or(health);

        // Note: TLS support requires additional setup with warp-tls crate
        // For now, running without TLS
        println!("RPC server starting on port {}", port);
        warp::serve(routes).run(([0, 0, 0, 0], port)).await;
    }
}

fn with_state<T: Clone + Send>(
    state: T,
) -> impl Filter<Extract = (T,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn handle_status(
    block_store: Arc<BlockStore>,
    mempool: Arc<Mempool>,
    _finality_gadget: Arc<Mutex<FinalityGadget>>,
) -> Result<impl warp::Reply, Infallible> {
    let height = block_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    let finalized_height = block_store.get_latest_finalized_height().unwrap_or(None);
    let mempool_size = mempool.size();

    let response = StatusResponse {
        height,
        finalized_height,
        mempool_size,
    };

    Ok(warp::reply::json(&response))
}

async fn handle_mempool(mempool: Arc<Mempool>) -> Result<impl warp::Reply, Infallible> {
    let transactions = mempool.get_transactions(100); // Limit to 100 for now
    let response = MempoolResponse {
        size: mempool.size(),
        transactions,
    };
    Ok(warp::reply::json(&response))
}

async fn handle_submit_tx(
    tx: Transaction,
    mempool: Arc<Mempool>,
    network_cmd_sender: mpsc::Sender<NetworkCommand>,
) -> Result<impl warp::Reply, Infallible> {
    match mempool.add_transaction(tx.clone()) {
        Ok(_) => {
            // Broadcast transaction to network
            let _ = network_cmd_sender.send(NetworkCommand::BroadcastTransaction(tx.clone())).await;
            
            let response = SubmitTxResponse {
                status: "success".to_string(),
                hash: hex::encode(tx.hash()),
            };
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = SubmitTxResponse {
                status: format!("error: {}", e),
                hash: "".to_string(),
            };
            Ok(warp::reply::json(&response))
        }
    }
}

async fn handle_metrics(
    metrics: Arc<crate::metrics::Metrics>,
) -> Result<impl warp::Reply, Infallible> {
    let prometheus_output = metrics.export_prometheus();
    Ok(warp::reply::with_header(
        prometheus_output,
        "Content-Type",
        "text/plain; version=0.0.4",
    ))
}

async fn handle_health() -> Result<impl warp::Reply, Infallible> {
    #[derive(Serialize)]
    struct HealthResponse {
        status: String,
    }
    
    let response = HealthResponse {
        status: "healthy".to_string(),
    };
    Ok(warp::reply::json(&response))
}

async fn handle_get_block_by_height(
    height: u64,
    block_store: Arc<BlockStore>,
) -> Result<impl warp::Reply, Infallible> {
    match block_store.get_block_by_height(height) {
        Ok(block) => {
            let response = BlockResponse { block };
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = BlockResponse { block: None };
            tracing::warn!("Failed to get block by height {}: {}", height, e);
            Ok(warp::reply::json(&response))
        }
    }
}

async fn handle_get_block_by_hash(
    hash_str: String,
    block_store: Arc<BlockStore>,
) -> Result<impl warp::Reply, Infallible> {
    // Parse hex hash
    let hash_bytes = match hex::decode(&hash_str) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&bytes);
            hash
        }
        _ => {
            let response = BlockResponse { block: None };
            return Ok(warp::reply::json(&response));
        }
    };

    match block_store.get_block_by_hash(&hash_bytes) {
        Ok(block) => {
            let response = BlockResponse { block };
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = BlockResponse { block: None };
            tracing::warn!("Failed to get block by hash {}: {}", hash_str, e);
            Ok(warp::reply::json(&response))
        }
    }
}

async fn handle_get_balance(
    address_str: String,
    state_store: Arc<storage::StateStore>,
) -> Result<impl warp::Reply, Infallible> {
    // Parse hex address
    let address_bytes = match hex::decode(&address_str) {
        Ok(bytes) if bytes.len() == 20 => {
            let mut address = [0u8; 20];
            address.copy_from_slice(&bytes);
            address
        }
        _ => {
            let response = BalanceResponse {
                address: address_str,
                balance: "0".to_string(),
                nonce: 0,
            };
            return Ok(warp::reply::json(&response));
        }
    };

    match state_store.get_account(&address_bytes) {
        Ok(Some(account)) => {
            let response = BalanceResponse {
                address: address_str,
                balance: account.balance.to_string(),
                nonce: account.nonce,
            };
            Ok(warp::reply::json(&response))
        }
        Ok(None) => {
            let response = BalanceResponse {
                address: address_str,
                balance: "0".to_string(),
                nonce: 0,
            };
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            tracing::warn!("Failed to get balance for {}: {}", address_str, e);
            let response = BalanceResponse {
                address: address_str,
                balance: "0".to_string(),
                nonce: 0,
            };
            Ok(warp::reply::json(&response))
        }
    }
}

async fn handle_get_receipt(
    tx_hash_str: String,
    receipt_store: Arc<storage::receipt_store::ReceiptStore>,
) -> Result<impl warp::Reply, Infallible> {
    // Parse hex hash
    let tx_hash_bytes = match hex::decode(&tx_hash_str) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&bytes);
            hash
        }
        _ => {
            let response = ReceiptResponse { receipt: None };
            return Ok(warp::reply::json(&response));
        }
    };

    match receipt_store.get_receipt(&tx_hash_bytes) {
        Ok(receipt) => {
            let response = ReceiptResponse { receipt };
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            tracing::warn!("Failed to get receipt for {}: {}", tx_hash_str, e);
            let response = ReceiptResponse { receipt: None };
            Ok(warp::reply::json(&response))
        }
    }
}
