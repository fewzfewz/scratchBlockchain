# Runtime Upgrades

Safe, governance-controlled runtime upgrade system for the modular blockchain.

## Overview

The runtime upgrade system allows the blockchain to evolve without requiring hard forks or network restarts. It provides:

- **Version Management** - Track and validate runtime versions
- **Hot-Swap** - Upgrade runtime without stopping the chain
- **State Migration** - Transform state between versions
- **Governance Control** - Community-approved upgrades
- **Rollback** - Revert to previous version if needed
- **Safety Checks** - Comprehensive validation

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                 Governance Layer                     │
│  (Proposal → Vote → Approval → Schedule Upgrade)    │
└──────────────────┬──────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────┐
│              Upgrade Coordinator                     │
│  - Version validation                                │
│  - State migration orchestration                     │
│  - Rollback management                               │
└──────────────────┬──────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────┐
│              Runtime Manager                         │
│  - Load new runtime                                  │
│  - Execute state migration                           │
│  - Activate new version                              │
└─────────────────────────────────────────────────────┘
```

## Components

### 1. Version Management

```rust
use runtime::{RuntimeVersion, UpgradeCoordinator};

// Create version
let version = RuntimeVersion::new(1, 0, 0);

// Check compatibility
let v1_0 = RuntimeVersion::new(1, 0, 0);
let v1_1 = RuntimeVersion::new(1, 1, 0);
assert!(v1_1.is_compatible(&v1_0));

// Check upgrade path
assert!(v1_0.can_upgrade_to(&v1_1));
```

### 2. Upgrade Coordinator

```rust
use runtime::UpgradeCoordinator;

// Initialize coordinator
let mut coordinator = UpgradeCoordinator::new(
    RuntimeVersion::new(1, 0, 0)
);

// Schedule upgrade
let upgrade_id = coordinator.schedule_upgrade(
    RuntimeVersion::new(1, 1, 0),
    new_runtime_code,
    activation_block,
    migration_plan,
)?;

// Execute at activation block
let new_state = coordinator.execute_upgrade(
    current_block,
    current_state,
)?;
```

### 3. State Migration

```rust
use runtime::{StateMigration, MigrationError};

// Implement custom migration
struct AddFieldMigration;

impl StateMigration for AddFieldMigration {
    fn name(&self) -> &str {
        "add_new_field"
    }

    fn migrate(&self, old_state: &[u8]) -> Result<Vec<u8>, MigrationError> {
        // Transform state
        let mut new_state = parse_state(old_state)?;
        new_state.new_field = default_value();
        Ok(serialize_state(new_state))
    }

    fn validate(&self, old: &[u8], new: &[u8]) -> Result<(), MigrationError> {
        // Verify migration correctness
        Ok(())
    }
}

// Register migration
let mut migrator = StateMigrator::new();
migrator.register_migration(Box::new(AddFieldMigration));
```

### 4. Snapshot & Rollback

```rust
// Snapshots are created automatically before upgrades
let snapshot_id = coordinator.execute_upgrade(block, state)?;

// Rollback if needed
let restored_state = coordinator.rollback_upgrade()?;
```

## Upgrade Process

### 1. Proposal (Governance)

```rust
// Create upgrade proposal
let proposal = UpgradeProposal {
    id: next_id(),
    proposer: validator_address,
    title: "Runtime v1.1.0 Upgrade",
    description: "Add staking rewards feature",
    new_version: RuntimeVersion::new(1, 1, 0),
    code_hash: compute_hash(&new_code),
    activation_delay: 100_800, // ~7 days
    migration_plan: Some(plan),
};

governance.propose_upgrade(proposal)?;
```

### 2. Voting

```rust
// Validators vote
governance.vote_upgrade(proposal_id, validator, true)?;

// Check if approved
if governance.is_approved(proposal_id) {
    // Schedule upgrade
    coordinator.schedule_upgrade(...)?;
}
```

### 3. Execution

```rust
// At activation block
if current_block >= activation_block {
    // Automatic execution
    let new_state = coordinator.execute_upgrade(
        current_block,
        current_state,
    )?;
    
    // Update chain state
    chain.update_state(new_state);
}
```

### 4. Validation

```rust
// Pre-upgrade validation
validator.validate_upgrade(
    &current_version,
    &new_version,
    &code,
)?;

