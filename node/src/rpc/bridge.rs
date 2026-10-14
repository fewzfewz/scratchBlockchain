//! Bridge relayer routes (dev mode).

use super::*;

pub(super) fn deterministic_address(label: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(label.as_bytes());
    format!("0x{}", hex::encode(&h.finalize()[..20]))
}

pub(super) async fn handle_bridge_status(
    state_trie: Arc<Mutex<PatriciaTrie>>,
    bridge_minted: Arc<Mutex<HashSet<String>>>,
    bridge_unlock_queue: Arc<Mutex<BridgeUnlockQueue>>,
    chain_id: u64,
) -> Result<impl warp::Reply, Infallible> {
    let validators_count = {
        let trie = state_trie.lock().await;
        trie.get(b"validators")
            .ok()
            .flatten()
            .and_then(|encoded| serde_json::from_slice::<Vec<ValidatorInfo>>(&encoded).ok())
            .map(|v| v.len())
            .unwrap_or(0)
    };
    let processed_mints = bridge_minted.lock().await.len();
    // Mints need the genesis relayer set; validators are not relayers.
    let config = crate::bridge_mint::load_config(&state_trie).await;
    let relayer_count = config.as_ref().map(|c| c.relayers.len()).unwrap_or(0);
    let relayer_threshold = config.as_ref().map(|c| c.threshold).unwrap_or(0);
    let relayers_ready = relayer_threshold > 0 && relayer_count > 0;
    let queue = bridge_unlock_queue.lock().await;
    let processed_unlocks = queue.processed_count();
    let pending_unlocks = queue.pending().len();
    let eth_rpc_configured = std::env::var("ETH_RPC_URL").is_ok();

    Ok(warp::reply::json(&BridgeStatusResponse {
        vault_address: deterministic_address("nebula-bridge-vault-v1"),
        defi_pool_address: deterministic_address("nebula-defi-pool-v1"),
        validators_count,
        relayers_ready,
        relayer_count,
        relayer_threshold,
        eth_rpc_configured,
        processed_mints,
        processed_unlocks,
        pending_unlocks,
        chain_id,
    }))
}

pub(super) async fn handle_bridge_pending_unlocks(
    bridge_unlock_queue: Arc<Mutex<BridgeUnlockQueue>>,
) -> Result<impl warp::Reply, Infallible> {
    let queue = bridge_unlock_queue.lock().await;
    Ok(warp::reply::json(&serde_json::json!({
        "pending": queue.pending(),
        "count": queue.pending().len(),
    })))
}

pub(super) async fn handle_bridge_unlock_ack(
    req: BridgeUnlockAckRequest,
    bridge_unlock_queue: Arc<Mutex<BridgeUnlockQueue>>,
) -> Result<impl warp::Reply, Infallible> {
    let hash_clean = req
        .nebula_tx_hash
        .trim()
        .trim_start_matches("0x")
        .to_lowercase();
    let mut queue = bridge_unlock_queue.lock().await;
    if queue.ack(&hash_clean) {
        Ok(warp::reply::json(&serde_json::json!({
            "status": "acknowledged",
            "nebula_tx_hash": format!("0x{}", hash_clean),
            "eth_tx_hash": req.eth_tx_hash,
        })))
    } else {
        Ok(warp::reply::json(&serde_json::json!({
            "error": "Pending unlock not found",
        })))
    }
}

pub(super) use common::bridge::EthLock;
#[cfg(test)]
use common::bridge::TOKENS_LOCKED_TOPIC;

/// Find the native-ETH `TokensLocked` event emitted by `bridge` in a receipt.
/// The amount and recipient a relayer attests to come from here, never from
/// the mint request.
pub(super) fn parse_eth_lock(receipt: &serde_json::Value, bridge: &str) -> Result<EthLock, String> {
    if receipt.get("status").and_then(|s| s.as_str()) != Some("0x1") {
        return Err("ETH transaction failed".into());
    }
    let logs = receipt
        .get("logs")
        .and_then(|l| l.as_array())
        .ok_or("ETH receipt has no logs")?;
    for log in logs {
        if let Some(lock) = common::bridge::parse_tokens_locked_log(log, bridge)? {
            return Ok(lock);
        }
    }
    Err("no TokensLocked event from the bridge contract in this transaction".into())
}

