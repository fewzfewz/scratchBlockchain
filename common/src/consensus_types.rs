use crate::types::{Block, Hash};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Step {
    Propose,
    Prevote,
    Precommit,
    Commit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub height: u64,
    pub round: u64,
    pub step: Step,
    pub block_hash: Option<Hash>, // None for nil
    pub signature: Vec<u8>,
    pub voter: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub height: u64,
    pub round: u64,
    pub block: Block,
    pub signature: Vec<u8>,
    pub proposer: Vec<u8>,
}

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
    pub stake: u64,
    /// Whether this validator has been slashed
    pub slashed: bool,
}
