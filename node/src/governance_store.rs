//! Wiring the `governance` crate into the node's state trie.
//!
//! Governance state lives in the state trie under the `b"governance"` key as a
//! serialized `ChainGovernance`. It is seeded at genesis (or lazily on first
//! startup of an existing chain) and updated deterministically whenever a block
//! containing governance transactions is applied — by the block proposer, by
//! every validator applying a finalized block, and during catch-up sync.

use common::types::Address;
use governance::{ChainGovernance, GovernanceParams, ProposalType, VoteChoice};
use std::sync::Arc;
use storage::trie::PatriciaTrie;
use storage::ChainStore;
use tokio::sync::Mutex;
use tracing::{info, warn};

/// Trie key under which the serialized governance state lives.
pub const GOVERNANCE_KEY: &[u8] = b"governance";

/// Initial treasury balance (smallest token unit) seeded at genesis.
const SEED_TREASURY_BALANCE: u128 = 5_000_000_000_000_000_000;

/// Load the persisted governance state, seeding it if the trie has none.
pub async fn load_or_init(
    trie: &Arc<Mutex<PatriciaTrie>>,
) -> Result<ChainGovernance, Box<dyn std::error::Error>> {
    let guard = trie.lock().await;
    if let Some(data) = guard.get(GOVERNANCE_KEY)? {
        if let Ok(state) = serde_json::from_slice(&data) {
            return Ok(state);
        }
        warn!("Stored governance state failed to parse — reseeding");
    }
    drop(guard);

    let state = seed_governance(trie).await?;
    let data = serde_json::to_vec(&state)?;
    trie.lock().await.insert(GOVERNANCE_KEY, &data)?;
    info!("Seeded on-chain governance state");
    Ok(state)
}

/// Seed a fresh governance state with a couple of illustrative proposals whose
/// votes are weighted by the genesis validator set.
async fn seed_governance(
    trie: &Arc<Mutex<PatriciaTrie>>,
) -> Result<ChainGovernance, Box<dyn std::error::Error>> {
    let params = GovernanceParams::default();
    let total_stake = total_stake(trie).await;
    let mut state = ChainGovernance::new(params, SEED_TREASURY_BALANCE, total_stake);

    let validators = validator_stakes(trie).await;
    let (v1, v2, v3) = match validators.as_slice() {
        [a, b, c, ..] => (a.clone(), b.clone(), c.clone()),
        [a, b] => (a.clone(), b.clone(), (b.0, 0)),
        [a] => (a.clone(), (a.0, 0), (a.0, 0)),
        _ => return Ok(state),
    };

    let seed = |state: &mut ChainGovernance,
                title: &str,
                description: &str,
                votes: Vec<(Address, u128, VoteChoice)>| {
        let id = state
            .propose(
                v1.0,
                title,
                description,
                ProposalType::TextProposal {
                    title: title.to_string(),
                    description: description.to_string(),
                },
                0,
            )
            .unwrap_or(0);
        for (voter, power, choice) in votes {
            let _ = state.vote(id, voter, choice, power, 0);
        }
    };

    seed(
        &mut state,
        "Upgrade consensus to GRANDPA finality",
        "Replace the current BFT voting pipeline with GRANDPA-style finality gadget for faster, more robust finalization.",
        vec![
            (v1.0, v1.1, VoteChoice::Yes),
            (v2.0, v2.1, VoteChoice::No),
            (v3.0, v3.1, VoteChoice::Yes),
        ],
    );
    seed(
        &mut state,
        "Treasury allocation for protocol development fund",
        "Allocate 2,000,000 NBL from the on-chain treasury to fund ongoing protocol development and audits.",
        vec![
            (v1.0, v1.1, VoteChoice::Yes),
            (v2.0, v2.1, VoteChoice::Yes),
            (v3.0, v3.1, VoteChoice::Abstain),
        ],
    );
    seed(
        &mut state,
        "Increase block time to 6 seconds",
        "Slow the block cadence from 3s to 6s to reduce bandwidth requirements for validator nodes.",
        vec![
            (v1.0, v1.1, VoteChoice::No),
            (v2.0, v2.1, VoteChoice::No),
            (v3.0, v3.1, VoteChoice::No),
        ],
    );

    Ok(state)
}

/// Apply any governance transactions in a block's extrinsics and persist the
/// updated state. Non-governance transactions are ignored here.
pub async fn apply_extrinsics(
    trie: &Arc<Mutex<PatriciaTrie>>,
    extrinsics: &[common::types::Transaction],
    current_block: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = load_or_init(trie).await?;
    let mut changed = false;

    for tx in extrinsics {
        if !ChainGovernance::is_governance_payload(&tx.payload) {
            continue;
        }
        let power = voter_power(trie, &tx.sender).await;
        match state.apply_payload(tx.sender, &tx.payload, power, current_block) {
            Ok(()) => {
                info!(
                    "Governance action applied at block {} (voter {} power {})",
                    current_block,
                    hex::encode(&tx.sender[..6]),
                    power
                );
                changed = true;
            }
            Err(e) => warn!(
                "Governance transaction rejected at block {}: {}",
                current_block, e
            ),
        }
    }

    if changed {
        let data = serde_json::to_vec(&state)?;
        trie.lock().await.insert(GOVERNANCE_KEY, &data)?;
    }
    Ok(())
}