pub(super) async fn verify_eth_lock_tx(
    eth_rpc: &str,
    tx_hash: &str,
    bridge_address: &str,
) -> Result<EthLock, String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getTransactionReceipt",
        "params": [tx_hash],
        "id": 1
    });
    let client = reqwest::Client::new();
    let resp = client
        .post(eth_rpc)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("ETH RPC unreachable: {e}"))?;
    let val: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Invalid ETH RPC response: {e}"))?;
    let receipt = val
        .get("result")
        .ok_or_else(|| "Missing result in ETH RPC response".to_string())?;
    if receipt.is_null() {
        return Err("ETH transaction not yet mined".into());
    }
    parse_eth_lock(receipt, bridge_address)
}

/// Dev-mode relayer: verifies the ETH lock against the node's own ETH
/// endpoint, attests to it with the dev key (the devnet genesis lists it as a
/// 1-of-1 relayer) and submits a `bridge_mint` transaction. The mint itself
/// happens in consensus, which checks the attestations against the genesis
/// relayer set and records the ETH tx hash so it can never be minted twice.
/// With a multi-relayer set, relayers sign `bridge_mint::attestation_message`
/// themselves and anyone submits the combined transaction.
pub(super) async fn handle_bridge_mint(
    req: BridgeMintRequest,
    chain_store: Arc<ChainStore>,
    state_trie: Arc<Mutex<PatriciaTrie>>,
    signer: Option<Arc<DevSigner>>,
    bridge_minted: Arc<Mutex<HashSet<String>>>,
    attestation_pool: Arc<Mutex<super::attestations::AttestationPool>>,
) -> Result<impl warp::Reply, Infallible> {
    let Some(signer) = signer else {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": "Bridge mint disabled: set api.dev_funding_key (NODE_API_DEV_FUNDING_KEY)",
        })));
    };
    let recipient = req.recipient.trim().to_lowercase();
    let Some(recipient_bytes) = parse_address(&recipient) else {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": "Invalid recipient address",
        })));
    };
    let amount: u128 = req
        .amount
        .as_ref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000_000_000_000_000_000u128);
    if amount == 0 {
        return Ok(warp::reply::json(
            &serde_json::json!({ "error": "Amount must be positive" }),
        ));
    }

    // Every mint must be backed by an Ethereum lock tx; there is no free mint.
    let Some(eth_hash) = req.eth_tx_hash.as_deref() else {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": "eth_tx_hash is required",
        })));
    };
    let eth_hash_lower = eth_hash.trim().to_lowercase();
    let Some(eth_hash_bytes) = hex::decode(eth_hash_lower.trim_start_matches("0x"))
        .ok()
        .and_then(|b| <[u8; 32]>::try_from(b).ok())
    else {
        return Ok(warp::reply::json(
            &serde_json::json!({ "error": "Invalid eth_tx_hash" }),
        ));
    };

    // Hold the lock across verify + submit so two concurrent requests for the
    // same ETH tx cannot both submit (consensus would reject the second).
    let mut pending = bridge_minted.lock().await;
    if crate::bridge_mint::already_minted(&chain_store, &eth_hash_bytes)
        || pending.contains(&eth_hash_lower)
    {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": "ETH lock already processed",
        })));
    }

    // Only the node's own ETH endpoint is trusted: a caller-supplied URL
    // could point at a fake node that reports any tx as a successful lock.
    if req.eth_rpc_url.is_some() {
        tracing::debug!("Ignoring caller-supplied eth_rpc_url for bridge mint");
    }
    let eth_rpc = std::env::var("ETH_RPC_URL").unwrap_or_default();
    if eth_rpc.is_empty() {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": "ETH_RPC_URL not configured on the node",
        })));
    }
    // The bridge contract address is node configuration too: a caller-chosen
    // contract could emit a fake TokensLocked event.
    if req.eth_bridge_address.is_some() {
        tracing::debug!("Ignoring caller-supplied eth_bridge_address for bridge mint");
    }
    let Ok(bridge_address) = std::env::var("ETH_BRIDGE_ADDRESS") else {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": "ETH_BRIDGE_ADDRESS not configured on the node",
        })));
    };
    let lock = match verify_eth_lock_tx(&eth_rpc, &eth_hash_lower, &bridge_address).await {
        Ok(lock) => lock,
        Err(e) => return Ok(warp::reply::json(&serde_json::json!({ "error": e }))),
    };
    if lock.recipient != recipient_bytes || req.amount.is_some() && lock.amount != amount {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": "recipient/amount do not match the Ethereum lock",
            "locked_amount": lock.amount.to_string(),
            "locked_recipient": format!("0x{}", hex::encode(lock.recipient)),
        })));
    }
    let amount = lock.amount;

    // This node is one relayer of the set: pool its attestation with the
    // others', and submit the mint once the threshold is met (otherwise the
    // standalone relayers complete it).
    let sub = super::attestations::AttestationSubmission {
        eth_tx_hash: format!("0x{}", hex::encode(eth_hash_bytes)),
        recipient: format!("0x{}", hex::encode(recipient_bytes)),
        amount: amount.to_string(),
        attestation: common::bridge::attest(
            &signer.key,
            signer.chain_id,
            &eth_hash_bytes,
            &recipient_bytes,
            amount,
        ),
    };
    let (count, config, minted) = match super::attestations::add_attestation(
        &sub,
        &state_trie,
        &chain_store,
        &attestation_pool,
        signer.chain_id,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return Ok(warp::reply::json(&serde_json::json!({ "error": e }))),
    };
    if minted {
        return Ok(warp::reply::json(
            &serde_json::json!({ "error": "ETH lock already processed" }),
        ));
    }
    if (count as u64) < u64::from(config.threshold) {
        return Ok(warp::reply::json(&serde_json::json!({
            "status": "attested",
            "attestations": count,
            "threshold": config.threshold,
            "note": "Waiting for the other relayers to attest; one of them submits the mint",
            "eth_tx_hash": sub.eth_tx_hash,
        })));
    }
    let attestations: Vec<common::bridge::Attestation> = attestation_pool
        .lock()
        .await
        .get(&hex::encode(eth_hash_bytes))
        .into_iter()
        .find(|e| e.recipient == recipient_bytes && e.amount == amount)
        .map(|e| e.attestations.into_values().collect())
        .unwrap_or_default();
    let payload = common::bridge::BridgeMint {
        eth_tx_hash: sub.eth_tx_hash.clone(),
        recipient: sub.recipient.clone(),
        amount: sub.amount.clone(),
        attestations,
    }
    .to_payload();
    match signer.submit(None, 0, payload).await {
        Ok(hash) => {
            pending.insert(eth_hash_lower);
            Ok(warp::reply::json(&serde_json::json!({
                "status": "submitted",
                "recipient": sub.recipient,
                "amount": sub.amount,
                "hash": hash,
                "eth_tx_hash": req.eth_tx_hash,
            })))
        }
        Err(e) => Ok(warp::reply::json(&serde_json::json!({
            "error": format!("Mint transaction failed: {}", e),
        }))),
    }
}

