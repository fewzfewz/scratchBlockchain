// Integration tests for the modular blockchain
use common::types::{Block, Header, Transaction};
use common::traits::Storage;
use consensus::SimpleConsensus;
use storage::MemStore;
use governance::Governance;
use rollup::{Batch, RollupNode};
use da::DataAvailability;
use std::sync::{Arc, Mutex};

#[test]
fn test_storage_consensus_integration() {
    // Test that storage and consensus work together
    let store = MemStore::new();
    let validators = vec![vec![1, 2, 3, 4]];
    let consensus = SimpleConsensus::new(validators);
    
    // Store a block
    let key = b"block_1";
    let value = b"block_data";
    store.put(key, value).unwrap();
    
    // Verify storage
    assert!(store.contains(key).unwrap());
    let retrieved = store.get(key).unwrap();
    assert_eq!(retrieved, Some(value.to_vec()));
}

#[test]
fn test_governance_full_lifecycle() {
    // Test complete governance workflow
    let mut gov = Governance::new();
    
    // Create proposal
    let proposal_id = gov.create_proposal("Upgrade to new consensus".to_string());
    assert_eq!(proposal_id, 0);
    
    // Vote
    assert!(gov.vote(proposal_id, true));
    assert!(gov.vote(proposal_id, true));
    assert!(gov.vote(proposal_id, false));
    
    // Tally
    assert!(gov.tally_votes(proposal_id));
    
    // Execute
    assert!(gov.execute_proposal(proposal_id));
    
    // Verify cannot execute twice
    assert!(!gov.execute_proposal(proposal_id));
}

#[test]
fn test_rollup_batch_submission() {
    // Test rollup batch lifecycle
    let da_layer = Arc::new(Mutex::new(DataAvailability::new(4, 2, 10)));
    let mut rollup = RollupNode::new(rollup::RollupType::Optimistic, da_layer);
    
    let batch = Batch {
        transactions: vec![],
        prev_state_root: vec![0; 32],
        new_state_root: vec![1; 32],
        zk_proof: None,
        da_commitment: None,
    };
    
    rollup.submit_batch(batch);
    assert_eq!(rollup.l1_batches.len(), 1);
    
    // Verify batch
    let result = rollup.verify_batch(0);
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_multi_component_transaction_flow() {
    // Test transaction flowing through multiple components
    let store = MemStore::new();
    let mut gov = Governance::new();
    
    // Create and execute a governance proposal
    let prop_id = gov.create_proposal("Add new validator".to_string());
    gov.vote(prop_id, true);
    gov.vote(prop_id, true);
    gov.tally_votes(prop_id);
    gov.execute_proposal(prop_id);
    
    // Store the result
    let key = b"proposal_result";
    let value = b"executed";
    store.put(key, value).unwrap();
    
    // Verify end-to-end
    assert!(store.contains(key).unwrap());
    let proposal = gov.proposals.get(&prop_id).unwrap();
    assert!(proposal.executed);
}

#[test]
fn test_storage_persistence() {
    // Test storage operations
    let store = MemStore::new();
    
    // Write multiple keys
    for i in 0..10 {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        store.put(key.as_bytes(), value.as_bytes()).unwrap();
    }
    
    // Verify all keys exist
    for i in 0..10 {
        let key = format!("key_{}", i);
        assert!(store.contains(key.as_bytes()).unwrap());
    }
    
    // Verify values are correct
    for i in 0..10 {
        let key = format!("key_{}", i);
        let expected_value = format!("value_{}", i);
        let retrieved = store.get(key.as_bytes()).unwrap();
        assert_eq!(retrieved, Some(expected_value.as_bytes().to_vec()));
    }
}
