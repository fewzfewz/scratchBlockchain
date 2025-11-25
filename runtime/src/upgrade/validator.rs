use crate::upgrade::version::RuntimeVersion;

/// Upgrade validator for safety checks
pub struct UpgradeValidator;

impl UpgradeValidator {
    pub fn new() -> Self {
        Self
    }

    /// Validate upgrade before execution
    pub fn validate_upgrade(
        &self,
        current: &RuntimeVersion,
        new: &RuntimeVersion,
        code: &[u8],
    ) -> Result<(), ValidationError> {
        // 1. Version compatibility
        self.check_version_compatibility(current, new)?;
        
        // 2. Code hash verification
        self.verify_code_hash(code)?;
        
        // 3. Resource requirements
        self.check_resource_requirements(code)?;
        
        Ok(())
    }

    fn check_version_compatibility(
        &self,
        current: &RuntimeVersion,
        new: &RuntimeVersion,
    ) -> Result<(), ValidationError> {
        if !current.can_upgrade_to(new) {
            return Err(ValidationError::IncompatibleVersion(
                format!("Cannot upgrade from {} to {}", current, new)
            ));
        }
        
        Ok(())
    }

    fn verify_code_hash(&self, code: &[u8]) -> Result<(), ValidationError> {
        if code.is_empty() {
            return Err(ValidationError::InvalidCode("Code is empty".to_string()));
        }
        
        // In production: verify code hash against approved hash
        Ok(())
    }

    fn check_resource_requirements(&self, code: &[u8]) -> Result<(), ValidationError> {
        // Check code size
        const MAX_CODE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
        if code.len() > MAX_CODE_SIZE {
            return Err(ValidationError::CodeTooLarge);
        }
        
        Ok(())
    }

    /// Validate state after upgrade
    pub fn validate_post_upgrade(
        &self,
        old_state_root: &[u8; 32],
        new_state_root: &[u8; 32],
        has_migrations: bool,
    ) -> Result<(), ValidationError> {
        // State root should change only if migrations were executed
        if has_migrations && old_state_root == new_state_root {
            return Err(ValidationError::StateNotMigrated);
        }
        
        // In production: verify invariants (total supply, etc.)
        Ok(())
    }
}

impl Default for UpgradeValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Incompatible version: {0}")]
    IncompatibleVersion(String),
    
    #[error("Invalid code: {0}")]
    InvalidCode(String),
    
    #[error("Code too large")]
    CodeTooLarge,
    
    #[error("State not migrated")]
    StateNotMigrated,
    
    #[error("Invariant violated: {0}")]
    InvariantViolated(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_validation() {
        let validator = UpgradeValidator::new();
        let v1 = RuntimeVersion::new(1, 0, 0);
        let v2 = RuntimeVersion::new(2, 0, 0);
        let v3 = RuntimeVersion::new(3, 0, 0);

        // Valid upgrade
        assert!(validator.check_version_compatibility(&v1, &v2).is_ok());
        
        // Invalid upgrade (skipping major version)
        assert!(validator.check_version_compatibility(&v1, &v3).is_err());
    }

    #[test]
    fn test_code_validation() {
        let validator = UpgradeValidator::new();
        
        // Empty code
        assert!(validator.verify_code_hash(&[]).is_err());
        
        // Valid code
        assert!(validator.verify_code_hash(b"valid code").is_ok());
    }
}