pub(super) fn find_tx_by_hash(
    chain_store: &ChainStore,
    tx_hash: &[u8; 32],
) -> Option<(Transaction, u64)> {
    let latest = chain_store.get_latest_height().ok().flatten().unwrap_or(0);
    let start = latest.saturating_sub(500);
    for height in (start..=latest).rev() {
        let Ok(Some(encoded)) = chain_store.get_block_by_height(height) else {
            continue;
        };
        let Ok(block) = serde_json::from_slice::<Block>(&encoded) else {
            continue;
        };
        for tx in block.extrinsics {
            if tx.hash() == *tx_hash {
                return Some((tx, height));
            }
        }
    }
    None
}

pub(super) fn verify_nebula_lock_tx(
    chain_store: &ChainStore,
    tx_hash: &[u8; 32],
    vault_address: &str,
) -> Result<(Transaction, u64), String> {
    let vault = parse_address_20(vault_address).ok_or("Invalid vault address")?;
    let (tx, height) = find_tx_by_hash(chain_store, tx_hash)
        .ok_or_else(|| "Nebula lock transaction not found".to_string())?;

    let to = tx.to.ok_or("Lock transaction missing destination")?;
    if to != vault {
        return Err("Transaction not sent to bridge vault".into());
    }
    if tx.value == 0 {
        return Err("Lock transaction has zero value".into());
    }

    let body = tx.payload.as_slice();
    let payload_str = std::str::from_utf8(body).map_err(|_| "Invalid bridge payload encoding")?;
    if !payload_str.starts_with("BRIDGE:ethereum:") {
        return Err(format!(
            "Expected BRIDGE:ethereum: payload, got {}",
            payload_str.chars().take(40).collect::<String>()
        ));
    }

    let receipt = chain_store
        .get_receipt(tx_hash)
        .map_err(|e| e.to_string())?
        .ok_or("Lock transaction receipt not found")?;
    let val: serde_json::Value =
        serde_json::from_slice(&receipt).map_err(|e| format!("Invalid receipt: {e}"))?;
    if !val
        .get("success")
        .and_then(|s| s.as_bool())
        .unwrap_or(false)
    {
        return Err("Lock transaction failed on-chain".into());
    }

    Ok((tx, height))
}

