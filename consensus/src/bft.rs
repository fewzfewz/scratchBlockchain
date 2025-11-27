use common::consensus_types::{Proposal, Step, Vote};
use common::crypto;
use crate::ValidatorInfo;
use common::types::{Block, Hash};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub propose_timeout_ms: u64,
    pub prevote_timeout_ms: u64,
    pub precommit_timeout_ms: u64,
    pub timeout_increase_factor: f64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            propose_timeout_ms: 3000,
            prevote_timeout_ms: 1000,
            precommit_timeout_ms: 1000,
            timeout_increase_factor: 1.1,
        }
    }
}

#[derive(Debug)]
pub enum BftEvent {
    BroadcastVote(Vote),
    BroadcastProposal(Proposal),
    FinalizeBlock(Block),
    NewRound(u64, u64), // height, round
    Timeout(Step),      // Timeout for a specific step
}

pub struct BftEngine {
    // Identity
    pub public_key: Vec<u8>,  // Changed from local_address to public_key
    // Signing key for this validator
    pub signing_key: common::crypto::SigningKey,
    
    // State
    pub height: u64,
    pub round: u64,
    pub step: Step,
    
    // Validators
    validators: HashMap<Vec<u8>, ValidatorInfo>,
    total_stake: u64,
    
    // Round state
    proposal: Option<Proposal>,
    votes: HashMap<(u64, Step), HashMap<Vec<u8>, Vote>>, // (round, step) -> voter -> vote
    #[allow(dead_code)]
    locked_block: Option<Block>,
    #[allow(dead_code)]
    valid_block: Option<Block>,
    
    // Timeout state
    timeout_config: TimeoutConfig,
    current_timeout: Option<Instant>,
    timeout_step: Option<Step>,
}

impl BftEngine {
    pub fn new(public_key: Vec<u8>, validators: Vec<ValidatorInfo>, start_height: u64, signing_key: crypto::SigningKey) -> Self {
        let mut val_map = HashMap::new();
        let mut total_stake = 0;
        for v in validators {
            total_stake += v.stake;
            val_map.insert(v.public_key.clone(), v);
        }

        Self {
            public_key,  // Changed from local_address
            signing_key,
            height: start_height,
            round: 0,
            step: Step::Propose,
            validators: val_map,
            total_stake,
            proposal: None,
            votes: HashMap::new(),
            locked_block: None,
            valid_block: None,
            timeout_config: TimeoutConfig::default(),
            current_timeout: None,
            timeout_step: None,
        }
    }

