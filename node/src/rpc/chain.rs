//! Chain reads (status, blocks, balances, receipts), transaction submission and network routes.

use super::*;

pub(super) async fn handle_status(
    chain_store: Arc<ChainStore>,
    tx_pool: Arc<TxPool>,
    chain_id: u64,
) -> Result<impl warp::Reply, Infallible> {
    let height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    let finalized_height = Some(height);
    let mempool_size = tx_pool.size();
    let peer_count = 0; // Would need network service connection for this

    Ok(warp::reply::json(&StatusResponse {
        height,
        finalized_height,
        mempool_size,
        peer_count,
        chain_id,
    }))
}

pub(super) async fn handle_mempool(tx_pool: Arc<TxPool>) -> Result<impl warp::Reply, Infallible> {
    let transactions = tx_pool.mempool_snapshot(100);
    Ok(warp::reply::json(&MempoolResponse {
        size: tx_pool.size(),
        transactions,
    }))
}

pub(super) async fn handle_submit_tx(
    tx: Transaction,
    tx_pool: Arc<TxPool>,
    network_cmd_sender: mpsc::Sender<NetworkCommand>,
    chain_id: u64,
) -> Result<impl warp::Reply, Infallible> {
    // Reject unauthenticated txs up front with a clear message (the mempool
    // checks again, and so does block execution).
    if let Err(e) = common::validation::verify_tx_signature(&tx, chain_id) {
        return Ok(warp::reply::json(&SubmitTxResponse {
            status: format!("error: {}", e),
            hash: String::new(),
        }));
    }

    match tx_pool.add_transaction(tx.clone()) {
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
        Err(e) => {
            let msg = e.to_string();
            let hash = if msg.contains("already in mempool") {
                hex::encode(tx.hash())
            } else {
                String::new()
            };
            Ok(warp::reply::json(&SubmitTxResponse {
                status: format!("error: {}", msg),
                hash,
            }))
        }
    }
}

