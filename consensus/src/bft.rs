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
    NewRound(u64, u64),
    Timeout(Step),
}

pub struct BftEngine {
    pub public_key: Vec<u8>,
    pub signing_key: common::crypto::SigningKey,

    pub height: u64,
    pub round: u64,
    pub step: Step,

    validators: HashMap<Vec<u8>, ValidatorInfo>,
    total_stake: u64,

    proposal: Option<Proposal>,

    /// The most recent proposal this node created itself (for re-broadcasting
    /// so peers that enter a height/round late still receive the proposal).
    own_proposal: Option<Proposal>,

    votes: HashMap<(u64, Step), HashMap<Vec<u8>, Vote>>,
    max_votes_per_round: usize,  // FIX: Added memory protection

    locked_block: Option<(Block, u64)>,
    valid_block: Option<(Block, u64)>,

    timeout_config: TimeoutConfig,
    current_timeout: Option<Instant>,
    timeout_step: Option<Step>,
}

impl BftEngine {
    pub fn new(
        public_key: Vec<u8>,
        validators: Vec<ValidatorInfo>,
        start_height: u64,
        signing_key: crypto::SigningKey,
    ) -> Self {
        let mut val_map = HashMap::new();
        let mut total_stake = 0;
        for v in validators {
            total_stake += v.stake;
            val_map.insert(v.public_key.clone(), v);
        }

        Self {
            public_key,
            signing_key,
            height: start_height,
            round: 0,
            step: Step::Propose,
            validators: val_map,
            total_stake,
            proposal: None,
            own_proposal: None,
            votes: HashMap::new(),
            max_votes_per_round: 1000,  // FIX: Prevent memory DoS
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

    pub fn serialize_vote(&self, vote: &Vote) -> Vec<u8> {
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
        self.own_proposal = None;
        self.start_timeout(Step::Propose);
        vec![BftEvent::NewRound(self.height, self.round)]
    }

    pub fn handle_proposal(&mut self, proposal: Proposal) -> Vec<BftEvent> {
        if proposal.height != self.height {
            return vec![];
        }

        let mut events = Vec::new();

        // Round-sync: accept a proposal for a higher round of this height by
        // first jumping to that round (see handle_vote).
        if proposal.round > self.round {
            tracing::info!(
                "Round-sync: jumping to round {} (was {}) for proposal",
                proposal.round, self.round
            );
            events.extend(self.start_round(proposal.round));
        }

        if proposal.round != self.round {
            return events;
        }
        if self.step != Step::Propose {
            return events;
        }

        let expected_proposer = self.select_proposer(self.height, self.round);
        if proposal.proposer != expected_proposer {
            tracing::warn!("Proposal from wrong proposer");
            return events;
        }

        let proposal_bytes = self.serialize_proposal(&proposal);
        if let Err(e) =
            crypto::verify_signature(&proposal.proposer, &proposal_bytes, &proposal.signature)
        {
            tracing::warn!("Invalid proposal signature: {}", e);
            return events;
        }

        self.proposal = Some(proposal.clone());
        self.step = Step::Prevote;
        self.start_timeout(Step::Prevote);

        let vote_hash = match &self.locked_block {
            Some((locked, _lock_round)) if locked.hash() != proposal.block.hash() => {
                tracing::info!("Locked on different block — prevoting nil");
                None
            }
            _ => Some(proposal.block.hash()),
        };

        let mut vote = Vote {
            height: self.height,
            round: self.round,
            step: Step::Prevote,
            block_hash: vote_hash,
            signature: vec![],
            voter: self.public_key.clone(),
        };
        let vote_bytes = self.serialize_vote(&vote);
        vote.signature = self.signing_key.sign(&vote_bytes);

        self.add_vote(vote.clone());
        events.push(BftEvent::BroadcastVote(vote));
        events
    }

    pub fn create_proposal(&mut self, block: Block) -> Vec<BftEvent> {
        if !self.is_proposer(self.height, self.round) {
            return vec![];
        }

        let propose_block = match &self.valid_block {
            Some((vb, _)) => vb.clone(),
            None => block,
        };

        let mut proposal = Proposal {
            height: self.height,
            round: self.round,
            block: propose_block.clone(),
            signature: vec![],
            proposer: self.public_key.clone(),
        };

        let proposal_bytes = self.serialize_proposal(&proposal);
        proposal.signature = self.signing_key.sign(&proposal_bytes);

        self.proposal = Some(proposal.clone());
        self.own_proposal = Some(proposal.clone());
        self.step = Step::Prevote;

        let vote_hash = match &self.locked_block {
            Some((locked, _)) if locked.hash() != propose_block.hash() => None,
            _ => Some(propose_block.hash()),
        };

        let mut vote = Vote {
            height: self.height,
            round: self.round,
            step: Step::Prevote,
            block_hash: vote_hash,
            signature: vec![],
            voter: self.public_key.clone(),
        };
        let vote_bytes = self.serialize_vote(&vote);
        vote.signature = self.signing_key.sign(&vote_bytes);

        self.add_vote(vote.clone());

        let mut events = vec![
            BftEvent::BroadcastProposal(proposal),
            BftEvent::BroadcastVote(vote),
        ];
        events.extend(self.check_quorum());
        events
    }

    pub fn handle_vote(&mut self, vote: Vote) -> Vec<BftEvent> {
        if vote.height != self.height {
            return vec![];
        }
        if !self.validators.contains_key(&vote.voter) {
            return vec![];
        }

        let mut events = Vec::new();

        // Round-sync: if a peer is already voting at a higher round of this
        // height, jump to that round so we stay aligned and can still vote.
        // This prevents the network from drifting apart on local timeouts,
        // which previously stalled consensus for many rounds per height.
        if vote.round > self.round {
            tracing::info!(
                "Round-sync: jumping to round {} (was {})",
                vote.round, self.round
            );
            events.extend(self.start_round(vote.round));
        }

        if vote.round != self.round {
            return events;
        }

        let vote_bytes = self.serialize_vote(&vote);
        if let Err(e) = crypto::verify_signature(&vote.voter, &vote_bytes, &vote.signature) {
            tracing::warn!("Invalid vote signature: {}", e);
            return events;
        }

        self.add_vote(vote.clone());
        events.extend(self.check_quorum());
        events
    }

    pub fn add_vote_public(&mut self, vote: Vote) {
        self.add_vote(vote)
    }

    pub fn check_quorum_public(&mut self) -> Vec<BftEvent> {
        self.check_quorum()
    }

    fn add_vote(&mut self, vote: Vote) {
        let round_votes = self.votes
            .entry((vote.round, vote.step))
            .or_default();
        
        // FIX: Prevent memory DoS attack
        if round_votes.len() >= self.max_votes_per_round {
            tracing::warn!("Too many votes for round {}, step {:?}", vote.round, vote.step);
            return;
        }
        
        round_votes.insert(vote.voter.clone(), vote);
    }

    fn check_quorum(&mut self) -> Vec<BftEvent> {
        let mut events = vec![];

        if self.step == Step::Prevote {
            if let Some(hash) = self.has_quorum(self.round, Step::Prevote) {
                self.step = Step::Precommit;
                self.start_timeout(Step::Precommit);

                // FIX: Only update locks if round >= current lock_round
                if let Some(block_hash) = &hash {
                    if let Some(proposal) = &self.proposal {
                        if &proposal.block.hash() == block_hash {
                            let current_lock_round = self.locked_block.as_ref().map(|(_, r)| *r).unwrap_or(0);
                            if self.round >= current_lock_round {
                                let block = proposal.block.clone();
                                let round = self.round;
                                self.valid_block = Some((block.clone(), round));
                                self.locked_block = Some((block, round));
                            }
                        }
                    }
                }

                let mut vote = Vote {
                    height: self.height,
                    round: self.round,
                    step: Step::Precommit,
                    block_hash: hash,
                    signature: vec![],
                    voter: self.public_key.clone(),
                };
                let vote_bytes = self.serialize_vote(&vote);
                vote.signature = self.signing_key.sign(&vote_bytes);

                self.add_vote(vote.clone());
                events.push(BftEvent::BroadcastVote(vote));
            }
        }

        if self.step == Step::Precommit {
            if let Some(Some(hash)) = self.has_quorum(self.round, Step::Precommit) {
                if let Some(proposal) = &self.proposal {
                    if proposal.block.hash() == hash {
                        self.step = Step::Commit;
                        events.push(BftEvent::FinalizeBlock(proposal.block.clone()));

                        self.height += 1;
                        self.round = 0;
                        self.step = Step::Propose;
                        self.proposal = None;
                        self.own_proposal = None;
                        self.locked_block = None;
                        self.valid_block = None;
                        self.votes.clear();
                        events.push(BftEvent::NewRound(self.height, 0));
                    }
                }
            }
        }

        events
    }

    pub fn has_quorum_public(&self, round: u64, step: Step) -> Option<Option<Hash>> {
        self.has_quorum(round, step)
    }

    fn has_quorum(&self, round: u64, step: Step) -> Option<Option<Hash>> {
        let votes = self.votes.get(&(round, step))?;

        let mut counts: HashMap<Option<Hash>, u64> = HashMap::new();
        for vote in votes.values() {
            let stake = self
                .validators
                .get(&vote.voter)
                .map(|v| v.stake)
                .unwrap_or(0);
            *counts.entry(vote.block_hash).or_default() += stake;
        }

        // Standard 2/3+ quorum. Using >= (rather than >) means a 3-validator
        // set with equal stakes can finalize with any 2 of 3 — otherwise all 3
        // would be required, making the network stall whenever one node lags.
        let threshold = (self.total_stake * 2) / 3;
        for (hash, stake) in counts {
            if stake >= threshold {
                return Some(hash);
            }
        }
        None
    }

    fn select_proposer(&self, height: u64, round: u64) -> Vec<u8> {
        // FIX: Handle empty validator set gracefully
        if self.validators.is_empty() {
            tracing::error!("No validators configured!");
            return self.public_key.clone();
        }
        
        let mut sorted_validators: Vec<&Vec<u8>> = self.validators.keys().collect();
        sorted_validators.sort();

        let mut seed_input = Vec::new();
        seed_input.extend_from_slice(&height.to_le_bytes());
        seed_input.extend_from_slice(&round.to_le_bytes());
        let seed = crypto::hash(&seed_input);

        let index = u64::from_le_bytes(seed[..8].try_into().unwrap_or([0u8; 8])) as usize
            % sorted_validators.len();

        sorted_validators[index].clone()
    }

    pub fn is_proposer(&self, height: u64, round: u64) -> bool {
        self.select_proposer(height, round) == self.public_key
    }

    fn calculate_timeout(&self, step: Step, round: u64) -> Duration {
        let base_timeout_ms = match step {
            Step::Propose => self.timeout_config.propose_timeout_ms,
            Step::Prevote => self.timeout_config.prevote_timeout_ms,
            Step::Precommit => self.timeout_config.precommit_timeout_ms,
            Step::Commit => return Duration::from_secs(0),
        };
        let multiplier = self.timeout_config.timeout_increase_factor.powi(round as i32);
        let timeout_ms = (base_timeout_ms as f64 * multiplier) as u64;
        Duration::from_millis(timeout_ms)
    }

    fn start_timeout(&mut self, step: Step) {
        let timeout_duration = self.calculate_timeout(step, self.round);
        self.current_timeout = Some(Instant::now() + timeout_duration);
        self.timeout_step = Some(step);
    }

    pub fn check_timeout(&self) -> Option<BftEvent> {
        if let (Some(timeout_instant), Some(timeout_step)) =
            (self.current_timeout, self.timeout_step)
        {
            if Instant::now() >= timeout_instant {
                return Some(BftEvent::Timeout(timeout_step));
            }
        }
        None
    }

    pub fn handle_timeout_propose(&mut self) -> Vec<BftEvent> {
        tracing::info!("Propose timeout at height={} round={} — voting nil", self.height, self.round);
        self.step = Step::Prevote;
        self.start_timeout(Step::Prevote);

        let mut vote = Vote {
            height: self.height,
            round: self.round,
            step: Step::Prevote,
            block_hash: None,
            signature: vec![],
            voter: self.public_key.clone(),
        };
        let vote_bytes = self.serialize_vote(&vote);
        vote.signature = self.signing_key.sign(&vote_bytes);
        self.add_vote(vote.clone());
        vec![BftEvent::BroadcastVote(vote)]
    }

    pub fn handle_timeout_prevote(&mut self) -> Vec<BftEvent> {
        tracing::info!("Prevote timeout at height={} round={} — precommitting nil", self.height, self.round);
        self.step = Step::Precommit;
        self.start_timeout(Step::Precommit);

        let mut vote = Vote {
            height: self.height,
            round: self.round,
            step: Step::Precommit,
            block_hash: None,
            signature: vec![],
            voter: self.public_key.clone(),
        };
        let vote_bytes = self.serialize_vote(&vote);
        vote.signature = self.signing_key.sign(&vote_bytes);
        self.add_vote(vote.clone());
        vec![BftEvent::BroadcastVote(vote)]
    }

    pub fn handle_timeout_precommit(&mut self) -> Vec<BftEvent> {
        tracing::info!("Precommit timeout at height={} round={} — advancing round", self.height, self.round);
        let new_round = self.round + 1;
        self.start_round(new_round)
    }

    pub fn collect_past_round_votes(&self) -> Vec<&Vote> {
        self.votes
            .iter()
            .filter(|((round, _step), _)| *round < self.round)
            .flat_map(|(_, voter_map)| voter_map.values())
            .collect()
    }

    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }

    /// Hot-reload validator set from on-chain state (call at height boundaries).
    pub fn update_validator_set(&mut self, validators: Vec<ValidatorInfo>) {
        self.validators.clear();
        self.total_stake = 0;
        for v in validators {
            if !v.slashed {
                self.total_stake = self.total_stake.saturating_add(v.stake);
                self.validators.insert(v.public_key.clone(), v);
            }
        }
    }

    /// Reset consensus state to begin voting at a new height.
    ///
    /// Used after a node has fallen behind (e.g. it locked a losing block and
    /// could no longer reach quorum) and has re-synced its chain from peers.
    /// Clears any stale lock/proposal/vote state so the node rejoins consensus
    /// on the canonical chain at the given height.
    pub fn reset_to_height(&mut self, new_height: u64) -> Vec<BftEvent> {
        self.height = new_height.max(1);
        self.round = 0;
        self.step = Step::Propose;
        self.proposal = None;
        self.own_proposal = None;
        self.locked_block = None;
        self.valid_block = None;
        self.votes.clear();
        self.current_timeout = None;
        self.timeout_step = None;
        self.start_round(0)
    }

    /// Re-broadcast our current proposal if we are still at the height/round it
    /// was created for. This lets peers that entered the height/round late (and
    /// therefore missed the original broadcast) still receive it and vote.
    pub fn re_propose(&mut self) -> Vec<BftEvent> {
        match &self.own_proposal {
            Some(proposal)
                if proposal.height == self.height
                    && proposal.round == self.round
                    && self.step != Step::Commit =>
            {
                vec![BftEvent::BroadcastProposal(proposal.clone())]
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::types::Header;

    fn make_block(slot: u64) -> Block {
        Block {
            header: Header {
                parent_hash: [0u8; 32],
                slot,
                state_root: [0u8; 32],
                extrinsics_root: [0u8; 32],
                epoch: 0,
                validator_set_id: 0,
                signature: vec![],
                gas_used: 0,
                base_fee: 0,
            },
            extrinsics: vec![],
        }
    }

    #[test]
    fn test_bft_single_validator_full_flow() {
        let signing_key = crypto::SigningKey::generate();
        let public_key = signing_key.public_key();
        let validator = ValidatorInfo {
            public_key: public_key.clone(),
            stake: 100,
            slashed: false,
        };
        let mut engine = BftEngine::new(public_key, vec![validator], 1, signing_key);

        let events = engine.start_round(0);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], BftEvent::NewRound(1, 0)));

        let block = make_block(1);
        let events = engine.create_proposal(block.clone());

        assert!(events.iter().any(|e| matches!(e, BftEvent::BroadcastProposal(_))));
        assert!(events.iter().any(|e| matches!(e, BftEvent::BroadcastVote(v) if v.step == Step::Prevote)));
        assert!(events.iter().any(|e| matches!(e, BftEvent::BroadcastVote(v) if v.step == Step::Precommit)));
        assert!(events.iter().any(|e| matches!(e, BftEvent::FinalizeBlock(_))));
        assert!(events.iter().any(|e| matches!(e, BftEvent::NewRound(_, _))));

        assert_eq!(engine.height, 2);
        assert_eq!(engine.round, 0);
        assert_eq!(engine.step, Step::Propose);
    }

    #[test]
    fn test_proposal_must_be_signed() {
        let signing_key = crypto::SigningKey::generate();
        let public_key = signing_key.public_key();
        let validator = ValidatorInfo {
            public_key: public_key.clone(),
            stake: 100,
            slashed: false,
        };
        let mut engine = BftEngine::new(public_key.clone(), vec![validator], 1, signing_key);
        engine.start_round(0);

        let bad_proposal = Proposal {
            height: 1,
            round: 0,
            block: make_block(1),
            signature: vec![],
            proposer: public_key,
        };
        let events = engine.handle_proposal(bad_proposal);
        assert!(events.is_empty(), "Unsigned proposal should be rejected");
    }

    #[test]
    fn test_timeout_propose_votes_nil() {
        let signing_key = crypto::SigningKey::generate();
        let public_key = signing_key.public_key();
        let validator = ValidatorInfo {
            public_key: public_key.clone(),
            stake: 100,
            slashed: false,
        };
        let mut engine = BftEngine::new(public_key, vec![validator], 1, signing_key);
        engine.start_round(0);

        let events = engine.handle_timeout_propose();
        assert_eq!(events.len(), 1);
        if let BftEvent::BroadcastVote(v) = &events[0] {
            assert_eq!(v.step, Step::Prevote);
            assert!(v.block_hash.is_none());
        } else {
            panic!("Expected BroadcastVote");
        }
    }

    #[test]
    fn test_cross_validator_vote_quorum_flow() {
        let sk1 = crypto::SigningKey::generate();
        let pk1 = sk1.public_key();
        let sk2 = crypto::SigningKey::generate();
        let pk2 = sk2.public_key();

        let v1 = ValidatorInfo { public_key: pk1.clone(), stake: 100, slashed: false };
        let v2 = ValidatorInfo { public_key: pk2.clone(), stake: 100, slashed: false };

        // Determine who is proposer for (height=1, round=0) given these 2 validators
        let proposer_pk = {
            let mut sorted: Vec<&Vec<u8>> = vec![&pk1, &pk2];
            sorted.sort();
            let seed_input = {
                let mut b = Vec::new();
                b.extend_from_slice(&1u64.to_le_bytes());
                b.extend_from_slice(&0u64.to_le_bytes());
                b
            };
            let seed = crypto::hash(&seed_input);
            let idx = u64::from_le_bytes(seed[..8].try_into().unwrap()) as usize % sorted.len();
            sorted[idx].clone()
        };

        let (proposer_sk, proposer_pk, nonproposer_sk, nonproposer_pk) =
            if proposer_pk == pk1 {
                (sk1, pk1, sk2, pk2)
            } else {
                (sk2, pk2, sk1, pk1)
            };

        let block = make_block(1);
        let block_hash = block.hash();

        // Proposer creates a signed proposal
        let p_bytes = {
            let mut b = Vec::new();
            b.extend_from_slice(&1u64.to_le_bytes());
            b.extend_from_slice(&0u64.to_le_bytes());
            b.extend_from_slice(&block_hash);
            b.extend_from_slice(&proposer_pk);
            b
        };
        let proposal = Proposal {
            height: 1, round: 0,
            block,
            signature: proposer_sk.sign(&p_bytes),
            proposer: proposer_pk.clone(),
        };

        // Non-proposer creates a signed prevote on the same block
        let vote = {
            let mut v = Vote {
                height: 1, round: 0,
                step: Step::Prevote,
                block_hash: Some(block_hash),
                signature: vec![],
                voter: nonproposer_pk.clone(),
            };
            let v_bytes = {
                let mut b = Vec::new();
                b.extend_from_slice(&v.height.to_le_bytes());
                b.extend_from_slice(&v.round.to_le_bytes());
                b.push(v.step as u8);
                b.extend_from_slice(&block_hash);
                b.extend_from_slice(&nonproposer_pk);
                b
            };
            v.signature = nonproposer_sk.sign(&v_bytes);
            v
        };

        // ---- TEST 1: manual add_vote + check_quorum ----
        {
            let proposer_sk_copy =
                crypto::SigningKey::from_bytes(&proposer_sk.to_bytes()).unwrap();
            let mut engine = BftEngine::new(
                proposer_pk.clone(),
                vec![v1.clone(), v2.clone()],
                1,
                proposer_sk_copy,
            );
            engine.start_round(0);

            // Receive proposal (moves engine to Prevote, adds proposer's own vote)
            let prop_events = engine.handle_proposal(proposal.clone());
            assert!(!prop_events.is_empty(), "proposal from correct proposer should be accepted");

            // Now add the non-proposer's vote
            engine.add_vote_public(vote.clone());
            let after = engine.votes.get(&(0, Step::Prevote)).map(|m| m.len()).unwrap_or(0);
            assert_eq!(after, 2, "both validator votes should be in the map");

            let quorum = engine.has_quorum_public(0, Step::Prevote);
            let events = engine.check_quorum_public();
            assert_eq!(events.len(), 1, "quorum should produce BroadcastVote");

            assert!(quorum.is_some(), "2-of-2 validators should reach quorum");
        }

        // ---- TEST 2: handle_vote directly (full flow) ----
        {
            let proposer_sk_copy =
                crypto::SigningKey::from_bytes(&proposer_sk.to_bytes()).unwrap();
            let mut engine = BftEngine::new(
                proposer_pk,
                vec![v1, v2],
                1,
                proposer_sk_copy,
            );
            engine.start_round(0);

            // Receive proposal
            engine.handle_proposal(proposal);

            // Receive non-proposer's vote
            let events = engine.handle_vote(vote);
            assert_eq!(events.len(), 1,
                "handle_vote on second prevote should reach quorum and return events");
            assert!(events.iter().any(|e| matches!(e, BftEvent::BroadcastVote(v) if v.step == Step::Precommit)),
                    "should transition to Precommit and broadcast");
        }
    }

    #[test]
    fn test_handle_vote_jumps_to_higher_round() {
        let sk1 = crypto::SigningKey::generate();
        let pk1 = sk1.public_key();
        let sk2 = crypto::SigningKey::generate();
        let pk2 = sk2.public_key();
        let sk3 = crypto::SigningKey::generate();
        let pk3 = sk3.public_key();

        let validators = vec![
            ValidatorInfo { public_key: pk1.clone(), stake: 100, slashed: false },
            ValidatorInfo { public_key: pk2.clone(), stake: 100, slashed: false },
            ValidatorInfo { public_key: pk3.clone(), stake: 100, slashed: false },
        ];

        let mut engine = BftEngine::new(pk1, validators, 1, sk1.clone());
        engine.start_round(0);

        // A peer (pk2) prevotes at round 3 of the same height.
        let block = make_block(1);
        let block_hash = block.hash();
        let mut vote = Vote {
            height: 1,
            round: 3,
            step: Step::Prevote,
            block_hash: Some(block_hash),
            signature: vec![],
            voter: pk2,
        };
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&vote.height.to_le_bytes());
        bytes.extend_from_slice(&vote.round.to_le_bytes());
        bytes.push(vote.step as u8);
        bytes.extend_from_slice(&block_hash);
        bytes.extend_from_slice(&vote.voter);
        vote.signature = sk2.sign(&bytes);

        let events = engine.handle_vote(vote);
        assert_eq!(engine.round, 3, "engine should round-sync to the vote's round");
        assert!(
            events.iter().any(|e| matches!(e, BftEvent::NewRound(1, 3))),
            "round-sync should emit a NewRound event"
        );

        // A stale vote from a lower round is now ignored.
        assert!(engine.handle_vote(Vote {
            height: 1,
            round: 1,
            step: Step::Prevote,
            block_hash: Some(block_hash),
            signature: vec![],
            voter: pk3,
        }).is_empty());
    }

