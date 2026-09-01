//! Persist and apply runtime upgrades from the state trie.

use crate::runtime_upgrade::{RuntimeUpgradeManager, RuntimeVersion, UpgradeProposal};
use std::sync::Arc;
use storage::trie::PatriciaTrie;
use tokio::sync::Mutex;
use tracing::{info, warn};

pub const RUNTIME_KEY: &[u8] = b"runtime";

fn default_version() -> RuntimeVersion {
    RuntimeVersion {
        spec_name: "nebula".to_string(),
        impl_name: "nebula-node".to_string(),
        authoring_version: 1,
        spec_version: 1,
        impl_version: 1,
    }
}

pub async fn load_or_init(
    trie: &Arc<Mutex<PatriciaTrie>>,
) -> Result<RuntimeUpgradeManager, Box<dyn std::error::Error>> {
    let guard = trie.lock().await;
    if let Some(data) = guard.get(RUNTIME_KEY)? {
        if let Ok(state) = serde_json::from_slice(&data) {
            return Ok(state);
        }
        warn!("Stored runtime state failed to parse — reseeding");
    }
    drop(guard);

    let state = RuntimeUpgradeManager::new(default_version());
    persist(trie, &state).await?;
    info!("Seeded runtime upgrade manager");
    Ok(state)
}

pub async fn persist(
    trie: &Arc<Mutex<PatriciaTrie>>,
    state: &RuntimeUpgradeManager,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = serde_json::to_vec(state)?;
    trie.lock().await.insert(RUNTIME_KEY, &data)?;
    Ok(())
}

/// Activate approved upgrades whose activation height has been reached.
pub async fn apply_at_height(
    trie: &Arc<Mutex<PatriciaTrie>>,
    height: u64,
) -> Result<Option<RuntimeVersion>, Box<dyn std::error::Error>> {
    let mut state = load_or_init(trie).await?;
    let pending: Vec<u64> = state
        .pending_upgrades()
        .iter()
        .filter(|p| p.approved && height >= p.activation_height)
        .map(|p| p.id)
        .collect();

    if pending.is_empty() {
        return Ok(None);
    }

    let mut activated = None;
    for id in pending {
        match state.execute_upgrade(id, height) {
            Ok(version) => {
                info!(
                    "Runtime upgrade {} activated at height {} (spec v{})",
                    id, height, version.spec_version
                );
                activated = Some(version);
            }
            Err(e) => warn!("Runtime upgrade {} not applied at height {}: {}", id, height, e),
        }
    }

    persist(trie, &state).await?;
    Ok(activated)
}

pub async fn propose_upgrade(
    trie: &Arc<Mutex<PatriciaTrie>>,
    new_version: RuntimeVersion,
    code_hash: [u8; 32],
    activation_height: u64,
    proposer: [u8; 20],
) -> Result<u64, String> {
    let mut state = load_or_init(trie).await.map_err(|e| e.to_string())?;
    let id = state.propose_upgrade(new_version, code_hash, activation_height, proposer)?;
    persist(trie, &state).await.map_err(|e| e.to_string())?;
    Ok(id)
}

pub async fn approve_upgrade(
    trie: &Arc<Mutex<PatriciaTrie>>,
    proposal_id: u64,
) -> Result<(), String> {
    let mut state = load_or_init(trie).await.map_err(|e| e.to_string())?;
    state.approve_upgrade(proposal_id)?;
    persist(trie, &state).await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn list_pending(
    trie: &Arc<Mutex<PatriciaTrie>>,
) -> Result<Vec<UpgradeProposal>, Box<dyn std::error::Error>> {
    let state = load_or_init(trie).await?;
    Ok(state.pending_upgrades().to_vec())
}

pub async fn current_version(
    trie: &Arc<Mutex<PatriciaTrie>>,
) -> Result<RuntimeVersion, Box<dyn std::error::Error>> {
    let state = load_or_init(trie).await?;
    Ok(state.current_version().clone())
}
