use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Runtime version information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeVersion {
    pub spec_name: String,
    pub impl_name: String,
    pub authoring_version: u32,
    pub spec_version: u32,
    pub impl_version: u32,
}

/// Runtime upgrade proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeProposal {
    pub id: u64,
    pub new_version: RuntimeVersion,
    pub code_hash: [u8; 32],
    pub activation_height: u64,
    pub proposer: [u8; 20],
    pub approved: bool,
}

/// Runtime upgrade manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeUpgradeManager {
    current_version: RuntimeVersion,
    pending_upgrades: HashMap<u64, UpgradeProposal>,
    upgrade_history: Vec<RuntimeVersion>,
    next_proposal_id: u64,
}

impl RuntimeUpgradeManager {
    pub fn new(initial_version: RuntimeVersion) -> Self {
        Self {
            current_version: initial_version.clone(),
            pending_upgrades: HashMap::new(),
            upgrade_history: vec![initial_version],
            next_proposal_id: 1,
        }
    }

    /// Propose a runtime upgrade
    pub fn propose_upgrade(
        &mut self,
        new_version: RuntimeVersion,
        code_hash: [u8; 32],
        activation_height: u64,
        proposer: [u8; 20],
    ) -> Result<u64, String> {
        // Validate version increment
        if new_version.spec_version <= self.current_version.spec_version {
            return Err("New version must be greater than current version".into());
        }

        let proposal_id = self.next_proposal_id;
        self.next_proposal_id += 1;

        let proposal = UpgradeProposal {
            id: proposal_id,
            new_version,
            code_hash,
            activation_height,
            proposer,
            approved: false,
        };

        self.pending_upgrades.insert(proposal_id, proposal);
        Ok(proposal_id)
    }

    /// Approve an upgrade proposal (via governance)
    pub fn approve_upgrade(&mut self, proposal_id: u64) -> Result<(), String> {
        let proposal = self.pending_upgrades
            .get_mut(&proposal_id)
            .ok_or("Proposal not found")?;
        
        proposal.approved = true;
        Ok(())
    }

    /// Execute an approved upgrade at the specified height
    pub fn execute_upgrade(&mut self, proposal_id: u64, current_height: u64) -> Result<RuntimeVersion, String> {
        let proposal = self.pending_upgrades
            .get(&proposal_id)
            .ok_or("Proposal not found")?;

        if !proposal.approved {
            return Err("Proposal not approved".into());
        }

        if current_height < proposal.activation_height {
            return Err("Activation height not reached".into());
        }

        // Execute upgrade
        self.current_version = proposal.new_version.clone();
        self.upgrade_history.push(proposal.new_version.clone());
        
        // Remove from pending
        self.pending_upgrades.remove(&proposal_id);

        Ok(self.current_version.clone())
    }

    /// Get current runtime version
    pub fn current_version(&self) -> &RuntimeVersion {
        &self.current_version
    }

    /// Get upgrade history
    pub fn history(&self) -> &[RuntimeVersion] {
        &self.upgrade_history
    }

    /// Rollback to previous version (emergency only)
    pub fn rollback(&mut self) -> Result<RuntimeVersion, String> {
        if self.upgrade_history.len() < 2 {
            return Err("No previous version to rollback to".into());
        }

        // Remove current version
        self.upgrade_history.pop();
        
        // Revert to previous
        self.current_version = self.upgrade_history.last().unwrap().clone();

        Ok(self.current_version.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_version(spec_version: u32) -> RuntimeVersion {
        RuntimeVersion {
            spec_name: "test-runtime".to_string(),
            impl_name: "test-node".to_string(),
            authoring_version: 1,
            spec_version,
            impl_version: 1,
        }
    }

    #[test]
    fn test_propose_upgrade() {
        let mut manager = RuntimeUpgradeManager::new(create_test_version(1));
        
        let proposal_id = manager.propose_upgrade(
            create_test_version(2),
            [1u8; 32],
            100,
            [1u8; 20],
        ).unwrap();

        assert_eq!(proposal_id, 1);
        assert_eq!(manager.pending_upgrades.len(), 1);
    }

    #[test]
    fn test_approve_and_execute_upgrade() {
        let mut manager = RuntimeUpgradeManager::new(create_test_version(1));
        
        let proposal_id = manager.propose_upgrade(
            create_test_version(2),
            [1u8; 32],
            100,
            [1u8; 20],
        ).unwrap();

        manager.approve_upgrade(proposal_id).unwrap();
        
        let new_version = manager.execute_upgrade(proposal_id, 100).unwrap();
        assert_eq!(new_version.spec_version, 2);
        assert_eq!(manager.current_version().spec_version, 2);
    }

    #[test]
    fn test_rollback() {
        let mut manager = RuntimeUpgradeManager::new(create_test_version(1));
        
        let proposal_id = manager.propose_upgrade(
            create_test_version(2),
            [1u8; 32],
            100,
            [1u8; 20],
        ).unwrap();

        manager.approve_upgrade(proposal_id).unwrap();
        manager.execute_upgrade(proposal_id, 100).unwrap();

        // Rollback
        let rolled_back = manager.rollback().unwrap();
        assert_eq!(rolled_back.spec_version, 1);
        assert_eq!(manager.current_version().spec_version, 1);
    }

    #[test]
    fn test_version_validation() {
        let mut manager = RuntimeUpgradeManager::new(create_test_version(2));
        
        // Try to propose older version
        let result = manager.propose_upgrade(
            create_test_version(1),
            [1u8; 32],
            100,
            [1u8; 20],
        );

        assert!(result.is_err());
    }
}
