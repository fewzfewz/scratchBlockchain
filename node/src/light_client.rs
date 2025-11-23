use anyhow::Result;
use common::types::{Block, Header, Hash};
use consensus::ValidatorInfo;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Sync committee for light client support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCommittee {
    /// Public keys of sync committee members
    pub members: Vec<Vec<u8>>,
    /// Aggregate public key for efficient verification
    pub aggregate_pubkey: Vec<u8>,
}

impl SyncCommittee {
    pub fn new(members: Vec<Vec<u8>>) -> Self {
        // In a real implementation, compute aggregate BLS key
        // For MVP, just use first member's key
        let aggregate_pubkey = members.first().cloned().unwrap_or_default();
        
        Self {
            members,
            aggregate_pubkey,
        }
    }

    pub fn size(&self) -> usize {
        self.members.len()
    }
}

/// Light client update containing header and sync committee signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightClientUpdate {
    /// Attested header
    pub attested_header: Header,
    /// Next sync committee (if rotation occurred)
    pub next_sync_committee: Option<SyncCommittee>,
    /// Sync committee bits (who participated)
    pub sync_committee_bits: Vec<bool>,
    /// Aggregate signature
    pub sync_committee_signature: Vec<u8>,
    /// Finality proof
    pub finality_branch: Vec<Hash>,
}

/// Light client state
#[derive(Debug, Clone)]
pub struct LightClientState {
    /// Current header
    pub current_header: Header,
    /// Current sync committee
    pub current_sync_committee: SyncCommittee,
    /// Next sync committee (during rotation period)
    pub next_sync_committee: Option<SyncCommittee>,
    /// Finalized header
    pub finalized_header: Option<Header>,
}

impl LightClientState {
    pub fn new(genesis_header: Header, sync_committee: SyncCommittee) -> Self {
        Self {
            current_header: genesis_header.clone(),
            current_sync_committee: sync_committee,
            next_sync_committee: None,
            finalized_header: Some(genesis_header),
        }
    }

    /// Apply a light client update
    pub fn apply_update(&mut self, update: LightClientUpdate) -> Result<()> {
        // Verify sync committee participation (need 2/3+)
        let participation_count = update.sync_committee_bits.iter().filter(|&&b| b).count();
        let threshold = (self.current_sync_committee.size() * 2) / 3;
        
        if participation_count < threshold {
            return Err(anyhow::anyhow!(
                "Insufficient sync committee participation: {} < {}",
                participation_count,
                threshold
            ));
        }

        // Verify signature (simplified - in production use BLS aggregate verification)
        if update.sync_committee_signature.len() != 64 {
            return Err(anyhow::anyhow!("Invalid signature length"));
        }

        // Update current header
        self.current_header = update.attested_header;

        // Handle sync committee rotation
        if let Some(next_committee) = update.next_sync_committee {
            self.next_sync_committee = Some(next_committee);
        }

        Ok(())
    }

    /// Finalize a header using finality proof
    pub fn finalize_header(&mut self, header: Header, finality_proof: Vec<Hash>) -> Result<()> {
        // Verify finality proof (Merkle proof)
        // In production, verify the Merkle branch
        if finality_proof.is_empty() {
            return Err(anyhow::anyhow!("Empty finality proof"));
        }

        self.finalized_header = Some(header);
        Ok(())
    }

    /// Rotate to next sync committee
    pub fn rotate_sync_committee(&mut self) -> Result<()> {
        if let Some(next) = self.next_sync_committee.take() {
            self.current_sync_committee = next;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No next sync committee available"))
        }
    }
}

/// Light client for header-only sync
pub struct LightClient {
    state: Arc<Mutex<LightClientState>>,
}

impl LightClient {
    pub fn new(genesis_header: Header, sync_committee: SyncCommittee) -> Self {
        let state = LightClientState::new(genesis_header, sync_committee);
        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }

    /// Process a light client update
    pub async fn process_update(&self, update: LightClientUpdate) -> Result<()> {
        let mut state = self.state.lock().await;
        state.apply_update(update)
    }

    /// Get current header
    pub async fn current_header(&self) -> Header {
        let state = self.state.lock().await;
        state.current_header.clone()
    }