    fn serialize_proposal(&self, proposal: &Proposal) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&proposal.height.to_le_bytes());
        bytes.extend_from_slice(&proposal.round.to_le_bytes());
        bytes.extend_from_slice(&proposal.block.hash());
        bytes.extend_from_slice(&proposal.proposer);
        bytes
    }
    
    fn serialize_vote(&self, vote: &Vote) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&vote.height.to_le_bytes());
        bytes.extend_from_slice(&vote.round.to_le_bytes());
        bytes.push(vote.step as u8);
        if let Some(hash) = &vote.block_hash {
            bytes.extend_from_slice(hash);
        }
        bytes.extend_from_slice(&vote.voter);
        bytes
    }

    pub fn start_round(&mut self, round: u64) -> Vec<BftEvent> {
        self.round = round;
        self.step = Step::Propose;
        self.proposal = None;
        self.votes.clear(); // In a real implementation, keep past round votes for evidence
        
        // Start propose timeout
        self.start_timeout(Step::Propose);
        
        let events = vec![BftEvent::NewRound(self.height, self.round)];
        
        // Check if I am the proposer
        if self.is_proposer(self.height, self.round) {
            // Node should see NewRound and trigger block production if proposer
        }
        
        events
    }

    pub fn handle_proposal(&mut self, proposal: Proposal) -> Vec<BftEvent> {
        if proposal.height != self.height || proposal.round != self.round {
            return vec![];
        }
        
        if self.step != Step::Propose {
            return vec![];
        }

        // Verify proposer
        if !self.is_proposer(proposal.height, proposal.round) {
            // Wrong proposer
            return vec![];
        }
        
        // Verify signature
        let proposal_bytes = self.serialize_proposal(&proposal);
        if let Err(e) = crypto::verify_signature(&proposal.proposer, &proposal_bytes, &proposal.signature) {
            tracing::warn!("Invalid proposal signature from {:?}: {}", proposal.proposer, e);
            return vec![];
        }
        
        self.proposal = Some(proposal.clone());
        self.step = Step::Prevote;
        
        // Start prevote timeout
        self.start_timeout(Step::Prevote);
        
        // Broadcast Prevote for this block
        let mut vote = Vote {
            height: self.height,
            round: self.round,
            step: Step::Prevote,
            block_hash: Some(proposal.block.hash()),
            signature: vec![], 
            voter: self.public_key.clone(),  // Use public_key instead of local_address
        };
        
        // Sign the vote
        let vote_bytes = self.serialize_vote(&vote);
        vote.signature = self.signing_key.sign(&vote_bytes);
        
        // Record my own vote
        self.add_vote(vote.clone());
        
        vec![BftEvent::BroadcastVote(vote)]
    }

    pub fn handle_vote(&mut self, vote: Vote) -> Vec<BftEvent> {
        if vote.height != self.height || vote.round != self.round {
            return vec![];
        }
        
        // Verify voter
        if !self.validators.contains_key(&vote.voter) {
            return vec![];
        }
        
        // Verify signature
        let vote_bytes = self.serialize_vote(&vote);
        if let Err(e) = crypto::verify_signature(&vote.voter, &vote_bytes, &vote.signature) {
            tracing::warn!("Invalid vote signature from {:?}: {}", vote.voter, e);
            return vec![];
        }
        
        self.add_vote(vote.clone());
        
        self.check_quorum()
    }

    fn add_vote(&mut self, vote: Vote) {
        self.votes
            .entry((vote.round, vote.step))
            .or_default()
            .insert(vote.voter.clone(), vote);
    }

    fn check_quorum(&mut self) -> Vec<BftEvent> {
        let mut events = vec![];
        
        // Check Prevotes
        if self.step == Step::Prevote {
            if let Some(hash) = self.has_quorum(self.round, Step::Prevote) {
                // We have 2/3 prevotes for a block (or nil)
                self.step = Step::Precommit;
                
                // Start precommit timeout
                self.start_timeout(Step::Precommit);
                
                // Broadcast Precommit
                let mut vote = Vote {
                    height: self.height,
                    round: self.round,
                    step: Step::Precommit,
                    block_hash: hash,
                    signature: vec![], 
                    voter: self.public_key.clone(),  // Use public_key instead of local_address
                };
                
                // Sign the vote
                let vote_bytes = self.serialize_vote(&vote);
                vote.signature = self.signing_key.sign(&vote_bytes);
                
                self.add_vote(vote.clone());
                events.push(BftEvent::BroadcastVote(vote));
            }
        }
        
        // Check Precommits
        if self.step == Step::Precommit {
            if let Some(Some(hash)) = self.has_quorum(self.round, Step::Precommit) {
                // We have 2/3 precommits for a block -> Commit
                if let Some(proposal) = &self.proposal {
                    if proposal.block.hash() == hash {
                        self.step = Step::Commit;
                        events.push(BftEvent::FinalizeBlock(proposal.block.clone()));
                        
                        // Advance height
                        self.height += 1;
                        self.round = 0;
                        self.step = Step::Propose;
                        self.proposal = None;
                        self.votes.clear();
                        events.push(BftEvent::NewRound(self.height, 0));
                    }
                }
            }
        }
        
        events
    }

    // Returns Some(hash) if 2/3 quorum reached. hash is Option<Hash> (None for nil)
    fn has_quorum(&self, round: u64, step: Step) -> Option<Option<Hash>> {
        let votes = self.votes.get(&(round, step))?;
        
        let mut counts: HashMap<Option<Hash>, u64> = HashMap::new();
        for vote in votes.values() {
            let stake = self.validators.get(&vote.voter).map(|v| v.stake).unwrap_or(0);
            *counts.entry(vote.block_hash).or_default() += stake;
        }
        
        let threshold = (self.total_stake * 2) / 3;
        
        for (hash, stake) in counts {
            if stake > threshold {
                return Some(hash);
            }
        }
        
        None
    }

    pub fn is_proposer(&self, height: u64, round: u64) -> bool {
        // Simple round-robin proposer selection
        let mut sorted_validators: Vec<&Vec<u8>> = self.validators.keys().collect();
        sorted_validators.sort();
        
        let index = (height + round) as usize % sorted_validators.len();
        let proposer = sorted_validators[index];
        
        proposer == &self.public_key  // Use public_key instead of local_address
    }

    pub fn create_proposal(&mut self, block: Block) -> Vec<BftEvent> {
        if !self.is_proposer(self.height, self.round) {
            return vec![];
        }
        
        let proposal = Proposal {
            height: self.height,
            round: self.round,
            block: block.clone(),
            signature: vec![], // TODO: Sign
            proposer: self.public_key.clone(),  // Use public_key instead of local_address
        };
        
        self.proposal = Some(proposal.clone());
        self.step = Step::Prevote;
        
        // Vote for it
        let vote = Vote {
            height: self.height,
            round: self.round,
            step: Step::Prevote,
            block_hash: Some(block.hash()),
            signature: vec![], // TODO: Sign
            voter: self.public_key.clone(),  // Use public_key instead of local_address
        };
        self.add_vote(vote.clone());
        
        let mut events = vec![
            BftEvent::BroadcastProposal(proposal),
            BftEvent::BroadcastVote(vote)
        ];
        
        events.extend(self.check_quorum());
        
        events
    }
    
    // Timeout methods
    
    /// Calculate timeout duration for a given step and round
    fn calculate_timeout(&self, step: Step, round: u64) -> Duration {
        let base_timeout_ms = match step {
            Step::Propose => self.timeout_config.propose_timeout_ms,
            Step::Prevote => self.timeout_config.prevote_timeout_ms,
            Step::Precommit => self.timeout_config.precommit_timeout_ms,
            Step::Commit => return Duration::from_secs(0), // No timeout in commit
        };
        
        // Apply exponential backoff based on round
        let multiplier = self.timeout_config.timeout_increase_factor.powi(round as i32);
        let timeout_ms = (base_timeout_ms as f64 * multiplier) as u64;
        
        Duration::from_millis(timeout_ms)
    }
    
    /// Start a timeout for the current step
    fn start_timeout(&mut self, step: Step) {
        let timeout_duration = self.calculate_timeout(step, self.round);
        self.current_timeout = Some(Instant::now() + timeout_duration);
        self.timeout_step = Some(step);
    }
    
    /// Check if the current timeout has expired
    pub fn check_timeout(&self) -> Option<BftEvent> {
        if let (Some(timeout_instant), Some(timeout_step)) = (self.current_timeout, self.timeout_step) {
            if Instant::now() >= timeout_instant {
                return Some(BftEvent::Timeout(timeout_step));
            }
        }
        None
    }
    
    /// Handle timeout in Propose step
    pub fn handle_timeout_propose(&mut self) -> Vec<BftEvent> {
        tracing::info!("Propose timeout - voting nil");
        
        self.step = Step::Prevote;
    self.start_timeout(Step::Prevote);
    
    let mut vote = Vote {
        height: self.height,
        round: self.round,
        step: Step::Prevote,
        block_hash: None,  // Nil vote
        signature: vec![],
        voter: self.public_key.clone(),  // Use public_key instead of local_address
    };
    
    // Sign the vote
    let vote_bytes = self.serialize_vote(&vote);
    vote.signature = self.signing_key.sign(&vote_bytes);
    
    self.add_vote(vote.clone());
    vec![BftEvent::BroadcastVote(vote)]
}