pub(super) async fn handle_bridge_unlock(
    req: BridgeUnlockRequest,
    chain_store: Arc<ChainStore>,
    bridge_unlock_queue: Arc<Mutex<BridgeUnlockQueue>>,
) -> Result<impl warp::Reply, Infallible> {
    let hash_clean = req
        .nebula_tx_hash
        .trim()
        .trim_start_matches("0x")
        .to_lowercase();
    let hash_bytes = match hex::decode(&hash_clean) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => {
            return Ok(warp::reply::json(&serde_json::json!({
                "error": "Invalid nebula_tx_hash",
            })))
        }
    };

    {
        let queue = bridge_unlock_queue.lock().await;
        if queue.is_processed(&hash_clean) {
            return Ok(warp::reply::json(&serde_json::json!({
                "error": "Nebula lock already processed for unlock",
            })));
        }
        if queue.is_pending(&hash_clean) {
            return Ok(warp::reply::json(&serde_json::json!({
                "status": "unlock_queued",
                "nebula_tx_hash": format!("0x{}", hash_clean),
                "note": "Already pending for relayer",
            })));
        }
    }

    let vault = deterministic_address("nebula-bridge-vault-v1");
    let (tx, block_height) = match verify_nebula_lock_tx(&chain_store, &hash_bytes, &vault) {
        Ok(v) => v,
        Err(e) => return Ok(warp::reply::json(&serde_json::json!({ "error": e }))),
    };

    let body = tx.payload.as_slice();
    let eth_recipient = req.eth_recipient.clone().unwrap_or_else(|| {
        std::str::from_utf8(body)
            .ok()
            .and_then(|s| s.strip_prefix("BRIDGE:ethereum:"))
            .map(|s| s.to_string())
            .unwrap_or_default()
    });

    if eth_recipient.is_empty() || parse_address_20(&eth_recipient).is_none() {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": "Missing or invalid eth_recipient",
        })));
    }

    let eth_rpc = req
        .eth_rpc_url
        .or_else(|| std::env::var("ETH_RPC_URL").ok())
        .unwrap_or_default();

    let eth_rpc_ok = if !eth_rpc.is_empty() {
        verify_eth_rpc_reachable(&eth_rpc).await.unwrap_or(false)
    } else {
        false
    };

    let source_chain: u32 = std::env::var("NEBULA_CHAIN_ID")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let dest_chain: u32 = std::env::var("ETH_CHAIN_ID")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let message_id = u64::from_le_bytes(hash_bytes[..8].try_into().unwrap_or([0u8; 8]));

    let pending = PendingUnlock {
        nebula_tx_hash: hash_clean.clone(),
        eth_recipient: eth_recipient.clone(),
        amount: tx.value.to_string(),
        sender: format!("0x{}", hex::encode(tx.sender)),
        nonce: tx.nonce,
        message_id,
        source_chain,
        dest_chain,
        block_height,
    };

    bridge_unlock_queue.lock().await.enqueue(pending);

    Ok(warp::reply::json(&serde_json::json!({
        "status": "unlock_queued",
        "nebula_tx_hash": format!("0x{}", hash_clean),
        "eth_recipient": eth_recipient,
        "amount": tx.value.to_string(),
        "block_height": block_height,
        "message_id": message_id,
        "source_chain": source_chain,
        "dest_chain": dest_chain,
        "eth_rpc_configured": !eth_rpc.is_empty(),
        "eth_rpc_reachable": eth_rpc_ok,
        "note": "Relayer should poll GET /bridge/pending_unlocks and submit Bridge.unlockTokens on Ethereum",
    })))
}

