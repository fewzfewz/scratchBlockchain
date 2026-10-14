//! Header-following light client.
//!
//! Starts from a trusted root (a block hash and the validator set that
//! certifies the next block, e.g. from genesis or a checkpoint obtained out of
//! band) and accepts each following block only if
//!
//! 1. it extends the trusted block (`parent_hash`, consecutive slot), and
//! 2. its commit certificate (`block.commit`) carries precommits from more
//!    than 2/3 of the trusted validator set's stake for exactly that block.
//!
//! That is the same finality check a full node applies to synced blocks, so a
//! light client cannot be fed a block that the validators did not finalize.
//! The header's `state_root` then commits to the chain state after the
//! block's parent, which callers can check state proofs against.
//!
//! Limitation: headers do not yet commit to the next validator set, so this
//! client cannot follow a validator-set change on its own. When a block's
//! certificate no longer reaches quorum in the trusted set, `verify_next`
//! returns [`LightClientError::ValidatorSetChanged`] and the caller must
//! re-anchor with a newly trusted set.

use common::types::{Block, Hash};
use consensus::ValidatorInfo;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LightClientError {
    /// The block does not extend the trusted block.
    NotNext {
        expected_slot: u64,
        got_slot: u64,
    },
    ParentMismatch,
    MissingCommit,
    /// The certificate is malformed or for another block.
    InvalidCommit(String),
    /// The certificate's signers do not reach quorum in the trusted set,
    /// which is what a validator-set change looks like from here.
    ValidatorSetChanged(String),
}

impl fmt::Display for LightClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotNext {
                expected_slot,
                got_slot,
            } => write!(f, "expected slot {}, got {}", expected_slot, got_slot),
            Self::ParentMismatch => write!(f, "block does not extend the trusted block"),
            Self::MissingCommit => write!(f, "block has no commit certificate"),
            Self::InvalidCommit(e) => write!(f, "invalid commit certificate: {}", e),
            Self::ValidatorSetChanged(e) => {
                write!(f, "no quorum in the trusted validator set: {}", e)
            }
        }
    }
}

impl std::error::Error for LightClientError {}

/// Check that `block` was finalized by `validators` (the set at BFT height
/// `slot + 1`). Returns the public keys whose precommits counted.
pub fn verify_finality(
    block: &Block,
    validators: &[ValidatorInfo],
) -> Result<Vec<Vec<u8>>, LightClientError> {
    let cert = block
        .commit
        .as_ref()
        .ok_or(LightClientError::MissingCommit)?;
    let hash = block.hash();
    let height = block.header.slot.saturating_add(1);
    if cert.block_hash != hash || cert.height != height {
        return Err(LightClientError::InvalidCommit(
            "certificate is for a different block".into(),
        ));
    }
    cert.verify(&hash, height, validators).map_err(|e| {
        if e.contains("quorum") {
            LightClientError::ValidatorSetChanged(e)
        } else {
            LightClientError::InvalidCommit(e)
        }
    })
}

#[derive(Debug, Clone)]
pub struct LightClient {
    trusted_slot: u64,
    trusted_hash: Hash,
    trusted_state_root: Hash,
    validators: Vec<ValidatorInfo>,
}

impl LightClient {
    /// Trust `block` (verified out of band) and `validators` as the set that
    /// certifies the blocks after it.
    pub fn new(trusted: &Block, validators: Vec<ValidatorInfo>) -> Self {
        Self {
            trusted_slot: trusted.header.slot,
            trusted_hash: trusted.hash(),
            trusted_state_root: trusted.header.state_root,
            validators,
        }
    }

    pub fn trusted_slot(&self) -> u64 {
        self.trusted_slot
    }

    pub fn trusted_hash(&self) -> Hash {
        self.trusted_hash
    }

    /// State root committed by the latest trusted header (the state after
    /// its parent block).
    pub fn trusted_state_root(&self) -> Hash {
        self.trusted_state_root
    }

