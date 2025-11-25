use consensus::{FinalityGadget, FinalityVote, ValidatorInfo};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn fuzz_finality_voting(
        block_number in any::<u64>(),
        block_hash in any::<[u8; 32]>(),
        validator_idx in 0u8..3u8,
        signature in any::<[u8; 64]>(),
        is_prevote in any::<bool>()
    ) {
        let validators = vec![
            ValidatorInfo { public_key: vec![1u8; 20], stake: 100, slashed: false },
            ValidatorInfo { public_key: vec![2u8; 20], stake: 100, slashed: false },
            ValidatorInfo { public_key: vec![3u8; 20], stake: 100, slashed: false },
        ];
        
        let mut gadget = FinalityGadget::new(validators.clone());
        let voter_idx = validator_idx as usize % validators.len();
        let voter_key = validators[voter_idx].public_key.clone();

        let vote = FinalityVote {
            block_hash,
            block_number,

            voter: voter_key,
            signature: signature.to_vec(),
        };

        // This should fail signature verification most of the time, but shouldn't panic
        if is_prevote {
            let _ = gadget.prevote(vote);
        } else {
            let _ = gadget.precommit(vote);
        }
    }
}

fn main() {
    println!("Run with `cargo test` to execute proptest fuzzing");
}