/// Handle timeout in Prevote step
pub fn handle_timeout_prevote(&mut self) -> Vec<BftEvent> {
    tracing::info!("Prevote timeout - precommitting nil");
    
    self.step = Step::Precommit;
    self.start_timeout(Step::Precommit);
    
    let mut vote = Vote {
        height: self.height,
        round: self.round,
        step: Step::Precommit,
        block_hash: None,  // Nil vote
        signature: vec![],
        voter: self.public_key.clone(),  // Use public_key instead of local_address
    };
    
    // Sign the vote
    let vote_bytes = self.serialize_vote(&vote);
    vote.signature = self.signing_key.sign(&vote_bytes);
    
    self.add_vote(vote.clone());
    vec![BftEvent::BroadcastVote(vote)]
}
    
    /// Handle timeout in Precommit step
    pub fn handle_timeout_precommit(&mut self) -> Vec<BftEvent> {
        tracing::info!("Precommit timeout - moving to next round");
        
        let new_round = self.round + 1;
        self.start_round(new_round)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::types::Header;

    #[test]
    fn test_bft_flow() {
        let signing_key = crypto::SigningKey::generate();
        let public_key = signing_key.public_key();
        
        let validator = ValidatorInfo {
            public_key: public_key.clone(),
            stake: 100,
            slashed: false,
        };
        let mut engine = BftEngine::new(public_key, vec![validator], 1, signing_key);
        
        // Start round
        let events = engine.start_round(0);
        assert_eq!(events.len(), 1); // NewRound
        if let BftEvent::NewRound(h, r) = events[0] {
            assert_eq!(h, 1);
            assert_eq!(r, 0);
        } else {
            panic!("Expected NewRound");
        }
        
        // Create proposal
        let block = Block {
            header: Header {
                parent_hash: [0u8; 32],
                slot: 1,
                state_root: [0u8; 32],
                extrinsics_root: [0u8; 32],
                epoch: 0,
                validator_set_id: 0,
                signature: vec![],
                gas_used: 0,
                base_fee: 0,
            },
            extrinsics: vec![],
        };
        let events = engine.create_proposal(block.clone());
        
        // With 1 validator, we should go straight to finality
        // Events: Proposal, Prevote, Precommit, Finalize, NewRound
        
        let has_proposal = events.iter().any(|e| matches!(e, BftEvent::BroadcastProposal(_)));
        let has_prevote = events.iter().any(|e| matches!(e, BftEvent::BroadcastVote(v) if v.step == Step::Prevote));
        let has_precommit = events.iter().any(|e| matches!(e, BftEvent::BroadcastVote(v) if v.step == Step::Precommit));
        let has_finalize = events.iter().any(|e| matches!(e, BftEvent::FinalizeBlock(_)));
        let has_new_round = events.iter().any(|e| matches!(e, BftEvent::NewRound(_, _)));
        
        assert!(has_proposal, "Missing Proposal");
        assert!(has_prevote, "Missing Prevote");
        assert!(has_precommit, "Missing Precommit");
        assert!(has_finalize, "Missing Finalize");
        assert!(has_new_round, "Missing NewRound");
        
        assert_eq!(engine.height, 2);
        assert_eq!(engine.round, 0);
        assert_eq!(engine.step, Step::Propose);
    }
}
