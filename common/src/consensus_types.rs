use crate::types::{Block, Hash};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Domain tags so a vote signature can never be replayed as a proposal (or a
/// transaction / header) signature, and to version the encodings.
pub const VOTE_DOMAIN: &[u8] = b"NEBULA_VOTE_V1";
pub const PROPOSAL_DOMAIN: &[u8] = b"NEBULA_PROPOSAL_V1";

/// BFT quorum: strictly more than two thirds of the total stake.
///
/// With `n = 3f + 1` equal-stake validators this needs `2f + 1` votes, so two
/// quorums always share at least one honest validator. (`>= floor(2n/3)` let
/// 2 of 4 validators finalize, so two disjoint pairs could finalize conflicting
/// blocks.)
pub fn is_quorum(stake: u128, total_stake: u128) -> bool {
    total_stake > 0 && stake.saturating_mul(3) > total_stake.saturating_mul(2)
}

/// More than one third of the stake: at least one honest validator is in any
/// such set, so it is safe to follow it (e.g. to skip to a higher round).
pub fn is_one_third_plus(stake: u128, total_stake: u128) -> bool {
    total_stake > 0 && stake.saturating_mul(3) > total_stake
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Step {
    Propose,
    Prevote,
    Precommit,
    Commit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vote {
    pub height: u64,
    pub round: u64,
    pub step: Step,
    pub block_hash: Option<Hash>, // None for nil
    pub signature: Vec<u8>,
    pub voter: Vec<u8>,
}

impl Vote {
    /// Canonical bytes the voter signs (domain-separated, nil vs. block tagged).
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(VOTE_DOMAIN.len() + 96);
        bytes.extend_from_slice(VOTE_DOMAIN);
        bytes.extend_from_slice(&self.height.to_le_bytes());
        bytes.extend_from_slice(&self.round.to_le_bytes());
        bytes.push(self.step as u8);
        match &self.block_hash {
            Some(hash) => {
                bytes.push(1);
                bytes.extend_from_slice(hash);
            }
            None => bytes.push(0),
        }
        bytes.extend_from_slice(&(self.voter.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&self.voter);
        bytes
    }

    pub fn verify_signature(&self) -> bool {
        crate::crypto::verify_signature(&self.voter, &self.signing_bytes(), &self.signature).is_ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub height: u64,
    pub round: u64,
    pub block: Block,
    pub signature: Vec<u8>,
    pub proposer: Vec<u8>,
}

impl Proposal {
    /// Canonical bytes the proposer signs.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PROPOSAL_DOMAIN.len() + 96);
        bytes.extend_from_slice(PROPOSAL_DOMAIN);
        bytes.extend_from_slice(&self.height.to_le_bytes());
        bytes.extend_from_slice(&self.round.to_le_bytes());
        bytes.extend_from_slice(&self.block.hash());
        bytes.extend_from_slice(&(self.proposer.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&self.proposer);
        bytes
    }

    pub fn verify_signature(&self) -> bool {
        crate::crypto::verify_signature(&self.proposer, &self.signing_bytes(), &self.signature)
            .is_ok()
    }
}

/// Proof that a block was finalized: the +2/3 precommits for it from one round.
/// Stored with every finalized block, so a syncing node can check finality
/// itself instead of trusting the peer (or a single proposer signature).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitCertificate {
    pub height: u64,
    pub round: u64,
    pub block_hash: Hash,
    pub precommits: Vec<Vote>,
}

impl CommitCertificate {
    /// Check the certificate against `validators` (the set at `height`).
    /// Returns the public keys of the validators whose precommits counted.
    pub fn verify(
        &self,
        block_hash: &Hash,
        height: u64,
        validators: &[ValidatorInfo],
    ) -> Result<Vec<Vec<u8>>, String> {
        if self.height != height || &self.block_hash != block_hash {
            return Err("Commit certificate is for a different block".into());
        }
        let total: u128 = validators
            .iter()
            .filter(|v| !v.slashed)
            .fold(0u128, |acc, v| acc.saturating_add(v.stake));
        let mut seen = HashSet::new();
        let mut signed_stake = 0u128;
        let mut signers = Vec::new();
        for vote in &self.precommits {
            if vote.height != height
                || vote.round != self.round
                || vote.step != Step::Precommit
                || vote.block_hash.as_ref() != Some(block_hash)
            {
                return Err("Commit certificate contains a mismatched vote".into());
            }
            let Some(validator) = validators
                .iter()
                .find(|v| v.public_key == vote.voter && !v.slashed)
            else {
                continue; // not in this validator set: carries no weight
            };
            if !seen.insert(vote.voter.clone()) {
                return Err("Commit certificate counts a validator twice".into());
            }
            if !vote.verify_signature() {
                return Err("Commit certificate has an invalid signature".into());
            }
            signed_stake = signed_stake.saturating_add(validator.stake);
            signers.push(vote.voter.clone());
        }
        if !is_quorum(signed_stake, total) {
            return Err(format!(
                "Commit certificate lacks a 2/3+ quorum ({} of {} stake)",
                signed_stake, total
            ));
        }
        Ok(signers)
    }
}

/// Two conflicting signed votes from one validator for the same
/// height/round/step: proof of equivocation, usable for slashing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoubleSignEvidence {
    pub first: Vote,
    pub second: Vote,
}

impl DoubleSignEvidence {
    /// Both votes are validly signed by the same voter, for the same
    /// height/round/step, and for different blocks.
    pub fn is_valid(&self) -> bool {
        let (a, b) = (&self.first, &self.second);
        a.voter == b.voter
            && a.height == b.height
            && a.round == b.round
            && a.step == b.step
            && a.block_hash != b.block_hash
            && a.verify_signature()
            && b.verify_signature()
    }
}

// Proposals carry a full block; boxing would only move the allocation.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusMessage {
    Vote(Vote),
    Proposal(Proposal),
}

/// Information about a participating validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    /// Ed25519 public key (32 bytes)
    pub public_key: Vec<u8>,
    /// Staked amount (higher stake = more voting power)
    pub stake: u128,
    /// Whether this validator has been slashed
    pub slashed: bool,
}
