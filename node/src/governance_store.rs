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

use crate::runtime_upgrade::RuntimeVersion;

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
    let mut params = GovernanceParams::default();
    let explicit_period: Option<u64> = std::env::var("GOVERNANCE_VOTING_PERIOD_BLOCKS")
        .ok()
        .and_then(|v| v.parse().ok());
    if let Some(n) = explicit_period {
        params.voting_period_blocks = n;
    }
    if let Ok(v) = std::env::var("GOVERNANCE_QUORUM_BPS") {
        if let Ok(n) = v.parse() {
            params.quorum_threshold_bps = n;
        }
    }

    let mut total_stake = total_stake(trie).await;
    if std::env::var("GOVERNANCE_TEST_MODE")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    {
        // A short default period for tests, unless one was set explicitly.
        if explicit_period.is_none() {
            params.voting_period_blocks = params.voting_period_blocks.min(20);
        }
        params.quorum_threshold_bps = params.quorum_threshold_bps.min(100);
        total_stake = total_stake.min(1_000_000);
    }

    let mut state = ChainGovernance::new(params, SEED_TREASURY_BALANCE, total_stake);

    let validators = validator_stakes(trie).await;
    let (v1, v2, v3) = match validators.as_slice() {
        [a, b, c, ..] => (*a, *b, *c),
        [a, b] => (*a, *b, (b.0, 0)),
        [a] => (*a, (a.0, 0), (a.0, 0)),
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
    chain_store: &Arc<ChainStore>,
    extrinsics: &[common::types::Transaction],
    current_block: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = load_or_init(trie).await?;
    let mut changed = false;

    for tx in extrinsics {
        let body = tx.payload.as_slice();
        if crate::bridge_mint::is_bridge_mint_payload(body) {
            match crate::bridge_mint::apply(trie, chain_store, tx).await {
                Ok(amount) => info!("🌉 Bridge mint of {} at block {}", amount, current_block),
                Err(e) => warn!("Bridge mint rejected at block {}: {}", current_block, e),
            }
            continue;
        }
        if is_staking_payload(body) {
            if let Err(e) = apply_staking_action(trie, chain_store, tx, body).await {
                warn!(
                    "Staking transaction rejected at block {}: {}",
                    current_block, e
                );
            }
            continue;
        }
        if !ChainGovernance::is_governance_payload(body) {
            continue;
        }
        let power = voter_power(trie, chain_store, &tx.sender).await;
        match state.apply_payload(tx.sender, body, power, current_block) {
            Ok(()) => {
                info!(
                    "Governance action applied at block {} (voter {} power {})",
                    current_block,
                    hex::encode(&tx.sender[..6]),
                    power
                );
                changed = true;
                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) {
                    if value.get("action").and_then(|a| a.as_str()) == Some("execute") {
                        if let Some(id) = value.get("proposal_id").and_then(|v| v.as_u64()) {
                            if let Err(e) =
                                on_proposal_executed(trie, &state, id, current_block).await
                            {
                                warn!("Post-execute hook failed for proposal {}: {}", id, e);
                            }
                        }
                    }
                }
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

/// Staking actions carried as signed transaction payloads:
///
/// - `{"action":"register_validator","stake":"<wei>","commission_rate":10}`
///   registers the sender as a validator, with the transaction's public key
///   as its consensus key, bonding `stake` from the sender's balance.
/// - `{"action":"delegate","validator":"0x…","amount":"<wei>"}` bonds
///   `amount` from the sender's balance to a registered validator.
///
/// They used to be RPC calls that wrote any stake into one node's state
/// without moving any tokens.
pub fn is_staking_payload(payload: &[u8]) -> bool {
    staking_action(payload).is_some()
}

fn staking_action(payload: &[u8]) -> Option<(String, serde_json::Value)> {
    if payload.first() != Some(&b'{') {
        return None;
    }
    let value: serde_json::Value = serde_json::from_slice(payload).ok()?;
    let action = value.get("action")?.as_str()?.to_string();
    matches!(action.as_str(), "register_validator" | "delegate").then_some((action, value))
}

/// Tokens a staking payload bonds, if it is a well-formed staking action.
pub fn staking_bond_amount(payload: &[u8]) -> Option<u128> {
    let (action, value) = staking_action(payload)?;
    let field = if action == "register_validator" {
        "stake"
    } else {
        "amount"
    };
    parse_amount(&value, field).ok()
}

fn parse_amount(value: &serde_json::Value, field: &str) -> Result<u128, String> {
    value
        .get(field)
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u128>().ok())
        .filter(|a| *a > 0)
        .ok_or_else(|| format!("Missing or invalid {}", field))
}

/// Debit `amount` from `address`, failing (and changing nothing) if the
/// balance is too low.
fn bond(chain_store: &ChainStore, address: &Address, amount: u128) -> Result<(), String> {
    let (balance, nonce) = crate::accounts::load(chain_store, address);
    let rest = balance
        .checked_sub(amount)
        .ok_or_else(|| format!("Insufficient balance to bond {}", amount))?;
    crate::accounts::store(chain_store, address, rest, nonce).map_err(|e| e.to_string())
}

async fn apply_staking_action(
    trie: &Arc<Mutex<PatriciaTrie>>,
    chain_store: &Arc<ChainStore>,
    tx: &common::types::Transaction,
    payload: &[u8],
) -> Result<(), String> {
    let (action, value) = staking_action(payload).ok_or("Not a staking action")?;
    match action.as_str() {
        "register_validator" => {
            let stake = parse_amount(&value, "stake")?;
            let commission = value
                .get("commission_rate")
                .and_then(|v| v.as_u64())
                .unwrap_or(10)
                .min(100);
            let (balance, _) = crate::accounts::load(chain_store, &tx.sender);
            if balance < stake {
                return Err(format!("Insufficient balance to bond {}", stake));
            }
            register_validator(trie, tx.sender, tx.public_key.to_vec(), stake, commission).await?;
            bond(chain_store, &tx.sender, stake)?;
            info!(
                "Validator {} registered with stake {}",
                hex::encode(&tx.sender[..6]),
                stake
            );
            Ok(())
        }
        "delegate" => {
            let amount = parse_amount(&value, "amount")?;
            let validator = value
                .get("validator")
                .and_then(|v| v.as_str())
                .and_then(|s| hex::decode(s.trim_start_matches("0x")).ok())
                .and_then(|b| <[u8; 20]>::try_from(b.as_slice()).ok())
                .ok_or("Missing or invalid validator")?;
            let (balance, _) = crate::accounts::load(chain_store, &tx.sender);
            if balance < amount {
                return Err(format!("Insufficient balance to bond {}", amount));
            }
            apply_delegate(trie, chain_store, tx.sender, validator, amount).await?;
            bond(chain_store, &tx.sender, amount)?;
            Ok(())
        }
        other => Err(format!("Unknown staking action {}", other)),
    }
}

fn parse_spec_version(version: &str) -> u32 {
    version
        .split('.')
        .filter_map(|part| part.trim().split('-').next()?.parse().ok())
        .next()
        .unwrap_or(1)
}

fn parse_code_hash(hash: &str) -> Result<[u8; 32], String> {
    let bytes = hex::decode(hash.trim_start_matches("0x")).map_err(|e| e.to_string())?;
    if bytes.len() != 32 {
        return Err("Software upgrade hash must be 32 bytes".into());
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

/// Side effects when a governance proposal is marked executed on-chain.
async fn on_proposal_executed(
    trie: &Arc<Mutex<PatriciaTrie>>,
    state: &ChainGovernance,
    proposal_id: u64,
    current_block: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(proposal) = state.get_proposal(proposal_id) else {
        return Ok(());
    };
    let ProposalType::SoftwareUpgrade { version, hash } = &proposal.proposal_type else {
        return Ok(());
    };

    let code_hash =
        parse_code_hash(hash).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let current = crate::runtime_upgrade_store::current_version(trie).await?;
    let parsed = parse_spec_version(version);
    let spec_version = parsed.max(current.spec_version.saturating_add(1));

    let new_version = RuntimeVersion {
        spec_name: current.spec_name.clone(),
        impl_name: current.impl_name.clone(),
        authoring_version: current.authoring_version,
        spec_version,
        impl_version: spec_version,
    };
    let activation_height = current_block.saturating_add(10);

    let upgrade_id = crate::runtime_upgrade_store::propose_upgrade(
        trie,
        new_version.clone(),
        code_hash,
        activation_height,
        proposal.proposer,
    )
    .await?;
    crate::runtime_upgrade_store::approve_upgrade(trie, upgrade_id).await?;

    info!(
        "Software upgrade proposal {} executed — runtime upgrade {} approved for height {} (spec v{})",
        proposal_id, upgrade_id, activation_height, spec_version
    );
    Ok(())
}

/// Voting power of an account: validator self-stake + delegations to them,
/// else delegated stake as delegator, else native token balance (1 wei = 1 vote).
pub async fn voter_power(
    trie: &Arc<Mutex<PatriciaTrie>>,
    chain_store: &Arc<ChainStore>,
    address: &Address,
) -> u128 {
    if let Some((_, power)) = validator_stakes(trie)
        .await
        .into_iter()
        .find(|(addr, _)| addr == address)
    {
        return power;
    }

    let delegator_hex = format!("0x{}", hex::encode(address));
    let key = [b"delegations/", delegator_hex.as_bytes()].concat();
    if let Ok(Some(encoded)) = chain_store.get_state(&key) {
        if let Ok(list) = serde_json::from_slice::<Vec<serde_json::Value>>(&encoded) {
            let delegated: u128 = list
                .iter()
                .filter_map(|e| e.get("amount")?.as_str()?.parse::<u128>().ok())
                .sum();
            if delegated > 0 {
                return delegated;
            }
        }
    }

    native_balance(chain_store, address)
}

fn native_balance(chain_store: &ChainStore, address: &Address) -> u128 {
    chain_store
        .get_state(address)
        .ok()
        .flatten()
        .and_then(|encoded| serde_json::from_slice::<serde_json::Value>(&encoded).ok())
        .and_then(|acc| acc.get("balance")?.as_str()?.parse().ok())
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
    amount: u128,
) -> Result<(), Box<dyn std::error::Error>> {
    if amount == 0 {
        return Ok(());
    }
    let mut state = load_or_init(trie).await?;
    state.treasury.balance = state.treasury.balance.saturating_add(amount);
    state.treasury.total_collected = state.treasury.total_collected.saturating_add(amount);
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
    entry.total_delegated = current
        .checked_add(amount)
        .ok_or("Delegation total overflow")?
        .to_string();
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

/// Record block production stats for the finalized block at `slot`.
///
/// - Increments `blocks_produced` for the actual proposer (matched by public key).
/// - Increments `blocks_missed` for the validator the deterministic round-0
///   leader schedule expected to propose this height but did not.
pub async fn record_block_stats(
    trie: &Arc<Mutex<PatriciaTrie>>,
    slot: u64,
    proposer_pubkey: &[u8],
) -> Result<(), String> {
    let mut validators = load_validators(trie).await?;
    if validators.is_empty() {
        return Ok(());
    }
    let proposer_hex = hex::encode(proposer_pubkey);
    // BFT height h finalizes the block at slot h - 1, and the proposer is
    // chosen by BFT height.
    let expected = expected_round0_proposer(&validators, slot.saturating_add(1));

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

/// Deterministic round-0 proposer for a height, using the same stake-weighted
/// selection as BFT (`common::consensus_types::select_proposer`) over the same
/// stake (own + delegated) that `load_consensus_validators` feeds the engine.
fn expected_round0_proposer(validators: &[ValidatorEntry], height: u64) -> Option<String> {
    let set: Vec<(Vec<u8>, u128)> = validators
        .iter()
        .filter(|v| v.is_active && !v.public_key.is_empty())
        .filter_map(|v| {
            let pk = hex::decode(&v.public_key).ok()?;
            let delegated: u128 = v.total_delegated.parse().unwrap_or(0);
            let stake = parse_stake_value(&v.stake).saturating_add(delegated).max(1);
            Some((pk, stake))
        })
        .collect();
    common::consensus_types::select_proposer(
        set.iter().map(|(pk, s)| (pk.as_slice(), *s)),
        height,
        0,
    )
    .map(hex::encode)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn voter_power_uses_native_balance_for_regular_accounts() {
        let inner = Arc::new(storage::MemDb::new());
        let chain_store = Arc::new(ChainStore::new(inner.clone()));
        let trie = Arc::new(Mutex::new(PatriciaTrie::new(inner).unwrap()));
        let addr = [9u8; 20];
        let account = serde_json::json!({
            "balance": "5000000000000000000",
            "nonce": 0,
        });
        chain_store
            .put_state(&addr, &serde_json::to_vec(&account).unwrap())
            .unwrap();
        let power = voter_power(&trie, &chain_store, &addr).await;
        assert_eq!(power, 5_000_000_000_000_000_000);
    }

    #[tokio::test]
    async fn on_schedule_proposers_are_not_counted_as_missed() {
        let inner = Arc::new(storage::MemDb::new());
        let trie = Arc::new(Mutex::new(PatriciaTrie::new(inner).unwrap()));
        let set: Vec<serde_json::Value> = (1u8..=4)
            .map(|i| {
                serde_json::json!({
                    "address": format!("0x{:040x}", i),
                    "public_key": hex::encode([i; 32]),
                    "stake": (u128::from(i) * 1_000_000_000_000_000_000).to_string(),
                    "is_active": true,
                    "total_delegated": "0",
                })
            })
            .collect();
        trie.lock()
            .await
            .insert(b"validators", &serde_json::to_vec(&set).unwrap())
            .unwrap();

        // The block at slot s is proposed at BFT height s + 1, round 0.
        let engine_set = load_consensus_validators(&trie).await.unwrap();
        for slot in 1..=40u64 {
            let proposer = common::consensus_types::select_proposer(
                engine_set
                    .iter()
                    .map(|v| (v.public_key.as_slice(), v.stake)),
                slot + 1,
                0,
            )
            .unwrap();
            record_block_stats(&trie, slot, &proposer).await.unwrap();
        }
        let after = load_validators(&trie).await.unwrap();
        assert!(after.iter().all(|v| v.blocks_missed == 0));
        assert_eq!(after.iter().map(|v| v.blocks_produced).sum::<u64>(), 40);
    }

    fn staking_tx(seed: u8, payload: serde_json::Value) -> common::types::Transaction {
        let key = ed25519_dalek::SigningKey::from_bytes(&[seed; 32]);
        let mut tx = common::types::Transaction::simple_transfer([0; 20], [0; 20], 0, 0, 1);
        tx.to = None;
        tx.payload = serde_json::to_vec(&payload).unwrap();
        tx.sign_with(&key);
        tx
    }

    #[tokio::test]
    async fn register_validator_tx_bonds_stake() {
        let inner = Arc::new(storage::MemDb::new());
        let chain_store = Arc::new(ChainStore::new(inner.clone()));
        let trie = Arc::new(Mutex::new(PatriciaTrie::new(inner).unwrap()));
        trie.lock().await.insert(b"validators", b"[]").unwrap();

        let stake = 2_000_000_000_000_000_000u128;
        let tx = staking_tx(
            11,
            serde_json::json!({"action": "register_validator", "stake": stake.to_string()}),
        );
        assert!(is_staking_payload(&tx.payload));
        assert_eq!(staking_bond_amount(&tx.payload), Some(stake));

        // Not enough balance: rejected, nothing changes.
        crate::accounts::store(&chain_store, &tx.sender, stake - 1, 1).unwrap();
        apply_extrinsics(&trie, &chain_store, std::slice::from_ref(&tx), 5)
            .await
            .unwrap();
        assert_eq!(load_validators(&trie).await.unwrap().len(), 0);

        crate::accounts::store(&chain_store, &tx.sender, stake + 10, 1).unwrap();
        apply_extrinsics(&trie, &chain_store, std::slice::from_ref(&tx), 6)
            .await
            .unwrap();
        let validators = load_validators(&trie).await.unwrap();
        assert_eq!(validators.len(), 1);
        assert_eq!(validators[0].public_key, hex::encode(tx.public_key));
        assert_eq!(crate::accounts::load(&chain_store, &tx.sender), (10, 1));
    }

    #[tokio::test]
    async fn delegate_tx_bonds_amount() {
        let inner = Arc::new(storage::MemDb::new());
        let chain_store = Arc::new(ChainStore::new(inner.clone()));
        let trie = Arc::new(Mutex::new(PatriciaTrie::new(inner).unwrap()));
        let validator = format!("0x{}", hex::encode([3u8; 20]));
        let set = serde_json::json!([{ "address": validator, "stake": "1" }]);
        trie.lock()
            .await
            .insert(b"validators", &serde_json::to_vec(&set).unwrap())
            .unwrap();

        let tx = staking_tx(
            12,
            serde_json::json!({"action": "delegate", "validator": validator, "amount": "40"}),
        );
        crate::accounts::store(&chain_store, &tx.sender, 100, 0).unwrap();
        apply_extrinsics(&trie, &chain_store, std::slice::from_ref(&tx), 1)
            .await
            .unwrap();
        assert_eq!(crate::accounts::load(&chain_store, &tx.sender), (60, 0));
        assert_eq!(
            load_validators(&trie).await.unwrap()[0].total_delegated,
            "40"
        );
    }
}