    #[test]
    fn test_re_propose_rebroadcasts_current_proposal() {
        let sk1 = crypto::SigningKey::generate();
        let pk1 = sk1.public_key();
        let sk2 = crypto::SigningKey::generate();
        let pk2 = sk2.public_key();
        let sk3 = crypto::SigningKey::generate();
        let pk3 = sk3.public_key();

        let validators = vec![
            ValidatorInfo { public_key: pk1.clone(), stake: 100, slashed: false },
            ValidatorInfo { public_key: pk2.clone(), stake: 100, slashed: false },
            ValidatorInfo { public_key: pk3.clone(), stake: 100, slashed: false },
        ];

        // Build the engine with whoever is the proposer at height 1, round 0.
        let mut sorted: Vec<&Vec<u8>> = vec![&pk1, &pk2, &pk3];
        sorted.sort();
        let mut seed_input = Vec::new();
        seed_input.extend_from_slice(&1u64.to_le_bytes());
        seed_input.extend_from_slice(&0u64.to_le_bytes());
        let seed = crypto::hash(&seed_input);
        let index = u64::from_le_bytes(seed[..8].try_into().unwrap()) as usize % 3;
        let proposer_pk = sorted[index].clone();
        let proposer_key = if proposer_pk == pk1 {
            sk1
        } else if proposer_pk == pk2 {
            sk2
        } else {
            sk3
        };

        let mut engine = BftEngine::new(proposer_pk, validators, 1, proposer_key);
        engine.start_round(0);
        engine.create_proposal(make_block(0));

        let events = engine.re_propose();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], BftEvent::BroadcastProposal(_)),
            "Proposer should re-broadcast its proposal");

        // After moving to a new round the old proposal must not be re-broadcast.
        engine.start_round(1);
        assert!(engine.re_propose().is_empty());
    }

    #[test]
    fn test_quorum_reached_with_two_of_three_equal_stakes() {
        let sk1 = crypto::SigningKey::generate();
        let pk1 = sk1.public_key();
        let sk2 = crypto::SigningKey::generate();
        let pk2 = sk2.public_key();
        let sk3 = crypto::SigningKey::generate();
        let pk3 = sk3.public_key();

        let validators = vec![
            ValidatorInfo { public_key: pk1.clone(), stake: 100, slashed: false },
            ValidatorInfo { public_key: pk2.clone(), stake: 100, slashed: false },
            ValidatorInfo { public_key: pk3.clone(), stake: 100, slashed: false },
        ];

        let mut engine = BftEngine::new(pk1.clone(), validators, 1, sk1.clone());
        let block = make_block(1);
        let block_hash = block.hash();

        // Two of the three validators prevote the same block.
        for (voter, key) in [(pk1, &sk1), (pk2, &sk2)] {
            let mut vote = Vote {
                height: 1,
                round: 0,
                step: Step::Prevote,
                block_hash: Some(block_hash),
                signature: vec![],
                voter,
            };
            let bytes = {
                let mut b = Vec::new();
                b.extend_from_slice(&vote.height.to_le_bytes());
                b.extend_from_slice(&vote.round.to_le_bytes());
                b.push(vote.step as u8);
                b.extend_from_slice(&block_hash);
                b.extend_from_slice(&vote.voter);
                b
            };
            vote.signature = key.sign(&bytes);
            engine.add_vote_public(vote);
        }

        let quorum = engine.has_quorum_public(0, Step::Prevote);
        assert_eq!(quorum, Some(Some(block_hash)),
            "2 of 3 equal-stake validators must reach the 2/3 quorum");
        let _ = sk3;
    }

    #[test]
    fn test_reset_to_height_clears_lock_and_votes() {
        let signing_key = crypto::SigningKey::generate();
        let public_key = signing_key.public_key();
        let validator = ValidatorInfo {
            public_key: public_key.clone(),
            stake: 100,
            slashed: false,
        };
        let mut engine = BftEngine::new(public_key, vec![validator], 1, signing_key);
        engine.start_round(0);

        // Simulate a stuck validator: lock a block and accumulate votes.
        let block = make_block(5);
        engine.locked_block = Some((block, 0));
        engine.handle_timeout_propose();
        engine.handle_timeout_prevote();

        let events = engine.reset_to_height(42);
        assert_eq!(engine.height, 42);
        assert_eq!(engine.round, 0);
        assert_eq!(engine.step, Step::Propose);
        assert!(engine.locked_block.is_none());
        assert!(engine.valid_block.is_none());
        assert!(engine.proposal.is_none());
        assert!(engine.votes.is_empty());
        assert!(events.iter().any(|e| matches!(e, BftEvent::NewRound(42, 0))));
    }

    #[test]
    fn test_votes_retained_across_rounds() {
        let signing_key = crypto::SigningKey::generate();
        let public_key = signing_key.public_key();
        let validator = ValidatorInfo {
            public_key: public_key.clone(),
            stake: 100,
            slashed: false,
        };
        let mut engine = BftEngine::new(public_key, vec![validator], 1, signing_key);
        engine.start_round(0);
        engine.handle_timeout_propose();
        engine.start_round(1);
        let past_votes = engine.collect_past_round_votes();
        assert!(!past_votes.is_empty(), "Past round votes should be retained");
    }
}