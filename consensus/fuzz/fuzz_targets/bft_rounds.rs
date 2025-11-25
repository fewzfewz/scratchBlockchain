use consensus::bft::BftEngine;
use consensus::ValidatorInfo;
use common::consensus_types::Vote;
use common::crypto::SigningKey;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn fuzz_bft_rounds(
        round in any::<u64>(),
        step in 0u8..3u8,
        block_hash in any::<Option<[u8; 32]>>(),
        validator_idx in 0u8..3u8,
        signature in any::<[u8; 64]>()
    ) {
        let validators = vec![
            ValidatorInfo { public_key: vec![1u8; 20], stake: 100, slashed: false },
            ValidatorInfo { public_key: vec![2u8; 20], stake: 100, slashed: false },
            ValidatorInfo { public_key: vec![3u8; 20], stake: 100, slashed: false },
        ];
        
        let local_address = validators[0].public_key.clone();
        // Create a dummy signing key (in a real fuzz test we might want valid keys, but for structure fuzzing this is fine)
        // We'll just use a random key for now, as we can't easily generate a valid SigningKey from arbitrary bytes without ed25519-dalek dependency here
        // For now, let's assume we can construct one or mock it. 
        // Actually, SigningKey usually requires a specific structure. 
        // Let's see if we can generate a valid one or if we need to mock the signature verification.
        // Since we are fuzzing the engine logic, invalid signatures are a valid test case.
        
        // We need a valid SigningKey to construct BftEngine.
        // Let's generate a random keypair.
        let signing_key = SigningKey::generate();

        let mut engine = BftEngine::new(local_address, validators.clone(), 1, signing_key);

        let voter_idx = validator_idx as usize % validators.len();
        let voter = validators[voter_idx].public_key.clone();
        
        // Map u8 to Step enum
        let step_enum = match step % 3 {
            0 => common::consensus_types::Step::Propose,
            1 => common::consensus_types::Step::Prevote,
            _ => common::consensus_types::Step::Precommit,
        };

        let vote = Vote {
            height: 1,
            round,
            step: step_enum,
            block_hash,
            voter,
            signature: signature.to_vec(),
        };

        // This should fail signature verification most of the time, but shouldn't panic
        let _ = engine.handle_vote(vote);
    }
}

fn main() {
    println!("Run with `cargo test` to execute proptest fuzzing");
}
