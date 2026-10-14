//! Ethereum → Nebula bridge protocol, shared by the node (which verifies
//! mints in consensus) and the standalone relayer (which produces them).
//!
//! A relayer watches `Bridge.sol` for native-ETH `TokensLocked` events and
//! signs [`attestation_message`] for each lock with its Ed25519 key. A
//! `bridge_mint` transaction carrying attestations from `threshold` distinct
//! relayers of the genesis relayer set mints the locked amount on Nebula.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const BRIDGE_MINT_DOMAIN: &[u8] = b"NEBULA_BRIDGE_MINT_V1";

/// keccak256("TokensLocked(uint64,address,address,uint256,bytes32)")
pub const TOKENS_LOCKED_TOPIC: &str =
    "0x1b3a88149ad03e75e48758d7d1b76a639a6883d27b6949badde69a6033bd90de";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// Hex Ed25519 public keys of the relayers.
    pub relayers: Vec<String>,
    /// Distinct relayer attestations a mint needs.
    pub threshold: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attestation {
    /// Hex Ed25519 public key of the relayer.
    pub relayer: String,
    /// Hex signature over [`attestation_message`].
    pub signature: String,
}

/// `bridge_mint` transaction payload (with `"action": "bridge_mint"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMint {
    pub eth_tx_hash: String,
    pub recipient: String,
    pub amount: String,
    #[serde(default)]
    pub attestations: Vec<Attestation>,
}

impl BridgeMint {
    /// JSON payload for a `bridge_mint` transaction.
    pub fn to_payload(&self) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "action": "bridge_mint",
            "eth_tx_hash": self.eth_tx_hash,
            "recipient": self.recipient,
            "amount": self.amount,
            "attestations": self.attestations,
        }))
        .unwrap_or_default()
    }
}

/// What a relayer signs for one mint.
pub fn attestation_message(
    chain_id: u64,
    eth_tx_hash: &[u8; 32],
    recipient: &[u8; 20],
    amount: u128,
) -> Vec<u8> {
    let mut msg = Vec::with_capacity(BRIDGE_MINT_DOMAIN.len() + 76);
    msg.extend_from_slice(BRIDGE_MINT_DOMAIN);
    msg.extend_from_slice(&chain_id.to_le_bytes());
    msg.extend_from_slice(eth_tx_hash);
    msg.extend_from_slice(recipient);
    msg.extend_from_slice(&amount.to_le_bytes());
    msg
}

/// Sign an attestation for one lock.
pub fn attest(
    key: &ed25519_dalek::SigningKey,
    chain_id: u64,
    eth_tx_hash: &[u8; 32],
    recipient: &[u8; 20],
    amount: u128,
) -> Attestation {
    use ed25519_dalek::Signer;
    let msg = attestation_message(chain_id, eth_tx_hash, recipient, amount);
    Attestation {
        relayer: hex::encode(key.verifying_key().to_bytes()),
        signature: hex::encode(key.sign(&msg).to_bytes()),
    }
}

pub fn decode_fixed<const N: usize>(hex_str: &str) -> Option<[u8; N]> {
    hex::decode(hex_str.trim().trim_start_matches("0x"))
        .ok()?
        .try_into()
        .ok()
}

/// Parsed and checked fields of a mint payload: (mint, eth tx hash, recipient, amount).
pub fn parse_mint(payload: &[u8]) -> Result<(BridgeMint, [u8; 32], [u8; 20], u128), String> {
    let mint: BridgeMint =
        serde_json::from_slice(payload).map_err(|e| format!("bad bridge_mint: {}", e))?;
    let eth_tx_hash = decode_fixed::<32>(&mint.eth_tx_hash).ok_or("bad eth_tx_hash")?;
    let recipient = decode_fixed::<20>(&mint.recipient).ok_or("bad recipient")?;
    let amount = mint
        .amount
        .parse::<u128>()
        .ok()
        .filter(|a| *a > 0)
        .ok_or("bad amount")?;
    Ok((mint, eth_tx_hash, recipient, amount))
}

/// Check one attestation: from a relayer of `config`, signing this mint.
/// Returns the relayer's lowercase hex key.
pub fn verify_attestation(
    config: &BridgeConfig,
    chain_id: u64,
    att: &Attestation,
    eth_tx_hash: &[u8; 32],
    recipient: &[u8; 20],
    amount: u128,
) -> Result<String, String> {
    let relayer = att.relayer.trim_start_matches("0x").to_lowercase();
    if !config
        .relayers
        .iter()
        .any(|r| r.trim_start_matches("0x").eq_ignore_ascii_case(&relayer))
    {
        return Err("not a relayer of the genesis relayer set".into());
    }
    let pk = hex::decode(&relayer).map_err(|_| "bad relayer key")?;
    let sig = hex::decode(att.signature.trim_start_matches("0x"))
        .map_err(|_| "bad attestation signature")?;
    let msg = attestation_message(chain_id, eth_tx_hash, recipient, amount);
    crate::crypto::verify_signature(&pk, &msg, &sig).map_err(|_| {
        format!(
            "invalid attestation from relayer {}",
            &relayer[..8.min(relayer.len())]
        )
    })?;
    Ok(relayer)
}

