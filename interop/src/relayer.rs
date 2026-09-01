use crate::{BridgeContract, CrossChainMessage};
use std::time::Duration;
use tokio::time::sleep;

pub struct Relayer {
    pub chain_a: BridgeContract,
    pub chain_b: BridgeContract,
}

impl Relayer {
    pub fn new(chain_a: BridgeContract, chain_b: BridgeContract) -> Self {
        Self { chain_a, chain_b }
    }

    pub async fn start(&mut self) {
        println!(
            "Relayer started between {} and {}",
            self.chain_a.chain_id, self.chain_b.chain_id
        );

        loop {
            sleep(Duration::from_secs(5)).await;
        }
    }

    /// Relay a cross-chain message to the destination chain bridge contract.
    pub fn process_message(
        &mut self,
        msg: CrossChainMessage,
        relayer_pubkey: Vec<u8>,
        relayer_sig: Vec<u8>,
    ) -> Result<(), String> {
        println!(
            "Relaying message from {} to {} (amount {})",
            msg.source_chain, msg.dest_chain, msg.amount
        );

        if msg.dest_chain == self.chain_a.chain_id {
            self.chain_a
                .unlock_assets(msg, relayer_sig, relayer_pubkey)
        } else if msg.dest_chain == self.chain_b.chain_id {
            self.chain_b
                .unlock_assets(msg, relayer_sig, relayer_pubkey)
        } else {
            Err(format!(
                "Unknown destination chain: {} (configured: {} / {})",
                msg.dest_chain, self.chain_a.chain_id, self.chain_b.chain_id
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_message_unlocks_on_dest_chain() {
        let relayer_pk = vec![1u8; 32];
        let mut nebula = BridgeContract::new("nebula".into(), vec![relayer_pk.clone()]);
        let mut eth = BridgeContract::new("ethereum".into(), vec![relayer_pk.clone()]);
        let mut relayer = Relayer::new(nebula, eth);

        let msg = CrossChainMessage {
            source_chain: "nebula".into(),
            dest_chain: "ethereum".into(),
            nonce: 42,
            sender: [2u8; 20],
            recipient: [3u8; 20],
            amount: 1_000,
            payload: vec![],
        };

        relayer
            .process_message(msg, relayer_pk, vec![])
            .expect("unlock on ethereum");
    }
}
