//! Gas price, fee history, gas estimation and read-only EVM calls.

use super::*;

pub(super) async fn handle_gas_price(
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
                        let base_fee_val = block_val
                            .get("header")
                            .and_then(|h| h.get("base_fee"))
                            .and_then(|b| b.as_u64())
                            .unwrap_or(1_000_000_000);

                        // Priority fees based on recent blocks (simplified)
                        let low = base_fee_val;
                        let medium = base_fee_val * 2;
                        let high = base_fee_val * 5;

                        (base_fee_val, low, medium, high)
                    } else {
                        (
                            1_000_000_000u64,
                            1_000_000_000,
                            2_000_000_000,
                            5_000_000_000,
                        )
                    }
                } else {
                    (
                        1_000_000_000u64,
                        1_000_000_000,
                        2_000_000_000,
                        5_000_000_000,
                    )
                }
            } else {
                (
                    1_000_000_000u64,
                    1_000_000_000,
                    2_000_000_000,
                    5_000_000_000,
                )
            }
        } else {
            (
                1_000_000_000u64,
                1_000_000_000,
                2_000_000_000,
                5_000_000_000,
            )
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

pub(super) async fn handle_estimate_gas(
    request: EstimateGasRequest,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    // Simulate the transaction against current state, like eth_estimateGas.
    // (A byte-count formula used to under-estimate every contract call and
    // creation, so transactions sent with its estimate ran out of gas.)
    let parse_addr = |s: &str| -> Option<revm::primitives::Address> {
        let clean = s.trim().trim_start_matches("0x");
        let bytes = hex::decode(clean).ok()?;
        (bytes.len() == 20).then(|| revm::primitives::Address::from_slice(&bytes))
    };
    let data = hex::decode(request.data.trim().trim_start_matches("0x")).unwrap_or_default();
    let to = parse_addr(&request.to).filter(|a| *a != revm::primitives::Address::ZERO);
    let from = parse_addr(&request.from).unwrap_or(revm::primitives::Address::ZERO);
    let value = request
        .value
        .as_deref()
        .and_then(|v| v.parse::<u128>().ok())
        .map(revm::primitives::U256::from)
        .unwrap_or_default();
    let (estimated_gas, error) = if to.is_some() && data.is_empty() {
        (21_000, None) // plain transfer
    } else {
        let store = Arc::new(crate::evm_store::ChainStoreEvmStore::new(
            chain_store.clone(),
        ));
        match execution::evm::EvmExecutor::simulate(store, from, to, data, value) {
            // Headroom for gas refunds and state that changes before inclusion.
            Ok(sim) => (sim.gas_used.saturating_mul(12) / 10, sim.error),
            Err(e) => (0, Some(e.to_string())),
        }
    };

    // Get current base fee from latest block
    let latest_height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    let base_fee = if let Ok(Some(hash_bytes)) = chain_store.get_block_hash_by_height(latest_height)
    {
        if hash_bytes.len() == 32 {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&hash_bytes);
            if let Ok(Some(encoded)) = chain_store.get_block(&hash) {
                if let Ok(block_val) = serde_json::from_slice::<serde_json::Value>(&encoded) {
                    block_val
                        .get("header")
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
    let max_fee = request
        .max_fee_per_gas
        .as_ref()
        .and_then(|f| f.parse::<u128>().ok())
        .unwrap_or(1_000_000_000);

    let total_cost = estimated_gas as u128 * max_fee;
    let estimated_priority = (max_fee / 2).max(1_000_000_000) as u64; // ≥ mempool min_fee_per_gas

    let response = EstimateGasResponse {
        estimated_gas,
        base_fee: base_fee.to_string(),
        total_cost_estimate: total_cost.to_string(),
        estimated_priority_fee: estimated_priority.to_string(),
        error,
    };

    Ok(warp::reply::json(&response))
}

pub(super) async fn handle_call_contract(
    request: CallContractRequest,
    chain_store: Arc<ChainStore>,
    _chain_id: u64,
) -> Result<impl warp::Reply, Infallible> {
    use crate::evm_store::ChainStoreEvmStore;
    use execution::evm::EvmExecutor;
    use revm::primitives::{Address, U256};
    use std::str::FromStr;

    let parse_addr = |s: &str| -> Option<Address> {
        let clean = s.trim_start_matches("0x");
        if clean.len() != 40 {
            return None;
        }
        Address::from_str(&format!("0x{clean}")).ok()
    };

    let from = match parse_addr(&request.from) {
        Some(a) => a,
        None => {
            return Ok(warp::reply::json(&CallContractResponse {
                result: "0x".to_string(),
                success: false,
                error: Some("invalid from address".to_string()),
            }))
        }
    };
    let to = match parse_addr(&request.to) {
        Some(a) => a,
        None => {
            return Ok(warp::reply::json(&CallContractResponse {
                result: "0x".to_string(),
                success: false,
                error: Some("invalid to address".to_string()),
            }))
        }
    };

    let data = hex::decode(request.data.trim_start_matches("0x")).unwrap_or_default();
    let value = request
        .value
        .as_ref()
        .and_then(|v| v.parse::<u128>().ok())
        .map(U256::from)
        .unwrap_or(U256::ZERO);

    let store = Arc::new(ChainStoreEvmStore::new(chain_store));
    match EvmExecutor::static_call(store, from, to, data, value) {
        Ok(bytes) => Ok(warp::reply::json(&CallContractResponse {
            result: format!("0x{}", hex::encode(bytes)),
            success: true,
            error: None,
        })),
        Err(e) => Ok(warp::reply::json(&CallContractResponse {
            result: "0x".to_string(),
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

pub(super) async fn handle_fee_history(
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
                        let base_fee = block_val
                            .get("header")
                            .and_then(|h| h.get("base_fee"))
                            .and_then(|b| b.as_u64())
                            .unwrap_or(1_000_000_000);
                        base_fees.push(base_fee.to_string());

                        let gas_used = block_val
                            .get("header")
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
