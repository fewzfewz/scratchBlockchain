//! Native account records in the flat state.
//!
//! Key: the 20-byte address. Value: JSON `{"balance": "<u128>", "nonce": n}`
//! (balance as a string so it survives JSON number limits). This is the one
//! place balances live: transfers, rewards, genesis and `/balance` all use it.

use storage::ChainStore;

/// `(balance, nonce)`; a missing or unreadable record is an empty account.
pub fn load(chain: &ChainStore, addr: &[u8]) -> (u128, u64) {
    decode(chain.get_state(addr).ok().flatten().as_deref())
}

pub fn decode(encoded: Option<&[u8]>) -> (u128, u64) {
    let Some(encoded) = encoded else {
        return (0, 0);
    };
    let Ok(acc) = serde_json::from_slice::<serde_json::Value>(encoded) else {
        return (0, 0);
    };
    let balance = acc
        .get("balance")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u128>().ok())
        .unwrap_or(0);
    let nonce = acc.get("nonce").and_then(|v| v.as_u64()).unwrap_or(0);
    (balance, nonce)
}

/// Canonical encoding (serde_json orders object keys, so this is stable).
pub fn encode(balance: u128, nonce: u64) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "balance": balance.to_string(),
        "nonce": nonce,
    }))
    .unwrap_or_default()
}

pub fn store(
    chain: &ChainStore,
    addr: &[u8],
    balance: u128,
    nonce: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    chain.put_state(addr, &encode(balance, nonce))
}

/// Add `amount` to an account, keeping its nonce. Returns the new balance.
pub fn credit(
    chain: &ChainStore,
    addr: &[u8],
    amount: u128,
) -> Result<u128, Box<dyn std::error::Error>> {
    let (balance, nonce) = load(chain, addr);
    let balance = balance
        .checked_add(amount)
        .ok_or("Balance overflow while crediting account")?;
    store(chain, addr, balance, nonce)?;
    Ok(balance)
}

/// Address of a validator's reward account: last 20 bytes of SHA-256 of its
/// public key (the same derivation as user accounts).
pub fn address_of_public_key(public_key: &[u8]) -> [u8; 20] {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(public_key);
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&digest[digest.len() - 20..]);
    addr
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn credit_keeps_nonce_and_checks_overflow() {
        let chain = ChainStore::new(Arc::new(storage::MemDb::new()));
        let addr = [7u8; 20];
        store(&chain, &addr, 10, 4).unwrap();
        assert_eq!(credit(&chain, &addr, 5).unwrap(), 15);
        assert_eq!(load(&chain, &addr), (15, 4));
        assert!(credit(&chain, &addr, u128::MAX).is_err());
        assert_eq!(load(&chain, &addr), (15, 4));
    }
}
