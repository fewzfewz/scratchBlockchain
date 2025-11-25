use governance::{
    InflationSchedule, StakingContract, SlashingReason, Treasury,
    Delegation, ValidatorMetadata,
};
use common::types::Address;

#[test]
fn test_inflation_schedule_halving() {
    let schedule = InflationSchedule::default();
    
    // Initial reward
    let reward_0 = schedule.calculate_reward(0);
    assert_eq!(reward_0, 10_000_000_000); // 10 tokens
    
    // Just before first halving
    let reward_before = schedule.calculate_reward(2_099_999);
    assert_eq!(reward_before, 10_000_000_000);
    
    // At first halving
    let reward_halving_1 = schedule.calculate_reward(2_100_000);
    assert_eq!(reward_halving_1, 5_000_000_000); // 5 tokens
    
    // At second halving
    let reward_halving_2 = schedule.calculate_reward(4_200_000);
    assert_eq!(reward_halving_2, 2_500_000_000); // 2.5 tokens
    
    // At third halving
    let reward_halving_3 = schedule.calculate_reward(6_300_000);
    assert_eq!(reward_halving_3, 1_250_000_000); // 1.25 tokens
}

#[test]
fn test_inflation_schedule_eventual_zero() {
    let schedule = InflationSchedule::default();
    
    // After 64 halvings, reward should be 0
    let reward = schedule.calculate_reward(64 * 2_100_000);
    assert_eq!(reward, 0);
}

#[test]
fn test_fee_burn_calculation() {
    let schedule = InflationSchedule::default();
    
    let total_fee = 1_000_000_000; // 1 token
    let burn = schedule.calculate_fee_burn(total_fee);
    
    // Should burn 50%
    assert_eq!(burn, 500_000_000);
}

#[test]
fn test_staking_contract_register_validator() {
    let mut contract = StakingContract::new(1000_000_000_000); // 1000 tokens min
    
    let validator_addr = Address::default();
    let pubkey = vec![1u8; 32];
    
    // Register with sufficient stake
    let result = contract.register_validator(
        validator_addr,
        pubkey.clone(),
        1000_000_000_000,
        10, // 10% commission
    );
    assert!(result.is_ok());
    
    // Try to register same validator again
    let result = contract.register_validator(
        validator_addr,
        pubkey,
        1000_000_000_000,
        10,
    );
    assert!(result.is_err());
}

#[test]
fn test_staking_contract_min_stake() {
    let mut contract = StakingContract::new(1000_000_000_000);
    
    let validator_addr = Address::default();
    let pubkey = vec![1u8; 32];
    
    // Try to register with insufficient stake
    let result = contract.register_validator(
        validator_addr,
        pubkey,
        500_000_000_000, // Below minimum
        10,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("below minimum"));
}

#[test]
fn test_delegation() {
    let mut contract = StakingContract::new(1000_000_000_000);
    
    let validator_addr = [1u8; 20];
    let delegator_addr = [2u8; 20];
    let pubkey = vec![1u8; 32];
    
    // Register validator
    contract.register_validator(validator_addr, pubkey, 1000_000_000_000, 10).unwrap();
    
    // Delegate to validator
    let result = contract.delegate(
        delegator_addr,
        validator_addr,
        500_000_000_000,
        0,
    );
    assert!(result.is_ok());
    
    // Check delegations
    let delegations = contract.get_delegations(&delegator_addr);
    assert_eq!(delegations.len(), 1);
    assert_eq!(delegations[0].amount, 500_000_000_000);
    assert_eq!(delegations[0].validator, validator_addr);
}

#[test]
fn test_undelegation_and_unbonding() {
    let mut contract = StakingContract::new(1000_000_000_000);
    
    let validator_addr = [1u8; 20];
    let delegator_addr = [2u8; 20];
    let pubkey = vec![1u8; 32];
    
    // Register and delegate
    contract.register_validator(validator_addr, pubkey, 1000_000_000_000, 10).unwrap();
    contract.delegate(delegator_addr, validator_addr, 500_000_000_000, 0).unwrap();
    
    // Undelegate
    let result = contract.undelegate(delegator_addr, validator_addr, 200_000_000_000, 100);
    assert!(result.is_ok());
    
    // Check remaining delegation
    let delegations = contract.get_delegations(&delegator_addr);
    assert_eq!(delegations.len(), 1);
    assert_eq!(delegations[0].amount, 300_000_000_000);
    
    // Process unbonding before completion - should return empty
    let completed = contract.process_unbonding(100_000);
    assert_eq!(completed.len(), 0);
    
    // Process unbonding after completion
    let completed = contract.process_unbonding(100_900); // After unbonding period
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].0, delegator_addr);
    assert_eq!(completed[0].1, 200_000_000_000);
}

#[test]
fn test_slashing_double_sign() {
    let mut contract = StakingContract::new(1000_000_000_000);
    
    let validator_addr = [1u8; 20];
    let pubkey = vec![1u8; 32];
    
    // Register validator with 1000 tokens
    contract.register_validator(validator_addr, pubkey, 1000_000_000_000, 10).unwrap();
    
    // Slash for double sign (5%)
    let slashed = contract.slash(validator_addr, SlashingReason::DoubleSign, 1000).unwrap();
    
    // Should slash 5% = 50 tokens
    assert_eq!(slashed, 50_000_000_000);
    
    // Treasury should receive slashed amount
    assert_eq!(contract.treasury_balance(), 50_000_000_000);
}

