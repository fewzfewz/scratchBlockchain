//! Adversarial slashing scenarios — double-sign and downtime detection.

use consensus::slashing::{SlashingConfig, SlashingTracker};
use consensus::SlashingCondition;

#[test]
fn adversarial_double_sign_triggers_slash() {
    let mut tracker = SlashingTracker::new(SlashingConfig::default());
    let validator = vec![0xAB; 32];
    tracker.record_double_sign(validator.clone(), 100, [1u8; 32], [2u8; 32]);
    assert!(tracker.check_double_sign_slash(&validator, 100));
}

#[test]
fn adversarial_downtime_accumulates() {
    let mut tracker = SlashingTracker::new(SlashingConfig::default());
    let validator = vec![0xCD; 32];
    for h in 0..5 {
        tracker.record_missed_block(validator.clone(), h);
    }
    let pct = tracker.get_liveness_percentage(&validator);
    assert!(pct < 100.0);
}

#[test]
fn adversarial_slashed_validator_skips_repeat() {
    let mut tracker = SlashingTracker::new(SlashingConfig::default());
    let validator = vec![0xEF; 32];
    tracker
        .slash(
            validator.clone(),
            SlashingCondition::DoubleSign {
                height: 1,
                validator: validator.clone(),
            },
        )
        .unwrap();
    assert!(tracker.is_slashed(&validator));
    tracker.record_double_sign(validator.clone(), 2, [2u8; 32], [3u8; 32]);
    assert!(tracker.is_slashed(&validator));
}
