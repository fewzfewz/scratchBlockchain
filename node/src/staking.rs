//! Staking lifecycle: epochs, unbonding, jailing and double-sign slashing.
//!
//! Everything here runs inside block application (the staged state of
//! `stage_block_effects`), reads and writes only the state trie and account
//! state, and so is covered by the state root like any other transition.
//!
//! * **Epochs.** The validator registry (`b"validators"`) changes whenever a
//!   staking transaction lands, but consensus uses a snapshot of it,
//!   `b"validators/epoch"`, taken at the start of every `EPOCH_BLOCKS`-th
//!   block. So the set that certifies block `s + 1` is known from the state
//!   before block `s`, and block `s` commits to it in
//!   `header.next_validators_hash`.
//! * **Unbonding.** `unbond` (own stake) and `undelegate` move stake into a
//!   queue; it returns to the owner's balance `UNBONDING_BLOCKS` later, and a
//!   validator caught double-signing loses part of its queued stake too.
//! * **Jailing.** A validator that misses its proposer turns for a sustained
//!   time (`MAX_MISSED_TURNS` in a row spanning `MIN_DOWNTIME_BLOCKS`) — not
//!   a node restart — misses `MAX_MISSED_TURNS` proposer turns in
//!   a row leaves the set at the next epoch; it can `unjail` itself after
//!   `JAIL_BLOCKS`.
//! * **Slashing.** Anyone can submit two conflicting signed votes from one
//!   validator (`evidence`). Every node checks them; the validator loses
//!   `DOUBLE_SIGN_SLASH_BPS` of its own stake and is removed for good.

use common::consensus_types::{validator_set_hash, DoubleSignEvidence, ValidatorInfo};
use common::types::{Address, Hash};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use storage::trie::PatriciaTrie;
use storage::ChainStore;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::governance_store::{
    load_validators, parse_stake_value, persist_validators, ValidatorEntry,
};

/// Blocks per epoch; validator-set changes take effect at the next multiple.
pub const EPOCH_BLOCKS: u64 = 20;
/// Blocks before unbonded stake is returned.
pub const UNBONDING_BLOCKS: u64 = 100;
/// Blocks a jailed validator must wait before `unjail`.
pub const JAIL_BLOCKS: u64 = 50;
/// Consecutive missed proposer turns that get a validator jailed.
pub const MAX_MISSED_TURNS: u64 = 10;
/// ...and the streak must span at least this many blocks, so a node restart
/// (a few seconds; a large validator misses a few turns) does not jail it.
/// Missing 3 turns in a row once jailed a genesis validator during a routine
/// restart. Measured in blocks, the window must stay short: an offline
/// validator slows the chain (each of its turns waits out a timeout).
pub const MIN_DOWNTIME_BLOCKS: u64 = 50;
/// Share of own stake (and own queued unbonding) burned for double signing.
pub const DOUBLE_SIGN_SLASH_BPS: u128 = 500;
/// Smallest own stake a validator may keep (1 NBL).
pub const MIN_SELF_STAKE: u128 = 1_000_000_000_000_000_000;

const EPOCH_SET_KEY: &[u8] = b"validators/epoch";
const UNBONDING_KEY: &[u8] = b"staking/unbonding";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnbondingEntry {
    /// Account the stake returns to (`0x…`).
    pub owner: String,
    /// Validator the stake was bonded to (`0x…`).
    pub validator: String,
    pub amount: String,
    /// First block at which the stake is returned.
    pub release_at: u64,
}

pub fn is_epoch_boundary(slot: u64) -> bool {
    slot.is_multiple_of(EPOCH_BLOCKS)
}

fn hex_addr(address: &Address) -> String {
    format!("0x{}", hex::encode(address))
}

fn own_stake(v: &ValidatorEntry) -> u128 {
    parse_stake_value(&v.stake)
}

fn total_stake(v: &ValidatorEntry) -> u128 {
    own_stake(v).saturating_add(v.total_delegated.parse().unwrap_or(0))
}

