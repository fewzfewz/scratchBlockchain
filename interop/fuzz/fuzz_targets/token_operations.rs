use interop::ethereum_bridge::EthereumBridge;
use proptest::prelude::*;
use common::types::Address;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn fuzz_lock_tokens(
        user in any::<[u8; 20]>(),
        token in any::<[u8; 20]>(),
        amount in any::<u128>(),
        eth_recipient in any::<[u8; 20]>()
    ) {
        let relayers = vec![[1u8; 20], [2u8; 20], [3u8; 20]];
        let mut bridge = EthereumBridge::new(1, 2, relayers, 2);

        let result = bridge.lock_tokens(user, token, amount, eth_recipient);

        if amount == 0 {
            assert!(result.is_err());
        } else {
            assert!(result.is_ok());
            let message = result.unwrap();
            assert_eq!(message.amount, amount);
            assert_eq!(message.sender, user);
            assert_eq!(message.recipient, eth_recipient);
            
            // Verify internal state
            let locked = bridge.get_locked_balance(&token, &user);
            assert_eq!(locked, amount);
        }
    }
}

fn main() {
    println!("Run with `cargo test` to execute proptest fuzzing");
}
