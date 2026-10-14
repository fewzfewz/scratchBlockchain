//! Account abstraction, MEV protection and slashing routes.

use super::*;

pub(super) async fn handle_submit_user_operation(
    op: UserOperation,
    tx_pool: Arc<TxPool>,
) -> Result<impl warp::Reply, Infallible> {
    match tx_pool.submit_user_operation(op) {
        Ok(hash) => Ok(warp::reply::json(&SubmitTxResponse {
            status: "success".into(),
            hash: hex::encode(hash),
        })),
        Err(e) => Ok(warp::reply::json(&SubmitTxResponse {
            status: format!("error: {}", e),
            hash: String::new(),
        })),
    }
}

pub(super) async fn handle_pending_user_operations(
    tx_pool: Arc<TxPool>,
) -> Result<impl warp::Reply, Infallible> {
    Ok(warp::reply::json(&serde_json::json!({
        "pending": tx_pool.pending_user_operations(),
    })))
}

pub(super) async fn handle_mev_commit(
    req: MevCommitRequest,
    tx_pool: Arc<TxPool>,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let Some(tx_hash) = parse_hex32(&req.tx_hash) else {
        return Ok(warp::reply::json(
            &serde_json::json!({"error": "Invalid tx_hash"}),
        ));
    };
    let Some(secret) = parse_hex32(&req.secret) else {
        return Ok(warp::reply::json(
            &serde_json::json!({"error": "Invalid secret"}),
        ));
    };
    let Some(sender) = parse_address(&req.sender) else {
        return Ok(warp::reply::json(
            &serde_json::json!({"error": "Invalid sender"}),
        ));
    };
    let height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    let commitment = tx_pool.submit_committed(tx_hash, secret, sender, req.nonce, height);
    Ok(warp::reply::json(&serde_json::json!({
        "status": "success",
        "commitment": hex::encode(commitment),
    })))
}

pub(super) async fn handle_mev_reveal(
    req: MevRevealRequest,
    tx_pool: Arc<TxPool>,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let Some(secret) = parse_hex32(&req.secret) else {
        return Ok(warp::reply::json(
            &serde_json::json!({"error": "Invalid secret"}),
        ));
    };
    let Some(commitment) = parse_hex32(&req.commitment) else {
        return Ok(warp::reply::json(
            &serde_json::json!({"error": "Invalid commitment"}),
        ));
    };
    let height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    match tx_pool.reveal_transaction(req.transaction, secret, commitment, height) {
        Ok(tx) => Ok(warp::reply::json(&SubmitTxResponse {
            status: "success".into(),
            hash: hex::encode(tx.hash()),
        })),
        Err(e) => Ok(warp::reply::json(&SubmitTxResponse {
            status: format!("error: {}", e),
            hash: String::new(),
        })),
    }
}

pub(super) async fn handle_mev_encrypted(
    enc: mev::EncryptedTransaction,
    tx_pool: Arc<TxPool>,
) -> Result<impl warp::Reply, Infallible> {
    match tx_pool.submit_encrypted(enc) {
        Ok(()) => Ok(warp::reply::json(&serde_json::json!({"status": "success"}))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({"error": e}))),
    }
}

pub(super) async fn handle_mev_decryption_share(
    share: mev::DecryptionShare,
    tx_pool: Arc<TxPool>,
) -> Result<impl warp::Reply, Infallible> {
    match tx_pool.submit_decryption_share(share) {
        Ok(()) => Ok(warp::reply::json(&serde_json::json!({"status": "success"}))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({"error": e}))),
    }
}

pub(super) async fn handle_slashing_events(
    tracker: Arc<Mutex<SlashingTracker>>,
) -> Result<impl warp::Reply, Infallible> {
    let slashed: Vec<String> = tracker
        .lock()
        .await
        .get_slashed_validators()
        .iter()
        .map(hex::encode)
        .collect();
    Ok(warp::reply::json(&serde_json::json!({
        "slashed_validators": slashed,
        "count": slashed.len(),
    })))
}