// Post-upgrade validation
validator.validate_post_upgrade(
    &old_state_root,
    &new_state_root,
)?;
```

## Safety Features

### Automatic Snapshots

Before every upgrade, a snapshot is automatically created:

```rust
// Snapshot created automatically
let snapshot = coordinator.execute_upgrade(...)?;

// Can be restored if needed
let state = coordinator.rollback_upgrade()?;
```

### Version Validation

```rust
// Only compatible upgrades allowed
if !current.can_upgrade_to(&new) {
    return Err("Incompatible version");
}

// No skipping major versions
if new.major > current.major + 1 {
    return Err("Cannot skip major versions");
}
```

### State Integrity

```rust
// State root must change
if old_root == new_root {
    return Err("State not migrated");
}

// Invariants preserved
verify_total_supply(old_state, new_state)?;
verify_account_balances(old_state, new_state)?;
```

## Testing

### Unit Tests

```bash
cargo test -p runtime
```

### Integration Tests

```rust
#[test]
fn test_full_upgrade_flow() {
    let mut coordinator = UpgradeCoordinator::new(v1_0_0);
    
    // Schedule
    coordinator.schedule_upgrade(v1_1_0, code, 1000, None)?;
    
    // Execute
    let new_state = coordinator.execute_upgrade(1000, state)?;
    
    // Verify
    assert_eq!(coordinator.current_version(), &v1_1_0);
}
```

## Configuration

### Upgrade Parameters

```toml
[upgrade]
# Minimum delay after approval (blocks)
timelock_delay = 100800  # ~7 days at 6s blocks

# Maximum snapshots to keep
max_snapshots = 10

# Governance requirements
quorum_threshold = 0.334
approval_threshold = 0.667
```

## Monitoring

### Events

- `UpgradeScheduled` - Upgrade scheduled for future block
- `UpgradeExecuted` - Upgrade successfully executed
- `UpgradeFailed` - Upgrade failed, rolled back
- `UpgradeCancelled` - Upgrade cancelled before execution

### Metrics

- Current runtime version
- Pending upgrade status
- Snapshot count
- Upgrade success rate

## Best Practices

### 1. Testing

- Test migrations on testnet first
- Verify state integrity after migration
- Test rollback procedures

### 2. Communication

- Announce upgrades well in advance
- Document breaking changes
- Provide migration guides

### 3. Gradual Rollout

- Start with testnet
- Monitor for issues
- Have rollback plan ready

### 4. Governance

- Require high approval threshold
- Use timelock for safety
- Allow emergency cancellation

## Emergency Procedures

### Cancel Upgrade

```rust
// Before execution
coordinator.cancel_upgrade()?;
```

### Rollback

```rust
// After execution if issues found
let restored_state = coordinator.rollback_upgrade()?;
```

### Emergency Pause

```rust
// Pause chain if critical issue
chain.pause()?;

// Rollback upgrade
coordinator.rollback_upgrade()?;

// Resume chain
chain.resume()?;
```

## Examples

### Simple Upgrade

```rust
// v1.0.0 → v1.1.0 (no state changes)
coordinator.schedule_upgrade(
    RuntimeVersion::new(1, 1, 0),
    new_code,
    activation_block,
    None, // No migration needed
)?;
```

### Upgrade with Migration

```rust
// v1.0.0 → v2.0.0 (breaking changes)
let migration_plan = MigrationPlan {
    from_version: v1_0_0,
    to_version: v2_0_0,
    migration_names: vec!["add_staking".to_string()],
    estimated_duration_secs: 300,
    requires_pause: false,
};

coordinator.schedule_upgrade(
    v2_0_0,
    new_code,
    activation_block,
    Some(migration_plan),
)?;
```

## Troubleshooting

### Upgrade Fails

1. Check logs for error details
2. Verify version compatibility
3. Validate migration logic
4. Rollback if needed

### State Corruption

1. Automatic rollback triggered
2. Restore from snapshot
3. Investigate migration bug
4. Fix and retry

### Consensus Issues

1. Emergency pause chain
2. Rollback upgrade
3. Coordinate with validators
4. Resume after fix

## License

MIT
