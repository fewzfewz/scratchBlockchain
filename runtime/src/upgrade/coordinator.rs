use serde::{Deserialize, Serialize};
use crate::upgrade::{
    version::{RuntimeVersion, RuntimeMetadata},
    snapshot::{SnapshotManager, SnapshotError},
    migration::{StateMigrator, MigrationPlan, MigrationError},
    validator::{UpgradeValidator, ValidationError},
};
use std::collections::HashMap;

/// Upgrade state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UpgradeState {
    Proposed,
    Scheduled,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Pending upgrade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingUpgrade {
    pub id: u64,
    pub from_version: RuntimeVersion,
    pub to_version: RuntimeVersion,
    pub code_hash: [u8; 32],
    pub activation_block: u64,
    pub state: UpgradeState,
    pub migration_plan: Option<MigrationPlan>,
}

/// Upgrade coordinator
pub struct UpgradeCoordinator {
    current_version: RuntimeVersion,
    version_history: HashMap<u32, RuntimeMetadata>,
    pending_upgrade: Option<PendingUpgrade>,
    snapshot_manager: SnapshotManager,
    migrator: StateMigrator,
    validator: UpgradeValidator,
}

impl UpgradeCoordinator {
    pub fn new(initial_version: RuntimeVersion) -> Self {
        Self {
            current_version: initial_version,
            version_history: HashMap::new(),
            pending_upgrade: None,
            snapshot_manager: SnapshotManager::new(10),
            migrator: StateMigrator::new(),
            validator: UpgradeValidator::new(),
        }
    }

    /// Get current runtime version
    pub fn current_version(&self) -> &RuntimeVersion {
        &self.current_version
    }

    /// Schedule an upgrade
    pub fn schedule_upgrade(
        &mut self,
        new_version: RuntimeVersion,
        code: Vec<u8>,
        activation_block: u64,
        migration_plan: Option<MigrationPlan>,
    ) -> Result<u64, UpgradeError> {
        // Check if there's already a pending upgrade
        if self.pending_upgrade.is_some() {
            return Err(UpgradeError::UpgradeAlreadyPending);
        }

        // Validate upgrade
        self.validator.validate_upgrade(&self.current_version, &new_version, &code)?;

        // Compute code hash
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&code);
        let code_hash: [u8; 32] = hasher.finalize().into();

        let upgrade_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let pending = PendingUpgrade {
            id: upgrade_id,
            from_version: self.current_version.clone(),
            to_version: new_version,
            code_hash,
            activation_block,
            state: UpgradeState::Scheduled,
            migration_plan,
        };

        self.pending_upgrade = Some(pending);