    /// Verify the block after the trusted one and, if valid, trust it.
    pub fn verify_next(&mut self, block: &Block) -> Result<(), LightClientError> {
        let expected_slot = self.trusted_slot.saturating_add(1);
        if block.header.slot != expected_slot {
            return Err(LightClientError::NotNext {
                expected_slot,
                got_slot: block.header.slot,
            });
        }
        if block.header.parent_hash != self.trusted_hash {
            return Err(LightClientError::ParentMismatch);
        }
        verify_finality(block, &self.validators)?;
        self.trusted_slot = block.header.slot;
        self.trusted_hash = block.hash();
        self.trusted_state_root = block.header.state_root;
        Ok(())
    }

    /// Re-anchor on a new validator set after a set change, keeping the
    /// trusted block. The caller must trust `validators` out of band.
    pub fn reset_validators(&mut self, validators: Vec<ValidatorInfo>) {
        self.validators = validators;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::consensus_types::{CommitCertificate, Step, Vote};
    use common::types::Header;
    use ed25519_dalek::Signer;

    fn keys() -> Vec<ed25519_dalek::SigningKey> {
        (20u8..24)
            .map(|i| ed25519_dalek::SigningKey::from_bytes(&[i; 32]))
            .collect()
    }

    fn set(keys: &[ed25519_dalek::SigningKey]) -> Vec<ValidatorInfo> {
        keys.iter()
            .map(|k| ValidatorInfo {
                public_key: k.verifying_key().to_bytes().to_vec(),
                stake: 100,
                slashed: false,
            })
            .collect()
    }

    fn certify(block: &mut Block, signers: &[ed25519_dalek::SigningKey]) {
        let hash = block.hash();
        let height = block.header.slot + 1;
        let precommits = signers
            .iter()
            .map(|k| {
                let mut v = Vote {
                    height,
                    round: 0,
                    step: Step::Precommit,
                    block_hash: Some(hash),
                    signature: vec![],
                    voter: k.verifying_key().to_bytes().to_vec(),
                };
                v.signature = k.sign(&v.signing_bytes()).to_bytes().to_vec();
                v
            })
            .collect();
        block.commit = Some(CommitCertificate {
            height,
            round: 0,
            block_hash: hash,
            precommits,
        });
    }

    fn child(parent: &Block, root: u8) -> Block {
        let mut header = Header::new(parent.hash(), parent.header.slot + 1);
        header.state_root = [root; 32];
        Block::new(header, vec![])
    }

    #[test]
    fn follows_certified_chain_and_rejects_everything_else() {
        let ks = keys();
        let genesis = Block::genesis();
        let mut lc = LightClient::new(&genesis, set(&ks));

        let mut b1 = child(&genesis, 1);
        certify(&mut b1, &ks[..3]);
        lc.verify_next(&b1).unwrap();
        assert_eq!(lc.trusted_hash(), b1.hash());
        assert_eq!(lc.trusted_state_root(), [1; 32]);

        // Only 2 of 4 signed: not final.
        let mut weak = child(&b1, 2);
        certify(&mut weak, &ks[..2]);
        assert!(matches!(
            lc.verify_next(&weak),
            Err(LightClientError::ValidatorSetChanged(_))
        ));

        // Certificate copied from a different block.
        let mut forged = child(&b1, 3);
        forged.commit = b1.commit.clone();
        assert!(matches!(
            lc.verify_next(&forged),
            Err(LightClientError::InvalidCommit(_))
        ));

        // Signed by keys outside the trusted set.
        let outsiders: Vec<_> = (40u8..44)
            .map(|i| ed25519_dalek::SigningKey::from_bytes(&[i; 32]))
            .collect();
        let mut foreign = child(&b1, 4);
        certify(&mut foreign, &outsiders);
        assert!(lc.verify_next(&foreign).is_err());

        // Not linked to the trusted block, or skipping ahead.
        let mut orphan = child(&genesis, 5);
        certify(&mut orphan, &ks);
        assert!(lc.verify_next(&orphan).is_err());
        let mut b2 = child(&b1, 6);
        certify(&mut b2, &ks);
        let mut b3 = child(&b2, 7);
        certify(&mut b3, &ks);
        assert!(matches!(
            lc.verify_next(&b3),
            Err(LightClientError::NotNext { .. })
        ));

        // Nothing above moved the trust root; the real next block still works.
        assert_eq!(lc.trusted_hash(), b1.hash());
        lc.verify_next(&b2).unwrap();
        lc.verify_next(&b3).unwrap();
        assert_eq!(lc.trusted_slot(), 3);
    }
}