/// Voting power of an account: its staked (and delegated) balance, resolved
/// from the consensus-state `b"validators"` trie entry. The validator set is
/// identical on every node, so vote weights agree across the network — unlike
/// raw account balances, some of which are only credited on a single node.
pub async fn voter_power(trie: &Arc<Mutex<PatriciaTrie>>, address: &Address) -> u128 {
    validator_stakes(trie)
        .await
        .iter()
        .find(|(addr, _)| addr == address)
        .map(|(_, power)| *power)
        .unwrap_or(0)
}

fn parse_balance(v: &serde_json::Value) -> Option<u128> {
    v.as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| v.as_u64().map(|n| n as u128))
}

/// Total stake of the validator set (from the `b"validators"` trie entry).
async fn total_stake(trie: &Arc<Mutex<PatriciaTrie>>) -> u128 {
    validator_stakes(trie).await.iter().map(|(_, s)| s).sum()
}

/// Addresses and staked (self-stake + delegated) balances of the validator set.
async fn validator_stakes(trie: &Arc<Mutex<PatriciaTrie>>) -> Vec<(Address, u128)> {
    let guard = trie.lock().await;
    let Ok(Some(data)) = guard.get(b"validators") else {
        return vec![];
    };

    #[derive(serde::Deserialize)]
    struct Entry {
        address: String,
        stake: serde_json::Value,
        total_delegated: serde_json::Value,
    }

    let entries: Vec<Entry> = serde_json::from_slice(&data).unwrap_or_default();
    entries
        .iter()
        .filter_map(|e| {
            let bytes = hex::decode(e.address.trim_start_matches("0x")).ok()?;
            if bytes.len() != 20 {
                return None;
            }
            let mut addr = [0u8; 20];
            addr.copy_from_slice(&bytes);
            let stake = parse_balance(&e.stake)?;
            let delegated = parse_balance(&e.total_delegated).unwrap_or(0);
            Some((addr, stake + delegated))
        })
        .collect()
}

/// Credit a portion of block fees to the on-chain treasury.
pub async fn collect_treasury_fee(
    trie: &Arc<Mutex<PatriciaTrie>>,
    amount: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    if amount == 0 {
        return Ok(());
    }
    let mut state = load_or_init(trie).await?;
    state.treasury.balance = state.treasury.balance.saturating_add(amount as u128);
    state.treasury.total_collected = state
        .treasury
        .total_collected
        .saturating_add(amount as u128);
    let data = serde_json::to_vec(&state)?;
    trie.lock().await.insert(GOVERNANCE_KEY, &data)?;
    Ok(())
}