#[test]
fn test_slashing_downtime() {
    let mut contract = StakingContract::new(1000_000_000_000);
    
    let validator_addr = [1u8; 20];
    let pubkey = vec![1u8; 32];
    
    // Register validator
    contract.register_validator(validator_addr, pubkey, 1000_000_000_000, 10).unwrap();
    
    // Slash for downtime (0.1%)
    let slashed = contract.slash(validator_addr, SlashingReason::Downtime, 1000).unwrap();
    
    // Should slash 0.1% = 1 token
    assert_eq!(slashed, 1_000_000_000);
}

#[test]
fn test_auto_slash_on_missed_blocks() {
    let mut contract = StakingContract::new(1000_000_000_000);
    
    let validator_addr = [1u8; 20];
    let pubkey = vec![1u8; 32];
    
    // Register validator
    contract.register_validator(validator_addr, pubkey, 1000_000_000_000, 10).unwrap();
    
    let initial_treasury = contract.treasury_balance();
    
    // Record 99 missed blocks - should not slash
    for _ in 0..99 {
        contract.record_missed_block(validator_addr).unwrap();
    }
    assert_eq!(contract.treasury_balance(), initial_treasury);
    
    // 100th missed block - should trigger auto-slash
    contract.record_missed_block(validator_addr).unwrap();
    assert!(contract.treasury_balance() > initial_treasury);
}

#[test]
fn test_reward_distribution() {
    let mut contract = StakingContract::new(1000_000_000_000);
    
    let validator_addr = [1u8; 20];
    let delegator_addr = [2u8; 20];
    let pubkey = vec![1u8; 32];
    
    // Register validator with 1000 tokens, 10% commission
    contract.register_validator(validator_addr, pubkey, 1000_000_000_000, 10).unwrap();
    
    // Delegator stakes 1000 tokens
    contract.delegate(delegator_addr, validator_addr, 1000_000_000_000, 0).unwrap();
    
    // Distribute 100 tokens block reward + 20 tokens fees
    let block_reward = 100_000_000_000;
    let fees = 20_000_000_000;
    
    contract.distribute_rewards(validator_addr, block_reward, fees, 1000).unwrap();
    
    // Check delegations were updated with rewards
    let delegations = contract.get_delegations(&delegator_addr);
    assert!(delegations[0].rewards_earned > 0);
    
    // Treasury should have received 10% of block reward
    assert_eq!(contract.treasury_balance(), 10_000_000_000);
}

#[test]
fn test_treasury_operations() {
    let mut treasury = Treasury::new();
    
    assert_eq!(treasury.balance, 0);
    
    // Deposit
    treasury.deposit(1000);
    assert_eq!(treasury.balance, 1000);
    assert_eq!(treasury.total_collected, 1000);
    
    // Spend
    let result = treasury.spend(500);
    assert!(result.is_ok());
    assert_eq!(treasury.balance, 500);
    assert_eq!(treasury.total_spent, 500);
    
    // Try to spend more than balance
    let result = treasury.spend(1000);
    assert!(result.is_err());
}

#[test]
fn test_validator_sorting_by_total_stake() {
    let mut contract = StakingContract::new(1000_000_000_000);
    
    let val1 = [1u8; 20];
    let val2 = [2u8; 20];
    let val3 = [3u8; 20];
    
    // Register validators with different stakes
    contract.register_validator(val1, vec![1u8; 32], 1000_000_000_000, 10).unwrap();
    contract.register_validator(val2, vec![2u8; 32], 2000_000_000_000, 10).unwrap();
    contract.register_validator(val3, vec![3u8; 32], 1500_000_000_000, 10).unwrap(); // Above minimum
    
    // Add delegations to val3 to make it have highest total stake
    contract.delegate([10u8; 20], val3, 3000_000_000_000, 0).unwrap();
    
    let active = contract.get_active_validators();
    
    // Should be sorted by total stake (self + delegated)
    // val3: 1500 + 3000 = 4500 (highest)
    // val2: 2000 + 0 = 2000
    // val1: 1000 + 0 = 1000
    assert_eq!(active[0].0.address, val3);
    assert_eq!(active[1].0.address, val2);
    assert_eq!(active[2].0.address, val1);
}

#[test]
fn test_commission_rate_validation() {
    let mut contract = StakingContract::new(1000_000_000_000);
    
    let validator_addr = Address::default();
    let pubkey = vec![1u8; 32];
    
    // Try to register with invalid commission (>100%)
    let result = contract.register_validator(
        validator_addr,
        pubkey.clone(),
        1000_000_000_000,
        150, // Invalid
    );
    assert!(result.is_err());
    
    // Valid commission rates
    for rate in [0, 10, 50, 100] {
        let addr = [rate as u8; 20];
        let result = contract.register_validator(
            addr,
            pubkey.clone(),
            1000_000_000_000,
            rate,
        );
        assert!(result.is_ok());
    }
}

#[test]
fn test_max_validators_limit() {
    let mut contract = StakingContract::new(1000_000_000_000);
    
    // Register 100 validators (the maximum)
    for i in 0..100 {
        let addr = [i as u8; 20];
        let result = contract.register_validator(
            addr,
            vec![i as u8; 32],
            1000_000_000_000,
            10,
        );
        assert!(result.is_ok());
    }
    
    // Try to register 101st validator
    let result = contract.register_validator(
        [255u8; 20],
        vec![255u8; 32],
        1000_000_000_000,
        10,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Maximum validators"));
}