/// Check that `threshold` distinct relayers of `config` signed this mint.
/// Attestations from keys outside the set carry no weight.
pub fn verify_attestations(
    config: &BridgeConfig,
    chain_id: u64,
    mint: &BridgeMint,
    eth_tx_hash: &[u8; 32],
    recipient: &[u8; 20],
    amount: u128,
) -> Result<(), String> {
    if config.threshold == 0 || config.relayers.is_empty() {
        return Err("bridge has no relayer set".into());
    }
    let mut signed = HashSet::new();
    for att in &mint.attestations {
        match verify_attestation(config, chain_id, att, eth_tx_hash, recipient, amount) {
            Ok(relayer) => {
                signed.insert(relayer);
            }
            Err(e) if e.starts_with("not a relayer") => {}
            Err(e) => return Err(e),
        }
    }
    if (signed.len() as u64) < u64::from(config.threshold) {
        return Err(format!(
            "{} of {} required relayer attestations",
            signed.len(),
            config.threshold
        ));
    }
    Ok(())
}

/// A native-ETH lock read from a `TokensLocked` event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthLock {
    pub amount: u128,
    pub recipient: [u8; 20],
}

/// Parse one Ethereum log (`eth_getLogs` / receipt format). `Ok(None)` when it
/// is not a `TokensLocked` event of `bridge`; an error when it is one that
/// cannot be bridged (an ERC20 lock, or malformed data).
pub fn parse_tokens_locked_log(
    log: &serde_json::Value,
    bridge: &str,
) -> Result<Option<EthLock>, String> {
    let bridge = bridge.trim().to_lowercase();
    let from_bridge = log
        .get("address")
        .and_then(|a| a.as_str())
        .is_some_and(|a| a.to_lowercase() == bridge);
    let topics: Vec<&str> = log
        .get("topics")
        .and_then(|t| t.as_array())
        .map(|t| t.iter().filter_map(|x| x.as_str()).collect())
        .unwrap_or_default();
    if !from_bridge || topics.len() != 4 || topics[0].to_lowercase() != TOKENS_LOCKED_TOPIC {
        return Ok(None);
    }
    // topics: [sig, messageId, user, token]; token must be address(0).
    let token = hex::decode(topics[3].trim_start_matches("0x")).unwrap_or_default();
    if token.len() != 32 || token.iter().any(|b| *b != 0) {
        return Err("only native ETH locks can be bridged".into());
    }
    // data: amount (uint256) ‖ recipient (bytes32, address left-padded)
    let data = log
        .get("data")
        .and_then(|d| d.as_str())
        .and_then(|d| hex::decode(d.trim_start_matches("0x")).ok())
        .filter(|d| d.len() == 64)
        .ok_or("malformed TokensLocked data")?;
    if data[..16].iter().any(|b| *b != 0) {
        return Err("locked amount exceeds u128".into());
    }
    let mut amount = [0u8; 16];
    amount.copy_from_slice(&data[16..32]);
    if data[32..44].iter().any(|b| *b != 0) {
        return Err("lock recipient is not a 20-byte address".into());
    }
    let mut recipient = [0u8; 20];
    recipient.copy_from_slice(&data[44..64]);
    Ok(Some(EthLock {
        amount: u128::from_be_bytes(amount),
        recipient,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attestation_round_trip_and_set_membership() {
        let relayer = ed25519_dalek::SigningKey::from_bytes(&[4; 32]);
        let outsider = ed25519_dalek::SigningKey::from_bytes(&[5; 32]);
        let config = BridgeConfig {
            relayers: vec![hex::encode(relayer.verifying_key().to_bytes())],
            threshold: 1,
        };
        let (hash, to, amount) = ([1u8; 32], [2u8; 20], 7u128);
        let good = attest(&relayer, 1, &hash, &to, amount);
        assert!(verify_attestation(&config, 1, &good, &hash, &to, amount).is_ok());
        // Wrong chain or amount: the signature no longer matches.
        assert!(verify_attestation(&config, 2, &good, &hash, &to, amount).is_err());
        assert!(verify_attestation(&config, 1, &good, &hash, &to, 8).is_err());
        let foreign = attest(&outsider, 1, &hash, &to, amount);
        assert!(verify_attestation(&config, 1, &foreign, &hash, &to, amount)
            .unwrap_err()
            .starts_with("not a relayer"));

        let mint = BridgeMint {
            eth_tx_hash: hex::encode(hash),
            recipient: hex::encode(to),
            amount: amount.to_string(),
            attestations: vec![foreign, good],
        };
        let (parsed, h, r, a) = parse_mint(&mint.to_payload()).unwrap();
        assert_eq!((h, r, a), (hash, to, amount));
        assert!(verify_attestations(&config, 1, &parsed, &h, &r, a).is_ok());
    }
}
