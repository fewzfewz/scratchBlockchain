//! Dev-mode routes that submit transactions from the genesis-funded dev account.

use super::*;

/// Minimum time (seconds) between faucet drips to the same address.
pub(super) const FAUCET_COOLDOWN_SECS: u64 = 60;

/// Submits node-signed transactions for the dev-mode routes.
pub(super) struct DevSigner {
    pub(super) key: ed25519_dalek::SigningKey,
    pub(super) address: [u8; 20],
    /// Next nonce to use; transactions may still be pending in the mempool,
    /// so the on-chain nonce alone would reuse them.
    pub(super) next_nonce: Mutex<u64>,
    pub(super) chain_store: Arc<ChainStore>,
    pub(super) tx_pool: Arc<TxPool>,
    pub(super) network_cmd_sender: mpsc::Sender<NetworkCommand>,
    pub(super) chain_id: u64,
}

impl DevSigner {
    /// Sign and submit a transaction from the dev account; returns its hash.
    pub(super) async fn submit(
        &self,
        to: Option<[u8; 20]>,
        value: u64,
        payload: Vec<u8>,
    ) -> Result<String, String> {
        let mut next = self.next_nonce.lock().await;
        let (_, onchain) = crate::accounts::load(&self.chain_store, &self.address);
        // Our transactions still waiting in the pool. If one was dropped after
        // it was accepted, `next` would point past a gap forever and every
        // later transaction would wait behind it: never run ahead of
        // on-chain nonce + pending.
        let pending = self
            .tx_pool
            .mempool_snapshot(100_000)
            .iter()
            .filter(|t| t.sender == self.address && t.nonce >= onchain)
            .count() as u64;
        let nonce = (*next).clamp(onchain, onchain + pending);
        let mut tx = Transaction {
            sender: self.address,
            nonce,
            public_key: [0; 32],
            payload,
            signature: vec![],
            gas_limit: 21_000,
            max_fee_per_gas: 1_000_000_000,
            max_priority_fee_per_gas: 1_000_000_000,
            chain_id: Some(self.chain_id),
            to,
            value,
        };
        tx.sign_with(&self.key);
        if let Err(e) = self.tx_pool.add_transaction(tx.clone()) {
            // Out of sync with the pool (e.g. a tx was dropped): restart from chain.
            *next = onchain;
            return Err(e);
        }
        *next = nonce + 1;
        let _ = self
            .network_cmd_sender
            .send(NetworkCommand::BroadcastTransaction(tx.clone()))
            .await;
        Ok(hex::encode(tx.hash()))
    }
}

/// Directly credit an account with test tokens (dev faucet, no transaction needed)
/// Largest faucet drip: a transaction's `value` is a u64 (~18.4 tokens).
pub(super) const FAUCET_MAX_DRIP: u128 = 10_000_000_000_000_000_000; // 10 tokens

pub(super) async fn handle_faucet_request(
    req: serde_json::Value,
    signer: Option<Arc<DevSigner>>,
    faucet_cooldowns: Arc<Mutex<HashMap<String, u64>>>,
) -> Result<impl warp::Reply, Infallible> {
    let Some(signer) = signer else {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": "Faucet disabled: set api.dev_funding_key (NODE_API_DEV_FUNDING_KEY)",
        })));
    };
    let Some(recipient) = req
        .get("address")
        .and_then(|v| v.as_str())
        .and_then(parse_address)
    else {
        return Ok(warp::reply::json(
            &serde_json::json!({"error": "Missing or invalid address"}),
        ));
    };
    let address = hex::encode(recipient);

    // Server-side cooldown: enforce a minimum wait between drips to the same address
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut cooldowns = faucet_cooldowns.lock().await;
    if let Some(&last_drip) = cooldowns.get(&address) {
        let elapsed = now.saturating_sub(last_drip);
        if elapsed < FAUCET_COOLDOWN_SECS {
            return Ok(warp::reply::json(&serde_json::json!({
                "error": "Faucet cooldown active",
                "retry_after_secs": FAUCET_COOLDOWN_SECS - elapsed,
            })));
        }
    }

    let drip_amount: u128 = req
        .get("amount")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(FAUCET_MAX_DRIP)
        .min(FAUCET_MAX_DRIP);

    // A real transfer from the dev account, ordered by consensus like any
    // other — the faucet used to overwrite balances (and reset nonces) on
    // the one node that served the request.
    match signer
        .submit(Some(recipient), drip_amount as u64, vec![])
        .await
    {
        Ok(hash) => {
            cooldowns.insert(address, now);
            Ok(warp::reply::json(&serde_json::json!({
                "status": "success",
                "amount": drip_amount.to_string(),
                "hash": hash,
                "note": "Funds arrive when the transfer is included in a block",
            })))
        }
        Err(e) => Ok(warp::reply::json(
            &serde_json::json!({"error": format!("Faucet transfer failed: {}", e)}),
        )),
    }
}