        Ok(upgrade_id)
    }

    /// Execute the pending upgrade
    pub fn execute_upgrade(
        &mut self,
        current_block: u64,
        state_data: &[u8],
    ) -> Result<Vec<u8>, UpgradeError> {
        let upgrade = self.pending_upgrade.as_mut()
            .ok_or(UpgradeError::NoUpgradePending)?;

        // Check if it's time to execute
        if current_block < upgrade.activation_block {
            return Err(UpgradeError::UpgradeNotReady);
        }

        // Update state
        upgrade.state = UpgradeState::InProgress;

        // Create snapshot before upgrade
        let snapshot_id = self.snapshot_manager.create_snapshot(
            self.current_version.clone(),
            current_block,
            state_data,
        )?;

        println!("Created snapshot {} before upgrade", snapshot_id);

        // Execute state migration
        let has_migrations = self.migrator.migration_count() > 0;
        let new_state = if has_migrations {
            self.migrator.execute_migrations(state_data)?
        } else {
            state_data.to_vec()
        };

        // Validate post-upgrade state
        use sha2::{Sha256, Digest};
        let mut old_hasher = Sha256::new();
        old_hasher.update(state_data);
        let old_state_root: [u8; 32] = old_hasher.finalize().into();

        let mut new_hasher = Sha256::new();
        new_hasher.update(&new_state);
        let new_state_root: [u8; 32] = new_hasher.finalize().into();

        self.validator.validate_post_upgrade(&old_state_root, &new_state_root, has_migrations)?;

        // Update current version
        self.current_version = upgrade.to_version.clone();
        upgrade.state = UpgradeState::Completed;

        // Record in history
        let metadata = RuntimeMetadata {
            version: self.current_version.clone(),
            code_hash: upgrade.code_hash,
            activated_at: current_block,
            state_version: self.current_version.spec_version,
            description: format!("Upgrade from {} to {}", 
                upgrade.from_version, upgrade.to_version),
        };
        self.version_history.insert(self.current_version.spec_version, metadata);

        // Clear pending upgrade
        self.pending_upgrade = None;

        Ok(new_state)
    }

    /// Rollback to previous version
    pub fn rollback_upgrade(&mut self) -> Result<Vec<u8>, UpgradeError> {
        // Get latest snapshot
        let snapshot = self.snapshot_manager.get_latest()
            .ok_or(UpgradeError::NoSnapshotAvailable)?;

        // Restore state
        let restored_state = self.snapshot_manager.restore_snapshot(snapshot.id)?;

        // Revert version
        self.current_version = snapshot.version.clone();

        // Clear pending upgrade
        if let Some(upgrade) = &mut self.pending_upgrade {
            upgrade.state = UpgradeState::Failed;
        }
        self.pending_upgrade = None;

        println!("Rolled back to version {}", self.current_version);

        Ok(restored_state)
    }

    /// Cancel pending upgrade
    pub fn cancel_upgrade(&mut self) -> Result<(), UpgradeError> {
        let upgrade = self.pending_upgrade.as_mut()
            .ok_or(UpgradeError::NoUpgradePending)?;

        if upgrade.state == UpgradeState::InProgress {
            return Err(UpgradeError::UpgradeInProgress);
        }

        upgrade.state = UpgradeState::Cancelled;
        self.pending_upgrade = None;

        Ok(())
    }

    /// Get pending upgrade
    pub fn get_pending_upgrade(&self) -> Option<&PendingUpgrade> {
        self.pending_upgrade.as_ref()
    }

    /// Get version history
    pub fn get_version_history(&self) -> &HashMap<u32, RuntimeMetadata> {
        &self.version_history
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UpgradeError {
    #[error("Upgrade already pending")]
    UpgradeAlreadyPending,
    
    #[error("No upgrade pending")]
    NoUpgradePending,
    
    #[error("Upgrade not ready for execution")]
    UpgradeNotReady,
    
    #[error("Upgrade in progress")]
    UpgradeInProgress,
    
    #[error("No snapshot available for rollback")]
    NoSnapshotAvailable,
    
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
    
    #[error("Migration error: {0}")]
    Migration(#[from] MigrationError),
    
    #[error("Snapshot error: {0}")]
    Snapshot(#[from] SnapshotError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_upgrade() {
        let mut coordinator = UpgradeCoordinator::new(RuntimeVersion::new(1, 0, 0));
        
        let result = coordinator.schedule_upgrade(
            RuntimeVersion::new(1, 1, 0),
            b"new code".to_vec(),
            1000,
            None,
        );
        
        assert!(result.is_ok());
        assert!(coordinator.get_pending_upgrade().is_some());
    }

    #[test]
    fn test_execute_upgrade() {
        let mut coordinator = UpgradeCoordinator::new(RuntimeVersion::new(1, 0, 0));
        
        coordinator.schedule_upgrade(
            RuntimeVersion::new(1, 1, 0),
            b"new code".to_vec(),
            100,
            None,
        ).unwrap();

        let state = b"test state";
        let result = coordinator.execute_upgrade(100, state);
        
        assert!(result.is_ok());
        assert_eq!(coordinator.current_version().minor, 1);
    }

    #[test]
    fn test_rollback() {
        let mut coordinator = UpgradeCoordinator::new(RuntimeVersion::new(1, 0, 0));
        
        // Execute upgrade
        coordinator.schedule_upgrade(
            RuntimeVersion::new(1, 1, 0),
            b"new code".to_vec(),
            100,
            None,
        ).unwrap();

        let state = b"test state";
        coordinator.execute_upgrade(100, state).unwrap();

        // Rollback
        let result = coordinator.rollback_upgrade();
        assert!(result.is_ok());
        assert_eq!(coordinator.current_version().minor, 0);
    }
}
