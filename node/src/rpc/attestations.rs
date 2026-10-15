//! Bridge attestation mailbox (Ethereum → Nebula).
//!
//! Independent relayers each watch Ethereum, sign an attestation for every
//! lock they see, and post it here; whichever relayer sees `threshold` of them
//! submits the `bridge_mint` transaction. The pool is node-local scratch space
//! — only attestations with a valid signature from a genesis relayer are
//! kept, and consensus re-verifies everything in the mint transaction.

use super::*;
use common::bridge::{Attestation, BridgeConfig};
use std::collections::{BTreeMap, VecDeque};

/// Locks remembered at most; the oldest are dropped first.
const MAX_LOCKS: usize = 10_000;

#[derive(Debug, Clone)]
pub(crate) struct PoolEntry {
    pub recipient: [u8; 20],
    pub amount: u128,
    /// Keyed by relayer key, so each relayer counts once.
    pub attestations: BTreeMap<String, Attestation>,
}

#[derive(Default)]
pub(crate) struct AttestationPool {
    /// Per ETH lock tx hash (lowercase hex, no 0x). Relayers that disagree on
    /// recipient or amount form separate entries; only one can reach the
    /// threshold with honest relayers.
    locks: BTreeMap<String, Vec<PoolEntry>>,
    order: VecDeque<String>,
}

impl AttestationPool {
    /// Add a verified attestation; returns how many distinct relayers have
    /// now attested to this (lock, recipient, amount).
    pub(crate) fn add(
        &mut self,
        eth_tx_hash: &str,
        recipient: [u8; 20],
        amount: u128,
        relayer: String,
        attestation: Attestation,
    ) -> usize {
        if !self.locks.contains_key(eth_tx_hash) {
            if self.order.len() >= MAX_LOCKS {
                if let Some(old) = self.order.pop_front() {
                    self.locks.remove(&old);
                }
            }
            self.order.push_back(eth_tx_hash.to_string());
        }
        let entries = self.locks.entry(eth_tx_hash.to_string()).or_default();
        let idx = match entries
            .iter()
            .position(|e| e.recipient == recipient && e.amount == amount)
        {
            Some(i) => i,
            None => {
                entries.push(PoolEntry {
                    recipient,
                    amount,
                    attestations: BTreeMap::new(),
                });
                entries.len() - 1
            }
        };
        let entry = &mut entries[idx];
        entry.attestations.insert(relayer, attestation);
        entry.attestations.len()
    }

    pub(crate) fn get(&self, eth_tx_hash: &str) -> Vec<PoolEntry> {
        self.locks.get(eth_tx_hash).cloned().unwrap_or_default()
    }

    pub(crate) fn remove(&mut self, eth_tx_hash: &str) {
        self.locks.remove(eth_tx_hash);
        self.order.retain(|h| h != eth_tx_hash);
    }
}

pub(super) fn normalize_hash(h: &str) -> String {
    h.trim().trim_start_matches("0x").to_lowercase()
}

#[derive(Debug, Deserialize)]
pub(super) struct AttestationSubmission {
    pub eth_tx_hash: String,
    pub recipient: String,
    pub amount: String,
    pub attestation: Attestation,
}

/// Check and pool one relayer attestation. Shared by the public route and
/// the dev-mode relayer route.
pub(super) async fn add_attestation(
    sub: &AttestationSubmission,
    state_trie: &Arc<Mutex<PatriciaTrie>>,
    chain_store: &ChainStore,
    pool: &Mutex<AttestationPool>,
    chain_id: u64,
) -> Result<(usize, BridgeConfig, bool), String> {
    let eth = common::bridge::decode_fixed::<32>(&sub.eth_tx_hash).ok_or("bad eth_tx_hash")?;
    let recipient = common::bridge::decode_fixed::<20>(&sub.recipient).ok_or("bad recipient")?;
    let amount = sub
        .amount
        .parse::<u128>()
        .ok()
        .filter(|a| *a > 0)
        .ok_or("bad amount")?;
    let config = crate::bridge_mint::load_config(state_trie)
        .await
        .ok_or("bridge is not configured in genesis")?;
    if crate::bridge_mint::already_minted(chain_store, &eth) {
        return Ok((0, config, true));
    }
    let relayer = common::bridge::verify_attestation(
        &config,
        chain_id,
        &sub.attestation,
        &eth,
        &recipient,
        amount,
    )?;
    let count = pool.lock().await.add(
        &hex::encode(eth),
        recipient,
        amount,
        relayer,
        sub.attestation.clone(),
    );
    Ok((count, config, false))
}

pub(super) async fn handle_post_attestation(
    sub: AttestationSubmission,
    state_trie: Arc<Mutex<PatriciaTrie>>,
    chain_store: Arc<ChainStore>,
    pool: Arc<Mutex<AttestationPool>>,
    chain_id: u64,
) -> Result<impl warp::Reply, Infallible> {
    let reply = match add_attestation(&sub, &state_trie, &chain_store, &pool, chain_id).await {
        Ok((_, _, true)) => serde_json::json!({ "status": "minted" }),
        Ok((count, config, false)) => serde_json::json!({
            "status": "accepted",
            "attestations": count,
            "threshold": config.threshold,
        }),
        Err(e) => serde_json::json!({ "error": e }),
    };
    Ok(warp::reply::json(&reply))
}

pub(super) async fn handle_get_attestations(
    eth_tx_hash: String,
    state_trie: Arc<Mutex<PatriciaTrie>>,
    chain_store: Arc<ChainStore>,
    pool: Arc<Mutex<AttestationPool>>,
) -> Result<impl warp::Reply, Infallible> {
    let key = normalize_hash(&eth_tx_hash);
    let minted = common::bridge::decode_fixed::<32>(&key)
        .is_some_and(|h| crate::bridge_mint::already_minted(&chain_store, &h));
    if minted {
        pool.lock().await.remove(&key);
    }
    let config = crate::bridge_mint::load_config(&state_trie).await;
    let entries: Vec<serde_json::Value> = pool
        .lock()
        .await
        .get(&key)
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "recipient": format!("0x{}", hex::encode(e.recipient)),
                "amount": e.amount.to_string(),
                "attestations": e.attestations.values().collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(warp::reply::json(&serde_json::json!({
        "eth_tx_hash": format!("0x{}", key),
        "minted": minted,
        "threshold": config.as_ref().map(|c| c.threshold).unwrap_or(0),
        "relayers": config.map(|c| c.relayers).unwrap_or_default(),
        "entries": entries,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_counts_each_relayer_once_per_claim() {
        let mut pool = AttestationPool::default();
        let att = |r: &str| Attestation {
            relayer: r.into(),
            signature: "00".into(),
        };
        assert_eq!(pool.add("aa", [1; 20], 5, "r1".into(), att("r1")), 1);
        assert_eq!(pool.add("aa", [1; 20], 5, "r1".into(), att("r1")), 1);
        assert_eq!(pool.add("aa", [1; 20], 5, "r2".into(), att("r2")), 2);
        // A different claim about the same lock is tracked separately.
        assert_eq!(pool.add("aa", [1; 20], 6, "r3".into(), att("r3")), 1);
        assert_eq!(pool.get("aa").len(), 2);
        pool.remove("aa");
        assert!(pool.get("aa").is_empty());
    }
}