    /// Get finalized header
    pub async fn finalized_header(&self) -> Option<Header> {
        let state = self.state.lock().await;
        state.finalized_header.clone()
    }

    /// Verify state proof (for querying state)
    pub fn verify_state_proof(
        &self,
        _state_root: Hash,
        key: &[u8],
        value: &[u8],
        proof: &[Hash],
    ) -> bool {
        // Simplified Merkle proof verification
        // In production, implement full Merkle Patricia trie verification
        !proof.is_empty() && key.len() > 0 && value.len() > 0
    }
}

/// Sync committee manager for full nodes
pub struct SyncCommitteeManager {
    /// Current sync committee
    current_committee: SyncCommittee,
    /// Validators
    validators: Vec<ValidatorInfo>,
    /// Committee size
    committee_size: usize,
    /// Rotation period (in slots)
    rotation_period: u64,
}

impl SyncCommitteeManager {
    pub fn new(validators: Vec<ValidatorInfo>, committee_size: usize, rotation_period: u64) -> Self {
        // Select initial sync committee from validators
        let members: Vec<Vec<u8>> = validators
            .iter()
            .take(committee_size)
            .map(|v| v.public_key.clone())
            .collect();

        let current_committee = SyncCommittee::new(members);

        Self {
            current_committee,
            validators,
            committee_size,
            rotation_period,
        }
    }

    /// Generate light client update for a block
    pub fn generate_update(
        &self,
        header: Header,
        should_rotate: bool,
    ) -> LightClientUpdate {
        let next_sync_committee = if should_rotate {
            // Select next committee (simplified - random selection)
            let members: Vec<Vec<u8>> = self.validators
                .iter()
                .take(self.committee_size)
                .map(|v| v.public_key.clone())
                .collect();
            Some(SyncCommittee::new(members))
        } else {
            None
        };

        // Generate sync committee bits (all participated for MVP)
        let sync_committee_bits = vec![true; self.current_committee.size()];

        // Generate aggregate signature (mock for MVP)
        let sync_committee_signature = vec![0; 64];

        LightClientUpdate {
            attested_header: header,
            next_sync_committee,
            sync_committee_bits,
            sync_committee_signature,
            finality_branch: vec![[0; 32]; 4], // Mock finality proof
        }
    }

    /// Check if rotation should occur at this slot
    pub fn should_rotate(&self, slot: u64) -> bool {
        slot % self.rotation_period == 0
    }

    pub fn current_committee(&self) -> &SyncCommittee {
        &self.current_committee
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::types::Header;

    #[test]
    fn test_sync_committee_creation() {
        let members = vec![vec![1; 32], vec![2; 32], vec![3; 32]];
        let committee = SyncCommittee::new(members.clone());
        
        assert_eq!(committee.size(), 3);
        assert_eq!(committee.members, members);
    }

    #[test]
    fn test_light_client_state() {
        let genesis = Header::new([0; 32], 0);
        let committee = SyncCommittee::new(vec![vec![1; 32]]);
        
        let state = LightClientState::new(genesis.clone(), committee);
        assert_eq!(state.current_header.slot, 0);
        assert!(state.finalized_header.is_some());
    }

    #[tokio::test]
    async fn test_light_client_update() {
        let genesis = Header::new([0; 32], 0);
        let committee = SyncCommittee::new(vec![vec![1; 32]]);
        let client = LightClient::new(genesis, committee);

        let update = LightClientUpdate {
            attested_header: Header::new([1; 32], 1),
            next_sync_committee: None,
            sync_committee_bits: vec![true],
            sync_committee_signature: vec![0; 64],
            finality_branch: vec![[0; 32]],
        };

        assert!(client.process_update(update).await.is_ok());
        
        let current = client.current_header().await;
        assert_eq!(current.slot, 1);
    }

    #[test]
    fn test_sync_committee_manager() {
        let validators = vec![
            ValidatorInfo {
                public_key: vec![1; 32],
                stake: 100,
                slashed: false,
            },
            ValidatorInfo {
                public_key: vec![2; 32],
                stake: 100,
                slashed: false,
            },
        ];

        let manager = SyncCommitteeManager::new(validators, 2, 256);
        assert_eq!(manager.current_committee().size(), 2);
        assert!(manager.should_rotate(256));
        assert!(!manager.should_rotate(255));
    }
}
