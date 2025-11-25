use serde::{Deserialize, Serialize};
use std::fmt;

/// Runtime version information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeVersion {
    /// Major version - breaking changes
    pub major: u32,
    /// Minor version - new features, backwards compatible
    pub minor: u32,
    /// Patch version - bug fixes
    pub patch: u32,
    /// Specification version - increments with any change
    pub spec_version: u32,
    /// Implementation version - specific to this implementation
    pub impl_version: u32,
}

impl RuntimeVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            spec_version: major * 1000 + minor * 100 + patch,
            impl_version: 1,
        }
    }

    /// Check if this version is compatible with another version
    pub fn is_compatible(&self, other: &RuntimeVersion) -> bool {
        // Major version must match
        if self.major != other.major {
            return false;
        }
        
        // Minor version can be higher (backwards compatible)
        self.minor >= other.minor
    }

    /// Check if this version can upgrade to another version
    pub fn can_upgrade_to(&self, new: &RuntimeVersion) -> bool {
        // Spec version must increase
        if new.spec_version <= self.spec_version {
            return false;
        }

        // Major version can only increment by 1 (no skipping)
        if new.major > self.major + 1 {
            return false;
        }

        // If major version increases, minor and patch should reset
        if new.major > self.major {
            return new.minor == 0 && new.patch == 0;
        }

        true
    }
}

impl fmt::Display for RuntimeVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}.{}.{} (spec: {}, impl: {})", 
            self.major, self.minor, self.patch, 
            self.spec_version, self.impl_version)
    }
}

/// Metadata about a runtime version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeMetadata {
    pub version: RuntimeVersion,
    pub code_hash: [u8; 32],
    pub activated_at: u64,
    pub state_version: u32,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() {
        let v1_0_0 = RuntimeVersion::new(1, 0, 0);
        let v1_1_0 = RuntimeVersion::new(1, 1, 0);
        let v2_0_0 = RuntimeVersion::new(2, 0, 0);

        // Same major, higher minor is compatible
        assert!(v1_1_0.is_compatible(&v1_0_0));
        
        // Different major is not compatible
        assert!(!v2_0_0.is_compatible(&v1_0_0));
    }

    #[test]
    fn test_can_upgrade() {
        let v1_0_0 = RuntimeVersion::new(1, 0, 0);
        let v1_1_0 = RuntimeVersion::new(1, 1, 0);
        let v2_0_0 = RuntimeVersion::new(2, 0, 0);
        let v3_0_0 = RuntimeVersion::new(3, 0, 0);

        // Can upgrade to next minor version
        assert!(v1_0_0.can_upgrade_to(&v1_1_0));
        
        // Can upgrade to next major version
        assert!(v1_0_0.can_upgrade_to(&v2_0_0));
        
        // Cannot skip major versions
        assert!(!v1_0_0.can_upgrade_to(&v3_0_0));
    }
}