pub(super) async fn verify_eth_rpc_reachable(eth_rpc: &str) -> Result<bool, String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_blockNumber",
        "params": [],
        "id": 1
    });
    let client = reqwest::Client::new();
    let resp = client
        .post(eth_rpc)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("ETH RPC unreachable: {e}"))?;
    let val: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Invalid ETH RPC response: {e}"))?;
    Ok(!val.get("result").map(|r| r.is_null()).unwrap_or(true))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BRIDGE: &str = "0x5fbdb2315678afecb367f032d93f642f64180aa3";

    fn receipt(address: &str, token: &str, amount_hex: &str, recipient: &str) -> serde_json::Value {
        serde_json::json!({
            "status": "0x1",
            "logs": [{
                "address": address,
                "topics": [
                    TOKENS_LOCKED_TOPIC,
                    format!("0x{:064x}", 7),
                    format!("0x{:064x}", 1),
                    token,
                ],
                "data": format!("0x{:0>64}{:0>64}", amount_hex, recipient),
            }],
        })
    }

    #[test]
    fn eth_lock_comes_from_the_bridge_event() {
        let zero = format!("0x{}", "0".repeat(64));
        let recip = "ab".repeat(20);
        let lock =
            parse_eth_lock(&receipt(BRIDGE, &zero, "de0b6b3a7640000", &recip), BRIDGE).unwrap();
        assert_eq!(lock.amount, 1_000_000_000_000_000_000);
        assert_eq!(lock.recipient, [0xab; 20]);

        // Another contract emitting the same event, an ERC20 lock, or a
        // failed transaction is not a lock.
        let other = "0x1111111111111111111111111111111111111111";
        assert!(parse_eth_lock(&receipt(other, &zero, "1", &recip), BRIDGE).is_err());
        let usdc = format!("0x{:0>64}", "e7f1725e7734ce288f8367e1bb143e90bb3f0512");
        assert!(parse_eth_lock(&receipt(BRIDGE, &usdc, "1", &recip), BRIDGE).is_err());
        let mut failed = receipt(BRIDGE, &zero, "1", &recip);
        failed["status"] = serde_json::json!("0x0");
        assert!(parse_eth_lock(&failed, BRIDGE).is_err());
    }
}