/// May take part in consensus: active, not jailed or tombstoned, and still
/// holding the minimum own stake.
pub(crate) fn is_eligible(v: &ValidatorEntry) -> bool {
    v.is_active
        && !v.jailed
        && !v.tombstoned
        && own_stake(v) >= MIN_SELF_STAKE
        && !v.public_key.is_empty()
}

/// The consensus set the registry describes right now.
pub(crate) fn registry_set(entries: &[ValidatorEntry]) -> Vec<ValidatorInfo> {
    entries
        .iter()
        .filter(|v| is_eligible(v))
        .filter_map(|v| {
            let public_key = hex::decode(&v.public_key).ok()?;
            // Keep the full wei stake as u128: clamping would give every
            // validator the same voting power.
            Some(ValidatorInfo {
                public_key,
                stake: total_stake(v).max(1),
                slashed: false,
            })
        })
        .collect()
}

async fn load_epoch_set(
    trie: &Arc<Mutex<PatriciaTrie>>,
) -> Result<Option<Vec<ValidatorInfo>>, String> {
    let guard = trie.lock().await;
    match guard.get(EPOCH_SET_KEY).map_err(|e| e.to_string())? {
        Some(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

/// The validator set consensus uses now: the epoch snapshot (or, on a chain
/// that has not applied a block since this was introduced, the registry).
pub async fn consensus_set(trie: &Arc<Mutex<PatriciaTrie>>) -> Result<Vec<ValidatorInfo>, String> {
    if let Some(set) = load_epoch_set(trie).await? {
        return Ok(set);
    }
    Ok(registry_set(&load_validators(trie).await?))
}

/// Hash of the set that certifies block `slot + 1`, computed from the state
/// before block `slot` (what `begin_block(slot)` will leave in place).
pub async fn next_validators_hash(
    trie: &Arc<Mutex<PatriciaTrie>>,
    slot: u64,
) -> Result<Hash, String> {
    let snapshot_now = is_epoch_boundary(slot) || load_epoch_set(trie).await?.is_none();
    let set = if snapshot_now {
        registry_set(&load_validators(trie).await?)
    } else {
        consensus_set(trie).await?
    };
    Ok(validator_set_hash(&set))
}

/// Start of block `slot`, before its transactions: take the epoch snapshot
/// if due and return matured unbonding stake. Returns the set that
/// certified this block (the snapshot before any rotation), for proposer
/// accounting.
pub async fn begin_block(
    trie: &Arc<Mutex<PatriciaTrie>>,
    chain_store: &ChainStore,
    slot: u64,
) -> Result<Vec<ValidatorInfo>, String> {
    let previous = load_epoch_set(trie).await?;
    let certifying = match &previous {
        Some(set) => set.clone(),
        None => registry_set(&load_validators(trie).await?),
    };
    if previous.is_none() || is_epoch_boundary(slot) {
        let next = registry_set(&load_validators(trie).await?);
        if next.is_empty() {
            // Never rotate to an empty set (the chain could not continue).
            warn!("Epoch at block {}: registry has no eligible validator, keeping the old set", slot);
        } else {
            if validator_set_hash(&next) != validator_set_hash(&certifying) {
                info!(
                    "🔁 Epoch at block {}: validator set now {} validators",
                    slot,
                    next.len()
                );
            }
            let bytes = serde_json::to_vec(&next).map_err(|e| e.to_string())?;
            trie.lock()
                .await
                .insert(EPOCH_SET_KEY, &bytes)
                .map_err(|e| e.to_string())?;
        }
    }
    release_unbonded(trie, chain_store, slot).await?;
    Ok(certifying)
}

async fn load_unbonding(trie: &Arc<Mutex<PatriciaTrie>>) -> Result<Vec<UnbondingEntry>, String> {
    let guard = trie.lock().await;
    match guard.get(UNBONDING_KEY).map_err(|e| e.to_string())? {
        Some(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
        None => Ok(Vec::new()),
    }
}

async fn persist_unbonding(
    trie: &Arc<Mutex<PatriciaTrie>>,
    entries: &[UnbondingEntry],
) -> Result<(), String> {
    let bytes = serde_json::to_vec(entries).map_err(|e| e.to_string())?;
    trie.lock()
        .await
        .insert(UNBONDING_KEY, &bytes)
        .map_err(|e| e.to_string())
}

/// Pending unbonding entries (for the RPC).
pub async fn unbonding_queue(
    trie: &Arc<Mutex<PatriciaTrie>>,
) -> Result<Vec<UnbondingEntry>, String> {
    load_unbonding(trie).await
}

async fn release_unbonded(
    trie: &Arc<Mutex<PatriciaTrie>>,
    chain_store: &ChainStore,
    slot: u64,
) -> Result<(), String> {
    let entries = load_unbonding(trie).await?;
    if !entries.iter().any(|e| e.release_at <= slot) {
        return Ok(());
    }
    let (due, pending): (Vec<_>, Vec<_>) = entries.into_iter().partition(|e| e.release_at <= slot);
    for entry in &due {
        let amount: u128 = entry.amount.parse().unwrap_or(0);
        let owner = parse_address(&entry.owner)?;
        let (balance, nonce) = crate::accounts::load(chain_store, &owner);
        let credited = balance
            .checked_add(amount)
            .ok_or("balance overflow releasing unbonded stake")?;
        crate::accounts::store(chain_store, &owner, credited, nonce).map_err(|e| e.to_string())?;
        info!(
            "🔓 Released {} unbonded to {} at block {}",
            amount, entry.owner, slot
        );
    }
    persist_unbonding(trie, &pending).await
}

fn parse_address(s: &str) -> Result<Address, String> {
    hex::decode(s.trim_start_matches("0x"))
        .ok()
        .and_then(|b| <[u8; 20]>::try_from(b.as_slice()).ok())
        .ok_or_else(|| format!("invalid address {}", s))
}

async fn queue_unbonding(
    trie: &Arc<Mutex<PatriciaTrie>>,
    owner: &Address,
    validator: &str,
    amount: u128,
    slot: u64,
) -> Result<(), String> {
    let mut queue = load_unbonding(trie).await?;
    queue.push(UnbondingEntry {
        owner: hex_addr(owner),
        validator: validator.to_string(),
        amount: amount.to_string(),
        release_at: slot.saturating_add(UNBONDING_BLOCKS),
    });
    persist_unbonding(trie, &queue).await
}

/// The registry entry of the validator sending a tx: matched by its address
/// or by the key that signed the tx (genesis validators carry placeholder
/// addresses that are not derived from their keys, so they could otherwise
/// never unbond or unjail).
fn own_index(validators: &[ValidatorEntry], sender: &Address, public_key: &[u8; 32]) -> Option<usize> {
    let me = hex_addr(sender);
    let pk = hex::encode(public_key);
    validators
        .iter()
        .position(|v| v.address.eq_ignore_ascii_case(&me) || v.public_key.eq_ignore_ascii_case(&pk))
}

/// `unbond`: a validator withdraws `amount` of its own stake. It must keep
/// `MIN_SELF_STAKE` or withdraw everything (which deactivates it), and the
/// last eligible validator cannot leave.
pub async fn unbond(
    trie: &Arc<Mutex<PatriciaTrie>>,
    sender: &Address,
    public_key: &[u8; 32],
    amount: u128,
    slot: u64,
) -> Result<(), String> {
    let mut validators = load_validators(trie).await?;
    let me = hex_addr(sender);
    let idx = own_index(&validators, sender, public_key).ok_or("sender is not a validator")?;
    let current = own_stake(&validators[idx]);
    let rest = current
        .checked_sub(amount)
        .ok_or_else(|| format!("cannot unbond {} of {} staked", amount, current))?;
    if rest != 0 && rest < MIN_SELF_STAKE {
        return Err("remaining stake would be below the 1 NBL minimum; unbond all of it to leave".into());
    }
    let was_eligible = is_eligible(&validators[idx]);
    validators[idx].stake = serde_json::Value::String(rest.to_string());
    if rest == 0 {
        validators[idx].is_active = false;
    }
    if was_eligible && registry_set(&validators).is_empty() {
        return Err("the last validator cannot leave".into());
    }
    let validator = validators[idx].address.clone();
    persist_validators(trie, &validators).await?;
    queue_unbonding(trie, sender, &validator, amount, slot).await?;
    info!("Validator {} unbonding {} at block {}", me, amount, slot);
    Ok(())
}

/// `undelegate`: a delegator withdraws `amount` from `validator`.
pub async fn undelegate(
    trie: &Arc<Mutex<PatriciaTrie>>,
    chain_store: &ChainStore,
    sender: &Address,
    validator: &Address,
    amount: u128,
    slot: u64,
) -> Result<(), String> {
    let delegator = hex_addr(sender);
    let val_hex = hex_addr(validator);
    let key = [b"delegations/", delegator.as_bytes()].concat();
    let mut list: Vec<serde_json::Value> = chain_store
        .get_state(&key)
        .ok()
        .flatten()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let is_mine = |d: &serde_json::Value| {
        d.get("validator_address")
            .and_then(|v| v.as_str())
            .is_some_and(|v| v.eq_ignore_ascii_case(&val_hex))
    };
    let amount_of = |d: &serde_json::Value| -> u128 {
        d.get("amount")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    };
    let delegated: u128 = list.iter().filter(|d| is_mine(d)).map(amount_of).sum();
    if amount > delegated {
        return Err(format!(
            "cannot undelegate {}: {} delegated to {}",
            amount, delegated, val_hex
        ));
    }

    // Take from this validator's entries, oldest first.
    let mut left = amount;
    for d in list.iter_mut().filter(|d| is_mine(d)) {
        if left == 0 {
            break;
        }
        let have = amount_of(d);
        let take = have.min(left);
        left -= take;
        d["amount"] = serde_json::Value::String((have - take).to_string());
    }
    list.retain(|d| !(is_mine(d) && amount_of(d) == 0));
    let fully_left = !list.iter().any(is_mine);

    let mut validators = load_validators(trie).await?;
    let entry = validators
        .iter_mut()
        .find(|v| v.address.eq_ignore_ascii_case(&val_hex))
        .ok_or("validator not found")?;
    let total: u128 = entry.total_delegated.parse().unwrap_or(0);
    entry.total_delegated = total.saturating_sub(amount).to_string();
    if fully_left {
        entry.delegator_count = entry.delegator_count.saturating_sub(1);
    }
    persist_validators(trie, &validators).await?;
    queue_unbonding(trie, sender, &val_hex, amount, slot).await?;
    // Last: the caller undoes trie writes if anything above fails, but not
    // this account-state write.
    chain_store
        .put_state(&key, &serde_json::to_vec(&list).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

/// `unjail`: a jailed validator rejoins (at the next epoch) once its jail
/// time is served.
pub async fn unjail(
    trie: &Arc<Mutex<PatriciaTrie>>,
    sender: &Address,
    public_key: &[u8; 32],
    slot: u64,
) -> Result<(), String> {
    let mut validators = load_validators(trie).await?;
    let me = hex_addr(sender);
    let idx = own_index(&validators, sender, public_key).ok_or("sender is not a validator")?;
    let entry = &mut validators[idx];
    if entry.tombstoned {
        return Err("validator was slashed for double signing and cannot return".into());
    }
    if !entry.jailed {
        return Err("validator is not jailed".into());
    }
    if slot < entry.jailed_until {
        return Err(format!("jailed until block {}", entry.jailed_until));
    }
    if own_stake(entry) < MIN_SELF_STAKE {
        return Err("own stake is below the 1 NBL minimum".into());
    }
    entry.jailed = false;
    entry.missed_in_row = 0;
    persist_validators(trie, &validators).await?;
    info!("Validator {} unjailed at block {}", me, slot);
    Ok(())
}

/// `evidence`: slash and permanently remove a validator that signed two
/// conflicting votes. Evidence older than the unbonding period is refused
/// (the stake it could slash may already be gone).
pub async fn apply_evidence(
    trie: &Arc<Mutex<PatriciaTrie>>,
    evidence: &DoubleSignEvidence,
    slot: u64,
) -> Result<u128, String> {
    if !evidence.is_valid() {
        return Err("evidence is not two validly signed conflicting votes".into());
    }
    if evidence.first.height.saturating_add(UNBONDING_BLOCKS) < slot {
        return Err(format!(
            "evidence from height {} is too old",
            evidence.first.height
        ));
    }
    let voter = hex::encode(&evidence.first.voter);
    let mut validators = load_validators(trie).await?;
    let entry = validators
        .iter_mut()
        .find(|v| v.public_key.eq_ignore_ascii_case(&voter))
        .ok_or("evidence is for a key that is not a validator")?;
    if entry.tombstoned {
        return Err("validator was already slashed for double signing".into());
    }
    let stake = own_stake(entry);
    let penalty = stake.saturating_mul(DOUBLE_SIGN_SLASH_BPS) / 10_000;
    entry.stake = serde_json::Value::String((stake - penalty).to_string());
    entry.tombstoned = true;
    entry.jailed = true;
    entry.jailed_until = u64::MAX;
    let mut slashed: u128 = entry.slashed.parse().unwrap_or(0);
    let address = entry.address.clone();

    // Stake the validator is already withdrawing is slashed too, so
    // unbonding right before getting caught does not escape the penalty.
    let mut queue = load_unbonding(trie).await?;
    let mut queued_penalty = 0u128;
    for e in queue
        .iter_mut()
        .filter(|e| e.validator.eq_ignore_ascii_case(&address) && e.owner.eq_ignore_ascii_case(&address))
    {
        let amount: u128 = e.amount.parse().unwrap_or(0);
        let cut = amount.saturating_mul(DOUBLE_SIGN_SLASH_BPS) / 10_000;
        e.amount = (amount - cut).to_string();
        queued_penalty += cut;
    }
    slashed = slashed.saturating_add(penalty).saturating_add(queued_penalty);
    if let Some(entry) = validators
        .iter_mut()
        .find(|v| v.address.eq_ignore_ascii_case(&address))
    {
        entry.slashed = slashed.to_string();
    }
    persist_validators(trie, &validators).await?;
    persist_unbonding(trie, &queue).await?;
    warn!(
        "⚔️ Validator {} slashed {} for double signing at height {} (block {})",
        address,
        penalty + queued_penalty,
        evidence.first.height,
        slot
    );
    Ok(penalty + queued_penalty)
}

/// Proposer-turn accounting for block `slot`, against `certifying` (the set
/// that chose its proposer). Resets the proposer's miss streak; a validator
/// that should have proposed and did not gets a miss, and enough misses in a
/// row jail it (unless that would leave no eligible validator).
pub async fn record_turn(
    trie: &Arc<Mutex<PatriciaTrie>>,
    certifying: &[ValidatorInfo],
    slot: u64,
    proposer_pubkey: &[u8],
) -> Result<(), String> {
    let mut validators = load_validators(trie).await?;
    if validators.is_empty() {
        return Ok(());
    }
    let proposer_hex = hex::encode(proposer_pubkey);
    // BFT height h finalizes the block at slot h - 1; round 0's proposer is
    // the one scheduled.
    let expected = common::consensus_types::select_proposer(
        certifying.iter().map(|v| (v.public_key.as_slice(), v.stake)),
        slot.saturating_add(1),
        0,
    )
    .map(hex::encode);

    for entry in validators.iter_mut() {
        if entry.public_key.eq_ignore_ascii_case(&proposer_hex) {
            entry.blocks_produced = entry.blocks_produced.saturating_add(1);
            entry.missed_in_row = 0;
        }
    }
    if let Some(expected) = expected.filter(|e| !e.eq_ignore_ascii_case(&proposer_hex)) {
        let eligible = validators.iter().filter(|v| is_eligible(v)).count();
        if let Some(entry) = validators
            .iter_mut()
            .find(|v| v.public_key.eq_ignore_ascii_case(&expected))
        {
            entry.blocks_missed = entry.blocks_missed.saturating_add(1);
            if entry.missed_in_row == 0 {
                entry.missed_since = slot;
            }
            entry.missed_in_row = entry.missed_in_row.saturating_add(1);
            let sustained = slot.saturating_sub(entry.missed_since) >= MIN_DOWNTIME_BLOCKS;
            if entry.missed_in_row >= MAX_MISSED_TURNS && sustained && !entry.jailed && eligible > 1 {
                entry.jailed = true;
                entry.jailed_until = slot.saturating_add(JAIL_BLOCKS);
                warn!(
                    "⛓️ Validator {} jailed at block {} after {} missed proposer turns",
                    entry.address, slot, entry.missed_in_row
                );
            }
        }
    }
    persist_validators(trie, &validators).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::consensus_types::{Step, Vote};
    use ed25519_dalek::{Signer, SigningKey};

    const NBL: u128 = 1_000_000_000_000_000_000;

    fn setup() -> (Arc<Mutex<PatriciaTrie>>, Arc<ChainStore>) {
        let inner = Arc::new(storage::MemDb::new());
        let chain_store = Arc::new(ChainStore::new(inner.clone()));
        let trie = Arc::new(Mutex::new(PatriciaTrie::new(inner).unwrap()));
        (trie, chain_store)
    }

    fn key(i: u8) -> SigningKey {
        SigningKey::from_bytes(&[i; 32])
    }

    fn pubkey(i: u8) -> [u8; 32] {
        key(i).verifying_key().to_bytes()
    }

    fn address(i: u8) -> Address {
        common::crypto::derive_address_from_public_key(&key(i).verifying_key().to_bytes())
    }

    async fn seed(trie: &Arc<Mutex<PatriciaTrie>>, stakes: &[(u8, u128)]) {
        let set: Vec<serde_json::Value> = stakes
            .iter()
            .map(|(i, stake)| {
                serde_json::json!({
                    "address": hex_addr(&address(*i)),
                    "public_key": hex::encode(key(*i).verifying_key().to_bytes()),
                    "stake": stake.to_string(),
                    "is_active": true,
                    "total_delegated": "0",
                })
            })
            .collect();
        trie.lock()
            .await
            .insert(b"validators", &serde_json::to_vec(&set).unwrap())
            .unwrap();
    }

    fn vote(k: &SigningKey, height: u64, hash: u8) -> Vote {
        let mut v = Vote {
            height,
            round: 0,
            step: Step::Precommit,
            block_hash: Some([hash; 32]),
            signature: vec![],
            voter: k.verifying_key().to_bytes().to_vec(),
        };
        v.signature = k.sign(&v.signing_bytes()).to_bytes().to_vec();
        v
    }

    #[tokio::test]
    async fn set_changes_only_at_epoch_boundaries_and_hash_is_predicted() {
        let (trie, cs) = setup();
        seed(&trie, &[(1, 10 * NBL), (2, 10 * NBL)]).await;
        // First block snapshots the registry.
        let predicted = next_validators_hash(&trie, 1).await.unwrap();
        begin_block(&trie, &cs, 1).await.unwrap();
        assert_eq!(validator_set_hash(&consensus_set(&trie).await.unwrap()), predicted);

        // A validator leaves mid-epoch: consensus keeps the old set.
        unbond(&trie, &address(2), &pubkey(2), 10 * NBL, 1).await.unwrap();
        for slot in 2..EPOCH_BLOCKS {
            let predicted = next_validators_hash(&trie, slot).await.unwrap();
            begin_block(&trie, &cs, slot).await.unwrap();
            let set = consensus_set(&trie).await.unwrap();
            assert_eq!(set.len(), 2, "slot {}", slot);
            assert_eq!(validator_set_hash(&set), predicted);
        }
        // At the boundary the change takes effect, as predicted.
        let predicted = next_validators_hash(&trie, EPOCH_BLOCKS).await.unwrap();
        begin_block(&trie, &cs, EPOCH_BLOCKS).await.unwrap();
        let set = consensus_set(&trie).await.unwrap();
        assert_eq!(set.len(), 1);
        assert_eq!(validator_set_hash(&set), predicted);
    }

    #[tokio::test]
    async fn unbonded_stake_returns_after_the_unbonding_period() {
        let (trie, cs) = setup();
        seed(&trie, &[(1, 10 * NBL), (2, 10 * NBL)]).await;
        unbond(&trie, &address(1), &pubkey(1), 4 * NBL, 5).await.unwrap();
        // Below the minimum without leaving entirely is refused.
        assert!(unbond(&trie, &address(1), &pubkey(1), 5 * NBL + 1, 5).await.is_err());

        begin_block(&trie, &cs, 5 + UNBONDING_BLOCKS - 1).await.unwrap();
        assert_eq!(crate::accounts::load(&cs, &address(1)).0, 0);
        begin_block(&trie, &cs, 5 + UNBONDING_BLOCKS).await.unwrap();
        assert_eq!(crate::accounts::load(&cs, &address(1)).0, 4 * NBL);
        assert!(load_unbonding(&trie).await.unwrap().is_empty());
    }

    /// A restart: several turns missed in a row within a few blocks.
    #[tokio::test]
    async fn short_outage_does_not_jail() {
        let (trie, _) = setup();
        seed(&trie, &[(1, 10 * NBL), (2, 10 * NBL)]).await;
        let set = registry_set(&load_validators(&trie).await.unwrap());
        let victim = key(2).verifying_key().to_bytes();
        let other = key(1).verifying_key().to_bytes();
        let mut misses = 0;
        for slot in 1..MIN_DOWNTIME_BLOCKS {
            let expected = common::consensus_types::select_proposer(
                set.iter().map(|v| (v.public_key.as_slice(), v.stake)),
                slot + 1,
                0,
            )
            .unwrap();
            let actual = if expected == victim { misses += 1; other.to_vec() } else { expected };
            record_turn(&trie, &set, slot, &actual).await.unwrap();
        }
        assert!(misses >= 3, "test needs several misses, got {}", misses);
        assert!(load_validators(&trie).await.unwrap().iter().all(|v| !v.jailed));
    }

    #[tokio::test]
    async fn genesis_validator_with_placeholder_address_unbonds_by_key() {
        let (trie, _) = setup();
        seed(&trie, &[(1, 10 * NBL), (2, 10 * NBL)]).await;
        let mut vals = load_validators(&trie).await.unwrap();
        vals[1].address = format!("0x{}", "22".repeat(20));
        persist_validators(&trie, &vals).await.unwrap();
        unbond(&trie, &address(2), &pubkey(2), NBL, 4).await.unwrap();
        // Another key cannot act for it.
        assert!(unbond(&trie, &address(3), &pubkey(3), NBL, 4).await.is_err());
    }

    #[tokio::test]
    async fn last_validator_cannot_leave() {
        let (trie, _) = setup();
        seed(&trie, &[(1, 10 * NBL)]).await;
        assert!(unbond(&trie, &address(1), &pubkey(1), 10 * NBL, 3).await.is_err());
    }

    #[tokio::test]
    async fn missed_turns_jail_and_unjail_after_the_jail_period() {
        let (trie, _) = setup();
        seed(&trie, &[(1, 10 * NBL), (2, 10 * NBL), (3, 10 * NBL)]).await;
        let set = registry_set(&load_validators(&trie).await.unwrap());
        let victim = hex::encode(key(3).verifying_key().to_bytes());
        let other = key(1).verifying_key().to_bytes();
        // Every turn scheduled for validator 3 is taken by validator 1.
        let mut slot = 0;
        let mut misses = 0;
        while misses < MAX_MISSED_TURNS || slot < MIN_DOWNTIME_BLOCKS + 10 {
            slot += 1;
            let expected = common::consensus_types::select_proposer(
                set.iter().map(|v| (v.public_key.as_slice(), v.stake)),
                slot + 1,
                0,
            )
            .unwrap();
            let actual = if hex::encode(&expected) == victim {
                misses += 1;
                other.to_vec()
            } else {
                expected
            };
            record_turn(&trie, &set, slot, &actual).await.unwrap();
        }
        let entry = load_validators(&trie)
            .await
            .unwrap()
            .into_iter()
            .find(|v| v.public_key == victim)
            .unwrap();
        assert!(entry.jailed);
        assert!(registry_set(&load_validators(&trie).await.unwrap()).len() == 2);
        // Genesis validators have placeholder addresses: unjail works by key.

        assert!(unjail(&trie, &address(3), &pubkey(3), slot + 1).await.is_err());
        unjail(&trie, &address(3), &pubkey(3), slot + JAIL_BLOCKS).await.unwrap();
        assert_eq!(registry_set(&load_validators(&trie).await.unwrap()).len(), 3);
    }

    #[tokio::test]
    async fn double_sign_evidence_slashes_and_tombstones() {
        let (trie, _) = setup();
        seed(&trie, &[(1, 100 * NBL), (2, 100 * NBL)]).await;
        unbond(&trie, &address(2), &pubkey(2), 20 * NBL, 10).await.unwrap();
        let k = key(2);
        let evidence = DoubleSignEvidence {
            first: vote(&k, 12, 1),
            second: vote(&k, 12, 2),
        };
        let penalty = apply_evidence(&trie, &evidence, 15).await.unwrap();
        // 5% of the 80 still bonded plus 5% of the 20 unbonding.
        assert_eq!(penalty, 5 * NBL);
        let entry = load_validators(&trie)
            .await
            .unwrap()
            .into_iter()
            .find(|v| v.address == hex_addr(&address(2)))
            .unwrap();
        assert!(entry.tombstoned);
        assert_eq!(own_stake(&entry), 76 * NBL);
        assert_eq!(load_unbonding(&trie).await.unwrap()[0].amount, (19 * NBL).to_string());
        // Same evidence again, or unjailing, does nothing.
        assert!(apply_evidence(&trie, &evidence, 16).await.is_err());
        assert!(unjail(&trie, &address(2), &pubkey(2), u64::MAX - 1).await.is_err());
        assert_eq!(registry_set(&load_validators(&trie).await.unwrap()).len(), 1);
    }

    #[tokio::test]
    async fn forged_or_stale_evidence_is_refused() {
        let (trie, _) = setup();
        seed(&trie, &[(1, 100 * NBL), (2, 100 * NBL)]).await;
        let k = key(2);
        // Same block twice is not equivocation.
        let same = DoubleSignEvidence { first: vote(&k, 12, 1), second: vote(&k, 12, 1) };
        assert!(apply_evidence(&trie, &same, 15).await.is_err());
        // Signed by someone else.
        let mut forged = DoubleSignEvidence { first: vote(&k, 12, 1), second: vote(&key(1), 12, 2) };
        forged.second.voter = forged.first.voter.clone();
        assert!(apply_evidence(&trie, &forged, 15).await.is_err());
        // Too old.
        let old = DoubleSignEvidence { first: vote(&k, 12, 1), second: vote(&k, 12, 2) };
        assert!(apply_evidence(&trie, &old, 13 + UNBONDING_BLOCKS).await.is_err());
    }

    #[tokio::test]
    async fn undelegate_returns_through_the_queue() {
        let (trie, cs) = setup();
        seed(&trie, &[(1, 10 * NBL)]).await;
        let delegator = address(9);
        crate::accounts::store(&cs, &delegator, 5 * NBL, 0).unwrap();
        crate::governance_store::apply_delegate(&trie, &cs, delegator, address(1), 5 * NBL)
            .await
            .unwrap();
        assert!(undelegate(&trie, &cs, &delegator, &address(1), 6 * NBL, 2).await.is_err());
        undelegate(&trie, &cs, &delegator, &address(1), 5 * NBL, 2).await.unwrap();
        let v = &load_validators(&trie).await.unwrap()[0];
        assert_eq!(v.total_delegated, "0");
        assert_eq!(v.delegator_count, 0);
        begin_block(&trie, &cs, 2 + UNBONDING_BLOCKS).await.unwrap();
        // apply_delegate does not debit (bond() does); the queue credits 5.
        assert_eq!(crate::accounts::load(&cs, &delegator).0, 10 * NBL);
    }
}