/// Record a delegation and persist delegator index + validator totals.
pub async fn apply_delegate(
    trie: &Arc<Mutex<PatriciaTrie>>,
    chain_store: &ChainStore,
    delegator: Address,
    validator: Address,
    amount: u128,
) -> Result<(), String> {
    if amount == 0 {
        return Err("Amount must be positive".into());
    }

    let mut validators = load_validators(trie).await?;
    let val_hex = format!("0x{}", hex::encode(validator));
    let entry = validators
        .iter_mut()
        .find(|v| v.address.eq_ignore_ascii_case(&val_hex))
        .ok_or("Validator not found")?;

    let current: u128 = entry.total_delegated.parse().unwrap_or(0);
    entry.total_delegated = (current + amount).to_string();
    entry.delegator_count = entry.delegator_count.saturating_add(1);
    persist_validators(trie, &validators).await?;

    let delegator_hex = format!("0x{}", hex::encode(delegator));
    let key = [b"delegations/", delegator_hex.as_bytes()].concat();
    let mut list: Vec<serde_json::Value> = chain_store
        .get_state(&key)
        .ok()
        .flatten()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    list.push(serde_json::json!({
        "validator_address": val_hex,
        "amount": amount.to_string(),
    }));
    chain_store
        .put_state(&key, &serde_json::to_vec(&list).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Register a new validator in the dynamic validator set (minimum stake required).
pub async fn register_validator(
    trie: &Arc<Mutex<PatriciaTrie>>,
    address: Address,
    public_key: Vec<u8>,
    stake: u128,
    commission_rate: u64,
) -> Result<(), String> {
    if stake < 1_000_000_000_000_000_000 {
        return Err("Stake below minimum (1 token)".into());
    }
    let mut validators = load_validators(trie).await?;
    let addr_hex = format!("0x{}", hex::encode(address));
    if validators
        .iter()
        .any(|v| v.address.eq_ignore_ascii_case(&addr_hex))
    {
        return Err("Validator already registered".into());
    }
    validators.push(ValidatorEntry {
        address: addr_hex,
        public_key: hex::encode(&public_key),
        stake: serde_json::Value::String(stake.to_string()),
        commission_rate,
        is_active: true,
        blocks_produced: 0,
        blocks_missed: 0,
        delegator_count: 0,
        total_delegated: "0".to_string(),
    });
    persist_validators(trie, &validators).await
}

#[derive(serde::Deserialize, serde::Serialize)]
struct ValidatorEntry {
    address: String,
    #[serde(default)]
    public_key: String,
    stake: serde_json::Value,
    #[serde(default)]
    commission_rate: u64,
    #[serde(default)]
    is_active: bool,
    #[serde(default)]
    blocks_produced: u64,
    #[serde(default)]
    blocks_missed: u64,
    #[serde(default)]
    delegator_count: u64,
    #[serde(default)]
    total_delegated: String,
}

async fn load_validators(trie: &Arc<Mutex<PatriciaTrie>>) -> Result<Vec<ValidatorEntry>, String> {
    let guard = trie.lock().await;
    let data = guard
        .get(b"validators")
        .map_err(|e| e.to_string())?
        .ok_or("No validator set")?;
    serde_json::from_slice(&data).map_err(|e| e.to_string())
}

async fn persist_validators(
    trie: &Arc<Mutex<PatriciaTrie>>,
    validators: &[ValidatorEntry],
) -> Result<(), String> {
    let data = serde_json::to_vec(validators).map_err(|e| e.to_string())?;
    trie.lock()
        .await
        .insert(b"validators", &data)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn parse_stake_value(v: &serde_json::Value) -> u128 {
    match v {
        serde_json::Value::String(s) => s.parse().unwrap_or(0),
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(0) as u128,
        _ => 0,
    }
}

/// Record block production stats for the finalized block at `height`.
///
/// - Increments `blocks_produced` for the actual proposer (matched by public key).
/// - Increments `blocks_missed` for the validator the deterministic round-0
///   leader schedule expected to propose this height but did not.
pub async fn record_block_stats(
    trie: &Arc<Mutex<PatriciaTrie>>,
    height: u64,
    proposer_pubkey: &[u8],
) -> Result<(), String> {
    let mut validators = load_validators(trie).await?;
    if validators.is_empty() {
        return Ok(());
    }
    let proposer_hex = hex::encode(proposer_pubkey);
    let expected = expected_round0_proposer(&validators, height);

    for entry in validators.iter_mut() {
        if entry.public_key.eq_ignore_ascii_case(&proposer_hex) {
            entry.blocks_produced = entry.blocks_produced.saturating_add(1);
        }
    }
    if let Some(expected_pubkey) = expected {
        if !expected_pubkey.eq_ignore_ascii_case(&proposer_hex) {
            if let Some(entry) = validators
                .iter_mut()
                .find(|v| v.public_key.eq_ignore_ascii_case(&expected_pubkey))
            {
                entry.blocks_missed = entry.blocks_missed.saturating_add(1);
            }
        }
    }
    persist_validators(trie, &validators).await
}

/// Deterministic round-0 proposer for a height, mirroring BFT's `select_proposer`
/// (sorted public keys, index = LE u64 of SHA-256(height || 0) mod n).
fn expected_round0_proposer(validators: &[ValidatorEntry], height: u64) -> Option<String> {
    let mut pubkeys: Vec<&str> = validators
        .iter()
        .filter(|v| v.is_active && !v.public_key.is_empty())
        .map(|v| v.public_key.as_str())
        .collect();
    if pubkeys.is_empty() {
        return None;
    }
    pubkeys.sort();

    let mut seed_input = Vec::with_capacity(16);
    seed_input.extend_from_slice(&height.to_le_bytes());
    seed_input.extend_from_slice(&0u64.to_le_bytes());
    let seed = common::crypto::hash(&seed_input);
    let index =
        u64::from_le_bytes(seed[..8].try_into().unwrap_or([0u8; 8])) as usize % pubkeys.len();
    Some(pubkeys[index].to_string())
}

/// Load active validators for BFT / finality hot-reload (stake + delegated).
pub async fn load_consensus_validators(
    trie: &Arc<Mutex<PatriciaTrie>>,
) -> Result<Vec<consensus::ValidatorInfo>, String> {
    let entries = load_validators(trie).await?;
    Ok(entries
        .into_iter()
        .filter(|v| v.is_active)
        .filter_map(|v| {
            let public_key = hex::decode(&v.public_key).ok()?;
            if public_key.is_empty() {
                return None;
            }
            let base = parse_stake_value(&v.stake);
            let delegated: u128 = v.total_delegated.parse().unwrap_or(0);
            // Keep the full wei stake as u128. Clamping to u64::MAX would give
            // every validator identical (maximum) voting power, so a single
            // validator could finalize solo — a safety violation that forked
            // the network.
            let total = base.saturating_add(delegated);
            Some(consensus::ValidatorInfo {
                public_key,
                stake: total.max(1),
                slashed: false,
            })
        })
        .collect())
}
