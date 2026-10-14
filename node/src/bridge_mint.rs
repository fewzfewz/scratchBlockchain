//! Bridge mints as consensus transactions.
//!
//! Tokens locked on Ethereum are minted here only by a `bridge_mint`
//! transaction that carries attestations from at least `threshold` distinct
//! relayers of the genesis relayer set. Each relayer signs
//! [`attestation_message`] (chain id, ETH lock tx hash, recipient, amount)
//! after checking the lock on Ethereum itself. Every node verifies the
//! attestations while applying the block, and the ETH tx hash is recorded in
//! consensus state, so a lock can be minted once — on every node, across
//! restarts — and no single relayer (or RPC node) can mint on its own.
//!
//! Payload (JSON, `to: None`, signed and fee-paying like any transaction —
//! anyone may submit it once it has enough attestations):
//!
//! ```json
//! {"action": "bridge_mint", "eth_tx_hash": "0x…32 bytes", "recipient": "0x…20 bytes",
//!  "amount": "<u128 wei>", "attestations": [{"relayer": "<hex ed25519 pk>", "signature": "<hex>"}]}
//! ```

pub use common::bridge::{
    attestation_message, decode_fixed, parse_mint as parse, verify_attestations, Attestation,
    BridgeConfig, BridgeMint, BRIDGE_MINT_DOMAIN,
};
use std::sync::Arc;
use storage::trie::PatriciaTrie;
use storage::ChainStore;
use tokio::sync::Mutex;

/// Trie key holding the relayer set and threshold (seeded from genesis).
pub const BRIDGE_CONFIG_KEY: &[u8] = b"bridge_config";
/// Flat-state prefix (inside the state root) for minted ETH lock tx hashes.
pub const MINTED_PREFIX: &[u8] = b"bridge:minted:";
/// Flat-state key for the total amount minted by the bridge.
pub const TOTAL_MINTED_KEY: &[u8] = b"bridge:total_minted";

pub fn is_bridge_mint_payload(payload: &[u8]) -> bool {
    payload.first() == Some(&b'{')
        && serde_json::from_slice::<serde_json::Value>(payload)
            .ok()
            .and_then(|v| v.get("action")?.as_str().map(|a| a == "bridge_mint"))
            .unwrap_or(false)
}

pub async fn load_config(trie: &Arc<Mutex<PatriciaTrie>>) -> Option<BridgeConfig> {
    let data = trie.lock().await.get(BRIDGE_CONFIG_KEY).ok()??;
    serde_json::from_slice(&data).ok()
}

pub fn minted_key(eth_tx_hash: &[u8; 32]) -> Vec<u8> {
    [MINTED_PREFIX, eth_tx_hash.as_slice()].concat()
}

pub fn already_minted(chain_store: &ChainStore, eth_tx_hash: &[u8; 32]) -> bool {
    matches!(chain_store.get_state(&minted_key(eth_tx_hash)), Ok(Some(_)))
}

