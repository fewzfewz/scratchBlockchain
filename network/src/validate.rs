//! Stateless checks run on every gossip message before it is forwarded.
//!
//! They need no chain state, so the network layer can reject forgeries
//! itself: a message that fails here is never relayed and its sender is
//! penalized. Stateful checks (nonces, the validator set, rounds, state
//! roots) stay in the node.

use common::consensus_types::{ConsensusMessage, Step};
use common::types::{Block, Transaction};

/// Largest payload accepted from gossip (the gossipsub frame is 64 KiB).
const MAX_PAYLOAD_BYTES: usize = 60 * 1024;

pub fn transaction(tx: &Transaction) -> Result<(), String> {
    if tx.payload.len() > MAX_PAYLOAD_BYTES {
        return Err("payload too large".into());
    }
    // The chain ID itself is checked by the node; the signature over it is
    // checked here.
    let chain_id = tx.chain_id.ok_or("missing chain_id")?;
    common::validation::verify_tx_signature(tx, chain_id)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// A gossiped block was finalized, so it must carry a commit certificate
/// whose precommits are validly signed for this block, and only
/// authenticated transactions. (Whether the signers hold +2/3 of the stake
/// needs the validator set and is checked by the node during sync.)
pub fn block(block: &Block) -> Result<(), String> {
    let cert = block
        .commit
        .as_ref()
        .ok_or("block has no commit certificate")?;
    let hash = block.hash();
    if cert.block_hash != hash || cert.height != block.header.slot + 1 {
        return Err("commit certificate is for a different block".into());
    }
    if cert.precommits.is_empty() {
        return Err("empty commit certificate".into());
    }
    for vote in &cert.precommits {
        if vote.step != Step::Precommit
            || vote.block_hash != Some(hash)
            || vote.height != cert.height
            || vote.round != cert.round
            || !vote.verify_signature()
        {
            return Err("invalid precommit in commit certificate".into());
        }
    }
    block.extrinsics.iter().try_for_each(transaction)
}

pub fn consensus_message(msg: &ConsensusMessage) -> Result<(), String> {
    match msg {
        ConsensusMessage::Vote(vote) => {
            if vote.verify_signature() {
                Ok(())
            } else {
                Err("invalid vote signature".into())
            }
        }
        ConsensusMessage::Proposal(proposal) => {
            if !proposal.verify_signature() {
                return Err("invalid proposal signature".into());
            }
            proposal.block.extrinsics.iter().try_for_each(transaction)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::consensus_types::{CommitCertificate, Proposal, Vote};
    use common::crypto::SigningKey;
    use common::types::Header;

    fn signed_tx() -> Transaction {
        let key = ed25519_dalek::SigningKey::from_bytes(&[5u8; 32]);
        let mut tx = Transaction::simple_transfer([0; 20], [1; 20], 7, 0, 1);
        tx.sign_with(&key);
        tx
    }

    fn signed_vote(key: &SigningKey, hash: [u8; 32], height: u64) -> Vote {
        let mut vote = Vote {
            height,
            round: 0,
            step: Step::Precommit,
            block_hash: Some(hash),
            signature: vec![],
            voter: key.public_key(),
        };
        vote.signature = key.sign(&vote.signing_bytes());
        vote
    }

    #[test]
    fn transactions() {
        assert!(transaction(&signed_tx()).is_ok());
        let mut forged = signed_tx();
        forged.value = 1_000;
        assert!(transaction(&forged).is_err());
        let mut huge = signed_tx();
        huge.payload = vec![0; MAX_PAYLOAD_BYTES + 1];
        assert!(transaction(&huge).is_err());
    }

    #[test]
    fn votes_and_proposals() {
        let key = SigningKey::generate();
        let vote = signed_vote(&key, [1; 32], 3);
        assert!(consensus_message(&ConsensusMessage::Vote(vote.clone())).is_ok());
        let mut forged = vote;
        forged.round = 9; // signature no longer matches
        assert!(consensus_message(&ConsensusMessage::Vote(forged)).is_err());

        let mut proposal = Proposal {
            height: 3,
            round: 0,
            block: Block::new(Header::new([0; 32], 2), vec![signed_tx()]),
            signature: vec![],
            proposer: key.public_key(),
        };
        proposal.signature = key.sign(&proposal.signing_bytes());
        assert!(consensus_message(&ConsensusMessage::Proposal(proposal.clone())).is_ok());
        let mut tampered = proposal;
        tampered.block.extrinsics[0].value += 1;
        assert!(consensus_message(&ConsensusMessage::Proposal(tampered)).is_err());
    }

    #[test]
    fn blocks_need_a_signed_certificate() {
        let key = SigningKey::generate();
        let mut b = Block::new(Header::new([0; 32], 4), vec![]);
        assert!(block(&b).is_err(), "no certificate");

        let hash = b.hash();
        b.commit = Some(CommitCertificate {
            height: 5,
            round: 0,
            block_hash: hash,
            precommits: vec![signed_vote(&key, hash, 5)],
        });
        assert!(block(&b).is_ok());

        let mut wrong = b.clone();
        wrong.header.slot = 9; // certificate no longer matches the block
        assert!(block(&wrong).is_err());
    }

    /// Deterministic xorshift, so failures reproduce.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n.max(1) as u64) as usize
        }
    }

    /// Decode a gossip payload as each message type and validate whatever
    /// decodes — exactly what the network layer does with untrusted bytes.
    fn decode_and_validate(bytes: &[u8]) {
        if let Ok(m) = serde_json::from_slice::<crate::TransactionMessage>(bytes) {
            let _ = transaction(&m.transaction);
        }
        if let Ok(m) = serde_json::from_slice::<crate::BlockMessage>(bytes) {
            let _ = block(&m.block);
        }
        if let Ok(m) = serde_json::from_slice::<ConsensusMessage>(bytes) {
            let _ = consensus_message(&m);
        }
    }

    /// Fuzz-style: random bytes and byte-level mutations of valid messages
    /// must never panic the decoder or validators (a panic here would let
    /// one gossip message crash every node that receives it).
    #[test]
    fn untrusted_gossip_never_panics() {
        let key = SigningKey::generate();
        let tx = signed_tx();
        let mut b = Block::new(Header::new([0; 32], 4), vec![tx.clone()]);
        let hash = b.hash();
        b.commit = Some(CommitCertificate {
            height: 5,
            round: 0,
            block_hash: hash,
            precommits: vec![signed_vote(&key, hash, 5)],
        });
        let seeds: Vec<Vec<u8>> = vec![
            serde_json::to_vec(&crate::TransactionMessage { transaction: tx }).unwrap(),
            serde_json::to_vec(&crate::BlockMessage { block: b }).unwrap(),
            serde_json::to_vec(&ConsensusMessage::Vote(signed_vote(&key, hash, 5))).unwrap(),
        ];

        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for round in 0..3_000 {
            let mut bytes = seeds[round % seeds.len()].clone();
            for _ in 0..1 + rng.below(8) {
                let i = rng.below(bytes.len());
                match rng.below(4) {
                    0 => bytes[i] = rng.next() as u8,
                    1 => bytes[i] = b"0123456789-[]{}\":,e".as_slice()[rng.below(19)],
                    2 => {
                        bytes.remove(i);
                    }
                    _ => bytes.insert(i, rng.next() as u8),
                }
                if bytes.is_empty() {
                    break;
                }
            }
            decode_and_validate(&bytes);
        }
        for _ in 0..2_000 {
            let len = rng.below(256);
            let bytes: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
            decode_and_validate(&bytes);
        }
    }
}
