//! Validators, delegations, governance and runtime-upgrade reads.

use super::*;

pub(super) async fn handle_runtime_version(
    state_trie: Arc<Mutex<PatriciaTrie>>,
) -> Result<impl warp::Reply, Infallible> {
    match crate::runtime_upgrade_store::current_version(&state_trie).await {
        Ok(version) => Ok(warp::reply::json(
            &serde_json::json!({ "version": version }),
        )),
        Err(e) => Ok(warp::reply::json(
            &serde_json::json!({ "error": e.to_string() }),
        )),
    }
}

pub(super) async fn handle_runtime_upgrades(
    state_trie: Arc<Mutex<PatriciaTrie>>,
) -> Result<impl warp::Reply, Infallible> {
    match crate::runtime_upgrade_store::list_pending(&state_trie).await {
        Ok(upgrades) => Ok(warp::reply::json(
            &serde_json::json!({ "upgrades": upgrades }),
        )),
        Err(e) => Ok(warp::reply::json(
            &serde_json::json!({ "error": e.to_string() }),
        )),
    }
}

pub(super) async fn handle_validators(
    state_trie: Arc<Mutex<PatriciaTrie>>,
) -> Result<impl warp::Reply, Infallible> {
    // Validators live in the state trie under the b"validators" key (written at genesis)
    // as a JSON array of ValidatorInfo objects.
    let trie = state_trie.lock().await;
    match trie.get(b"validators") {
        Ok(Some(encoded)) => {
            let vals = serde_json::from_slice::<Vec<ValidatorInfo>>(&encoded).unwrap_or_default();
            let count = vals.len();
            Ok(warp::reply::json(&ValidatorsResponse {
                validators: vals,
                count,
            }))
        }
        _ => {
            // No validators in trie yet
            Ok(warp::reply::json(&ValidatorsResponse {
                validators: vec![],
                count: 0,
            }))
        }
    }
}

pub(super) async fn handle_delegations(
    address_str: String,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let delegations_key = [b"delegations/", address_str.as_bytes()].concat();
    match chain_store.get_state(&delegations_key) {
        Ok(Some(encoded)) => {
            if let Ok(dels) = serde_json::from_slice::<Vec<DelegationInfo>>(&encoded) {
                Ok(warp::reply::json(&DelegationsResponse {
                    delegations: dels,
                    address: address_str,
                }))
            } else {
                Ok(warp::reply::json(&DelegationsResponse {
                    delegations: vec![],
                    address: address_str,
                }))
            }
        }
        _ => Ok(warp::reply::json(&DelegationsResponse {
            delegations: vec![],
            address: address_str,
        })),
    }
}

pub(super) async fn handle_governance(
    state_trie: Arc<Mutex<PatriciaTrie>>,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let (state, height) = load_governance_state(&state_trie, &chain_store).await;
    let Some(state) = state else {
        return Ok(warp::reply::json(&governance_empty()));
    };

    let proposals: Vec<serde_json::Value> = state
        .proposals
        .iter()
        .map(|p| proposal_json(p, &state, height))
        .collect();

    let params = &state.params;
    Ok(warp::reply::json(&serde_json::json!({
        "params": {
            "proposal_deposit": params.proposal_deposit.to_string(),
            "voting_period_blocks": params.voting_period_blocks,
            "quorum_threshold_bps": params.quorum_threshold_bps,
            "pass_threshold_bps": params.pass_threshold_bps,
        },
        "treasury": {
            "balance": state.treasury.balance.to_string(),
            "total_collected": state.treasury.total_collected.to_string(),
            "total_spent": state.treasury.total_spent.to_string(),
        },
        "total_stake": state.total_stake.to_string(),
        "next_proposal_id": state.next_proposal_id,
        "height": height,
        "proposals": proposals,
    })))
}

pub(super) async fn handle_proposal(
    id: u64,
    state_trie: Arc<Mutex<PatriciaTrie>>,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let (state, height) = load_governance_state(&state_trie, &chain_store).await;
    let Some(state) = state else {
        return Ok(warp::reply::json(&serde_json::json!({
            "proposal": None::<serde_json::Value>,
            "error": "No governance state",
        })));
    };

    match state.get_proposal(id) {
        Some(proposal) => Ok(warp::reply::json(&serde_json::json!({
            "proposal": proposal_json(proposal, &state, height),
            "error": None::<String>,
        }))),
        None => Ok(warp::reply::json(&serde_json::json!({
            "proposal": None::<serde_json::Value>,
            "error": format!("Proposal {} not found", id),
        }))),
    }
}

pub(super) async fn load_governance_state(
    state_trie: &Mutex<PatriciaTrie>,
    chain_store: &ChainStore,
) -> (Option<ChainGovernance>, u64) {
    let height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    let guard = state_trie.lock().await;
    let state = guard
        .get(b"governance")
        .ok()
        .flatten()
        .and_then(|data| serde_json::from_slice(&data).ok());
    (state, height)
}

pub(super) fn governance_empty() -> serde_json::Value {
    serde_json::json!({
        "params": { "proposal_deposit": "0", "voting_period_blocks": 0, "quorum_threshold_bps": 0, "pass_threshold_bps": 0 },
        "treasury": { "balance": "0", "total_collected": "0", "total_spent": "0" },
        "total_stake": "0",
        "next_proposal_id": 1,
        "height": 0,
        "proposals": [],
    })
}

pub(super) fn proposal_json(
    p: &Proposal,
    state: &ChainGovernance,
    height: u64,
) -> serde_json::Value {
    let status = state.resolve_status(p, height);
    let total_votes = p.yes_votes + p.no_votes + p.abstain_votes;
    let quorum = (state.total_stake * state.params.quorum_threshold_bps as u128) / 10_000;
    let voters: HashMap<String, String> = p
        .voters
        .iter()
        .map(|(a, c)| (format!("0x{}", a), c.as_str().to_string()))
        .collect();

    serde_json::json!({
        "id": p.id,
        "title": p.title,
        "description": p.description,
        "proposer": format!("0x{}", hex::encode(p.proposer)),
        "status": status_str(&status),
        "start_block": p.start_block,
        "end_block": p.end_block,
        "voting_period_blocks": state.params.voting_period_blocks,
        "yes_votes": p.yes_votes.to_string(),
        "no_votes": p.no_votes.to_string(),
        "abstain_votes": p.abstain_votes.to_string(),
        "total_votes": total_votes.to_string(),
        "quorum": quorum.to_string(),
        "voters": voters,
    })
}

pub(super) fn status_str(s: &ProposalStatus) -> &'static str {
    match s {
        ProposalStatus::Active => "active",
        ProposalStatus::Passed => "passed",
        ProposalStatus::Rejected => "rejected",
        ProposalStatus::Executed => "executed",
    }
}