/// 410 Gone with a pointer to the replacement.
pub(super) fn gone(message: &str) -> warp::reply::WithStatus<warp::reply::Json> {
    warp::reply::with_status(
        warp::reply::json(&serde_json::json!({ "error": message })),
        warp::http::StatusCode::GONE,
    )
}

pub(super) async fn handle_deploy_wasm(
    req: DeployWasmRequest,
    signer: Option<Arc<DevSigner>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    use base64::Engine as _;
    let Some(signer) = signer else {
        return Ok(warp::reply::json(&serde_json::json!({
            "status": "error",
            "error": "WASM deploy disabled: set api.dev_funding_key (NODE_API_DEV_FUNDING_KEY)",
        })));
    };
    if req.name.is_empty() || req.name.contains(':') || req.name.len() > 64 {
        return Ok(warp::reply::json(&serde_json::json!({
            "status": "error",
            "error": "Invalid contract name (1-64 chars, no ':')",
        })));
    }
    let wasm = match base64::engine::general_purpose::STANDARD.decode(&req.wasm) {
        Ok(b) => b,
        Err(e) => {
            return Ok(warp::reply::json(&serde_json::json!({
                "status": "error",
                "error": format!("Invalid base64 wasm: {}", e),
            })));
        }
    };
    // Validate up front for a clear error; the executor validates again.
    let code_hash = match execution::WasmExecutor::new()
        .map_err(|e| e.to_string())
        .and_then(|exec| exec.validate_module(&wasm).map_err(|e| e.to_string()))
    {
        Ok(h) => h,
        Err(e) => {
            return Ok(warp::reply::json(&serde_json::json!({
                "status": "error",
                "error": format!("Invalid WASM module: {}", e),
            })));
        }
    };
    let payload = [
        execution::WASM_DEPLOY_PREFIX,
        req.name.as_bytes(),
        b":",
        wasm.as_slice(),
    ]
    .concat();
    match signer.submit(None, 0, payload).await {
        Ok(hash) => Ok(warp::reply::json(&serde_json::json!({
            "status": "success",
            "name": req.name,
            "code_hash": hex::encode(code_hash),
            "hash": hash,
            "note": "The contract is registered when the deploy transaction is included in a block",
        }))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({
            "status": "error",
            "error": e,
        }))),
    }
}

pub(super) async fn handle_call_wasm(
    req: CallWasmRequest,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let registry = crate::wasm_registry::WasmRegistry::new(chain_store);
    let wasm = match registry.get(&req.name) {
        Ok(Some(b)) => b,
        Ok(None) => {
            return Ok(warp::reply::json(&serde_json::json!({
                "status": "error",
                "error": format!("Contract '{}' not found", req.name),
            })));
        }
        Err(e) => {
            return Ok(warp::reply::json(&serde_json::json!({
                "status": "error",
                "error": e.to_string(),
            })));
        }
    };
    let executor = match execution::WasmExecutor::new() {
        Ok(e) => e,
        Err(e) => {
            return Ok(warp::reply::json(&serde_json::json!({
                "status": "error",
                "error": format!("WASM engine init: {}", e),
            })));
        }
    };
    match executor.execute_i32(&wasm, &req.func, req.arg) {
        Ok((result, gas_used)) => Ok(warp::reply::json(&serde_json::json!({
            "status": "success",
            "result": result,
            "gas_used": gas_used,
        }))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({
            "status": "error",
            "error": e.to_string(),
        }))),
    }
}

pub(super) async fn handle_list_wasm(
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let registry = crate::wasm_registry::WasmRegistry::new(chain_store);
    match registry.list() {
        Ok(names) => Ok(warp::reply::json(&serde_json::json!({
            "contracts": names,
            "count": names.len(),
        }))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({
            "error": e.to_string(),
            "contracts": [],
            "count": 0,
        }))),
    }
}