pub(super) async fn handle_get_block_by_height(
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
                    Ok(warp::reply::json(&BlockResponse {
                        block: Some(val),
                        error: None,
                    }))
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

pub(super) async fn handle_get_block_by_hash(
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
            Ok(warp::reply::json(&BlockResponse {
                block: Some(val),
                error: None,
            }))
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

pub(super) async fn handle_get_balance(
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
            struct AccountEncoded {
                balance: String,
                nonce: u64,
            }
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

pub(super) async fn handle_get_receipt(
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
            Ok(warp::reply::json(&ReceiptResponse {
                receipt: Some(val),
                error: None,
            }))
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

pub(super) fn parse_address_20(s: &str) -> Option<[u8; 20]> {
    let bytes = hex::decode(s.trim_start_matches("0x")).ok()?;
    if bytes.len() != 20 {
        return None;
    }
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&bytes);
    Some(addr)
}

pub(super) fn address_hex(addr: &[u8; 20]) -> String {
    format!("0x{}", hex::encode(addr))
}

pub(super) async fn handle_txs_for_address(
    address_str: String,
    query: TxHistoryQuery,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let query_addr = match parse_address_20(&address_str) {
        Some(a) => a,
        None => {
            return Ok(warp::reply::json(&TxHistoryResponse {
                address: address_str,
                transactions: vec![],
                scanned_blocks: 0,
            }))
        }
    };

    let limit = query.limit.unwrap_or(25).min(100);
    let latest = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    let max_scan = 100u64;
    let start = latest.saturating_sub(max_scan.saturating_sub(1));
    let mut found = Vec::new();
    let mut scanned = 0u64;

    for height in (start..=latest).rev() {
        if found.len() >= limit {
            break;
        }
        scanned += 1;
        let Ok(Some(encoded)) = chain_store.get_block_by_height(height) else {
            continue;
        };
        let Ok(block) = serde_json::from_slice::<Block>(&encoded) else {
            continue;
        };

        for tx in &block.extrinsics {
            let matches_sender = tx.sender == query_addr;
            let matches_to = tx.to.map(|t| t == query_addr).unwrap_or(false);
            if !matches_sender && !matches_to {
                continue;
            }

            let hash = tx.hash();
            let status = chain_store
                .get_receipt(&hash)
                .ok()
                .flatten()
                .and_then(|encoded| serde_json::from_slice::<serde_json::Value>(&encoded).ok())
                .map(|receipt| {
                    if receipt
                        .get("success")
                        .and_then(|s| s.as_bool())
                        .unwrap_or(false)
                    {
                        "confirmed".to_string()
                    } else {
                        "failed".to_string()
                    }
                });

            found.push(TxHistoryEntry {
                hash: format!("0x{}", hex::encode(hash)),
                block_height: height,
                sender: address_hex(&tx.sender),
                to: tx.to.map(|t| address_hex(&t)),
                value: tx.value,
                nonce: tx.nonce,
                is_contract_creation: tx.to.is_none() && !tx.payload.is_empty(),
                status,
            });
        }
    }

    Ok(warp::reply::json(&TxHistoryResponse {
        address: address_str,
        transactions: found,
        scanned_blocks: scanned,
    }))
}

pub(super) async fn handle_metrics(
    metrics: Arc<crate::metrics::Metrics>,
) -> Result<impl warp::Reply, Infallible> {
    Ok(warp::reply::with_header(
        metrics.export_prometheus(),
        "Content-Type",
        "text/plain; version=0.0.4",
    ))
}

pub(super) async fn handle_health() -> Result<impl warp::Reply, Infallible> {
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

pub(super) async fn handle_connect_peer(
    request: ConnectPeerRequest,
    network_cmd_sender: mpsc::Sender<NetworkCommand>,
) -> Result<impl warp::Reply, Infallible> {
    match request.multiaddr.parse::<libp2p::Multiaddr>() {
        Ok(addr) => {
            let _ = network_cmd_sender.send(NetworkCommand::Dial(addr)).await;
            Ok(warp::reply::json(&ConnectPeerResponse {
                status: "success".into(),
            }))
        }
        Err(e) => Ok(warp::reply::json(&ConnectPeerResponse {
            status: format!("error: invalid multiaddr: {}", e),
        })),
    }
}

pub(super) async fn handle_peers() -> Result<impl warp::Reply, Infallible> {
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

pub(super) async fn handle_block_latest(
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let latest = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    if latest == 0 {
        return Ok(warp::reply::json(&BlockResponse {
            block: None,
            error: Some("No blocks have been produced yet".into()),
        }));
    }
    match chain_store.get_block_hash_by_height(latest) {
        Ok(Some(hash_bytes)) if hash_bytes.len() == 32 => {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&hash_bytes);
            match chain_store.get_block(&hash) {
                Ok(Some(encoded)) => {
                    let val: serde_json::Value =
                        serde_json::from_slice(&encoded).unwrap_or(serde_json::Value::Null);
                    Ok(warp::reply::json(&BlockResponse {
                        block: Some(val),
                        error: None,
                    }))
                }
                Ok(None) => Ok(warp::reply::json(&BlockResponse {
                    block: None,
                    error: Some(format!("Block data missing for height {}", latest)),
                })),
                Err(e) => Ok(warp::reply::json(&BlockResponse {
                    block: None,
                    error: Some(e.to_string()),
                })),
            }
        }
        Ok(_) => Ok(warp::reply::json(&BlockResponse {
            block: None,
            error: Some(format!("No block at height {}", latest)),
        })),
        Err(e) => Ok(warp::reply::json(&BlockResponse {
            block: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(super) fn parse_hex32(s: &str) -> Option<[u8; 32]> {
    let bytes = hex::decode(s.trim_start_matches("0x")).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Some(out)
}

pub(super) fn parse_address(s: &str) -> Option<[u8; 20]> {
    let bytes = hex::decode(s.trim_start_matches("0x")).ok()?;
    if bytes.len() != 20 {
        return None;
    }
    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes);
    Some(out)
}

pub(super) async fn handle_websocket(ws: warp::ws::WebSocket, chain_store: Arc<ChainStore>) {
    use futures_util::{SinkExt, StreamExt};
    use std::time::Duration;

    let (mut tx, mut rx) = ws.split();

    loop {
        let height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
        let msg = warp::ws::Message::text(format!(r#"{{"event":"newHead","height":{}}}"#, height));
        if tx.send(msg).await.is_err() {
            break;
        }
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(2)) => {}
            incoming = rx.next() => {
                if incoming.is_none() { break; }
            }
        }
    }
}
