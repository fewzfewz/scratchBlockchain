use interop::ethereum_bridge::{EthereumBridge, BridgeMessage};
use proptest::prelude::*;
use common::types::Address;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn fuzz_bridge_message_unlock(
        id in any::<u64>(),
        source_chain in any::<u32>(),
        dest_chain in any::<u32>(),
        sender in any::<[u8; 20]>(),
        recipient in any::<[u8; 20]>(),
        token in any::<[u8; 20]>(),
        amount in any::<u128>(),
        nonce in any::<u64>(),
        signatures in proptest::collection::vec(proptest::collection::vec(any::<u8>(), 64), 0..5)
    ) {
        let relayers = vec![[1u8; 20], [2u8; 20], [3u8; 20]];
        let mut bridge = EthereumBridge::new(1, 2, relayers, 2);

        let message = BridgeMessage {
            id,
            source_chain,
            dest_chain,
            sender,
            recipient,
            token,
            amount,
            nonce,
            signatures,
        };

        // We expect this to fail most of the time with random data, but it should never panic
        let _ = bridge.unlock_tokens(message);
    }
}

fn main() {
    // This is a placeholder main for the binary target
    println!("Run with `cargo test` to execute proptest fuzzing");
}