/// Apply a `bridge_mint` transaction (already authenticated by the executor).
/// Mints new tokens to the recipient; any failure leaves state unchanged.
pub async fn apply(
    trie: &Arc<Mutex<PatriciaTrie>>,
    chain_store: &ChainStore,
    tx: &common::types::Transaction,
) -> Result<u128, String> {
    let (mint, eth_tx_hash, recipient, amount) = parse(&tx.payload)?;
    let chain_id = tx.chain_id.ok_or("transaction has no chain id")?;
    let config = load_config(trie)
        .await
        .ok_or("bridge is not configured in genesis")?;
    verify_attestations(&config, chain_id, &mint, &eth_tx_hash, &recipient, amount)?;
    if already_minted(chain_store, &eth_tx_hash) {
        return Err("ETH lock already minted".into());
    }

    let total = chain_store
        .get_state(TOTAL_MINTED_KEY)
        .ok()
        .flatten()
        .and_then(|b| <[u8; 16]>::try_from(b.as_slice()).ok())
        .map(u128::from_le_bytes)
        .unwrap_or(0)
        .checked_add(amount)
        .ok_or("bridge total overflow")?;
    let (balance, nonce) = crate::accounts::load(chain_store, &recipient);
    let balance = balance.checked_add(amount).ok_or("balance overflow")?;

    crate::accounts::store(chain_store, &recipient, balance, nonce).map_err(|e| e.to_string())?;
    chain_store
        .put_state(&minted_key(&eth_tx_hash), &1u8.to_le_bytes())
        .map_err(|e| e.to_string())?;
    chain_store
        .put_state(TOTAL_MINTED_KEY, &total.to_le_bytes())
        .map_err(|e| e.to_string())?;
    Ok(amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;

    fn relayer(seed: u8) -> ed25519_dalek::SigningKey {
        ed25519_dalek::SigningKey::from_bytes(&[seed; 32])
    }

    fn mint_tx(
        signers: &[&ed25519_dalek::SigningKey],
        eth_hash: [u8; 32],
        amount: u128,
    ) -> common::types::Transaction {
        let recipient = [7u8; 20];
        let msg = attestation_message(1, &eth_hash, &recipient, amount);
        let attestations: Vec<serde_json::Value> = signers
            .iter()
            .map(|k| {
                serde_json::json!({
                    "relayer": hex::encode(k.verifying_key().to_bytes()),
                    "signature": hex::encode(k.sign(&msg).to_bytes()),
                })
            })
            .collect();
        let payload = serde_json::json!({
            "action": "bridge_mint",
            "eth_tx_hash": format!("0x{}", hex::encode(eth_hash)),
            "recipient": format!("0x{}", hex::encode(recipient)),
            "amount": amount.to_string(),
            "attestations": attestations,
        });
        let key = relayer(99);
        let mut tx = common::types::Transaction::simple_transfer([0; 20], [0; 20], 0, 0, 1);
        tx.to = None;
        tx.payload = serde_json::to_vec(&payload).unwrap();
        tx.sign_with(&key);
        tx
    }

    async fn setup(threshold: u32) -> (Arc<Mutex<PatriciaTrie>>, ChainStore) {
        let inner = Arc::new(storage::MemDb::new());
        let chain_store = ChainStore::new(inner.clone());
        let trie = Arc::new(Mutex::new(PatriciaTrie::new(inner).unwrap()));
        let config = BridgeConfig {
            relayers: (1u8..=3)
                .map(|i| hex::encode(relayer(i).verifying_key().to_bytes()))
                .collect(),
            threshold,
        };
        trie.lock()
            .await
            .insert(BRIDGE_CONFIG_KEY, &serde_json::to_vec(&config).unwrap())
            .unwrap();
        (trie, chain_store)
    }

    #[tokio::test]
    async fn mint_needs_threshold_of_distinct_relayers_and_happens_once() {
        let (trie, chain) = setup(2).await;
        let (r1, r2, outsider) = (relayer(1), relayer(2), relayer(50));
        let hash = [3u8; 32];
        let amount = 25_000_000_000_000_000_000u128; // above the old u64 cap

        // One relayer, the same relayer twice, or an outsider: not enough.
        assert!(is_bridge_mint_payload(
            &mint_tx(&[&r1], hash, amount).payload
        ));
        assert!(apply(&trie, &chain, &mint_tx(&[&r1], hash, amount))
            .await
            .is_err());
        assert!(apply(&trie, &chain, &mint_tx(&[&r1, &r1], hash, amount))
            .await
            .is_err());
        assert!(
            apply(&trie, &chain, &mint_tx(&[&r1, &outsider], hash, amount))
                .await
                .is_err()
        );
        assert_eq!(crate::accounts::load(&chain, &[7u8; 20]).0, 0);

        // Two distinct relayers: minted, and only once.
        let tx = mint_tx(&[&r1, &r2], hash, amount);
        assert_eq!(apply(&trie, &chain, &tx).await.unwrap(), amount);
        assert_eq!(crate::accounts::load(&chain, &[7u8; 20]).0, amount);
        assert!(apply(&trie, &chain, &tx).await.is_err(), "replayed lock");
        assert_eq!(crate::accounts::load(&chain, &[7u8; 20]).0, amount);
    }

    #[tokio::test]
    async fn attestation_binds_amount_and_recipient() {
        let (trie, chain) = setup(1).await;
        let mut tx = mint_tx(&[&relayer(1)], [4u8; 32], 100);
        // Raise the amount after signing: the attestation no longer matches.
        let mut payload: serde_json::Value = serde_json::from_slice(&tx.payload).unwrap();
        payload["amount"] = serde_json::json!("1000000");
        tx.payload = serde_json::to_vec(&payload).unwrap();
        let err = apply(&trie, &chain, &tx).await.unwrap_err();
        assert!(err.contains("invalid attestation"), "{}", err);
    }
}
