use serde::{Deserialize, Serialize};
use crate::upgrade::version::RuntimeVersion;

/// State migration trait
pub trait StateMigration: Send + Sync {
    /// Get migration name
    fn name(&self) -> &str;
    
    /// Migrate state from old version to new version
    fn migrate(&self, old_state: &[u8]) -> Result<Vec<u8>, MigrationError>;
    
    /// Validate migrated state
    fn validate(&self, old_state: &[u8], new_state: &[u8]) -> Result<(), MigrationError>;
}

/// Migration plan for an upgrade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub from_version: RuntimeVersion,
    pub to_version: RuntimeVersion,
    pub migration_names: Vec<String>,
    pub estimated_duration_secs: u64,
    pub requires_pause: bool,
}

/// State migrator
pub struct StateMigrator {
    migrations: Vec<Box<dyn StateMigration>>,
}

impl StateMigrator {
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    /// Register a migration
    pub fn register_migration(&mut self, migration: Box<dyn StateMigration>) {
        self.migrations.push(migration);
    }

    /// Execute all migrations
    pub fn execute_migrations(&self, state: &[u8]) -> Result<Vec<u8>, MigrationError> {
        let mut current_state = state.to_vec();

        for migration in &self.migrations {
            println!("Executing migration: {}", migration.name());
            
            let new_state = migration.migrate(&current_state)?;
            migration.validate(&current_state, &new_state)?;
            
            current_state = new_state;
        }

        Ok(current_state)
    }

    /// Get migration count
    pub fn migration_count(&self) -> usize {
        self.migrations.len()
    }
}

impl Default for StateMigrator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("Migration failed: {0}")]
    MigrationFailed(String),
    
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Incompatible state format")]
    IncompatibleState,
    
    #[error("Data corruption detected")]
    DataCorruption,
}

/// Example migration: Add version field to state
pub struct AddVersionFieldMigration;

impl StateMigration for AddVersionFieldMigration {
    fn name(&self) -> &str {
        "add_version_field"
    }

    fn migrate(&self, old_state: &[u8]) -> Result<Vec<u8>, MigrationError> {
        // Simplified: In production, parse state, add field, re-serialize
        let mut new_state = old_state.to_vec();
        new_state.extend_from_slice(b"_v2");
        Ok(new_state)
    }

    fn validate(&self, _old_state: &[u8], new_state: &[u8]) -> Result<(), MigrationError> {
        if !new_state.ends_with(b"_v2") {
            return Err(MigrationError::ValidationFailed(
                "Version field not added".to_string()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_execution() {
        let mut migrator = StateMigrator::new();
        migrator.register_migration(Box::new(AddVersionFieldMigration));

        let old_state = b"original_state";
        let new_state = migrator.execute_migrations(old_state).unwrap();

        assert!(new_state.ends_with(b"_v2"));
    }
}
