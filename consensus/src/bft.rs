use crate::ValidatorInfo;
use common::consensus_types::{
    is_one_third_plus, is_quorum, CommitCertificate, DoubleSignEvidence, Proposal, Step, Vote,
};
use common::crypto;
use common::types::{Block, Hash};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Votes for rounds further ahead than this are dropped, so a validator cannot
/// make us store votes for an unbounded number of future rounds.
const MAX_FUTURE_ROUNDS: u64 = 100;

#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub propose_timeout_ms: u64,
    pub prevote_timeout_ms: u64,
    pub precommit_timeout_ms: u64,
    /// Added per round (Tendermint's linear increase). An exponential
    /// increase made a node recovering from a long stall wait minutes per
    /// step.
    pub timeout_delta_ms: u64,
    pub max_timeout_ms: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            propose_timeout_ms: 3000,
            prevote_timeout_ms: 1000,
            precommit_timeout_ms: 1000,
            timeout_delta_ms: 500,
            max_timeout_ms: 30_000,
        }
    }
}

#[derive(Debug)]
pub enum BftEvent {
    BroadcastVote(Vote),
    BroadcastProposal(Proposal),
    /// Block finalized by BFT quorum, plus the proposer's ed25519 public key.
    /// `block.commit` carries the +2/3 precommits (commit certificate).
    FinalizeBlock(Block, Vec<u8>),
    NewRound(u64, u64),
    Timeout(Step),
    /// A peer is voting/proposing at a height higher than ours: we are behind
    /// and should fetch the contiguous chain from a peer before continuing.
    BehindSync(u64),
    /// We have a 2/3 precommit quorum for a block we don't hold locally; the
    /// node must fetch that block (and the contiguous chain) from a peer.
    NeedBlock(Hash, u64),
    /// A validator signed two different votes for the same height/round/step.
    Equivocation(DoubleSignEvidence),
}

/// The part of the engine that must survive a restart for safety: the lock
/// (Tendermint locking rule) and the last height/round/step we signed at
/// (so we never sign two different messages for the same slot).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BftSafetyState {
    pub height: u64,
    pub round: u64,
    pub locked: Option<(Block, u64)>,
    pub valid: Option<(Block, u64)>,
    pub last_signed: Option<(u64, u64, Step)>,
}

pub struct BftEngine {
    pub public_key: Vec<u8>,
    pub signing_key: common::crypto::SigningKey,

    pub height: u64,
    pub round: u64,
    pub step: Step,

    validators: HashMap<Vec<u8>, ValidatorInfo>,
    total_stake: u128,

    proposal: Option<Proposal>,

    /// The most recent proposal this node created itself (for re-broadcasting
    /// so peers that enter a height/round late still receive the proposal).
    own_proposal: Option<Proposal>,

    votes: HashMap<(u64, Step), HashMap<Vec<u8>, Vote>>,
    max_votes_per_round: usize,

    /// Valid proposals seen for the current height, keyed by block hash. Lets a
    /// node finalize a block that reached 2/3 precommit quorum even when it is
    /// not the block the node last proposed itself.
    proposals_by_hash: HashMap<Hash, Proposal>,

    /// Verified proposals for a future round of this height, replayed when we
    /// enter that round.
    future_proposals: HashMap<u64, Proposal>,

    locked_block: Option<(Block, u64)>,
    valid_block: Option<(Block, u64)>,

    /// Highest (height, round, step) we have signed a vote or proposal for.
    /// Never signs at or below it, so a restart cannot double-sign.
    last_signed: Option<(u64, u64, Step)>,

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
        let mut engine = Self {
            public_key,
            signing_key,
            height: start_height,
            round: 0,
            step: Step::Propose,
            validators: HashMap::new(),
            total_stake: 0,
            proposal: None,
            own_proposal: None,
            votes: HashMap::new(),
            max_votes_per_round: 1000,
            proposals_by_hash: HashMap::new(),
            future_proposals: HashMap::new(),
            locked_block: None,
            valid_block: None,
            last_signed: None,
            timeout_config: TimeoutConfig::default(),
            current_timeout: None,
            timeout_step: None,
        };
        engine.update_validator_set(validators);
        engine
    }

    /// Canonical vote signing bytes (kept for callers outside the engine).
    pub fn serialize_vote(&self, vote: &Vote) -> Vec<u8> {
        vote.signing_bytes()
    }

    /// Snapshot of the state that must be persisted before any signed message
    /// leaves the node.
    pub fn safety_state(&self) -> BftSafetyState {
        BftSafetyState {
            height: self.height,
            round: self.round,
            locked: self.locked_block.clone(),
            valid: self.valid_block.clone(),
            last_signed: self.last_signed,
        }
    }

    /// Restore persisted safety state after a restart. The signing watermark
    /// is always restored; the lock only if we resume at the same height.
    pub fn restore_safety_state(&mut self, state: BftSafetyState) {
        if state.last_signed > self.last_signed {
            self.last_signed = state.last_signed;
        }
        if state.height == self.height {
            self.round = self.round.max(state.round);
            self.locked_block = state.locked;
            self.valid_block = state.valid;
        }
    }

    /// Whether signing at (height, round, step) would not conflict with
    /// anything we already signed.
    fn may_sign(&self, step: Step) -> bool {
        match self.last_signed {
            Some(last) => (self.height, self.round, step) > last,
            None => true,
        }
    }

    /// Sign a vote for the current height/round, respecting the watermark.
    fn sign_vote(&mut self, step: Step, block_hash: Option<Hash>) -> Option<Vote> {
        if !self.may_sign(step) {
            tracing::warn!(
                "Refusing to sign {:?} at height={} round={}: already signed at or past {:?}",
                step,
                self.height,
                self.round,
                self.last_signed
            );
            return None;
        }
        let mut vote = Vote {
            height: self.height,
            round: self.round,
            step,
            block_hash,
            signature: vec![],
            voter: self.public_key.clone(),
        };
        vote.signature = self.signing_key.sign(&vote.signing_bytes());
        self.last_signed = Some((self.height, self.round, step));
        Some(vote)
    }

    pub fn start_round(&mut self, round: u64) -> Vec<BftEvent> {
        self.round = round;
        self.step = Step::Propose;
        self.proposal = None;
        self.own_proposal = None;
        // Proposals from a previous height are no longer useful for finalizing.
        let height = self.height;
        self.proposals_by_hash.retain(|_, p| p.height == height);
        self.future_proposals
            .retain(|r, p| p.height == height && *r >= round);
        self.start_timeout(Step::Propose);
        let mut events = vec![BftEvent::NewRound(self.height, self.round)];
        if let Some(proposal) = self.future_proposals.remove(&round) {
            events.extend(self.handle_proposal(proposal));
        }
        // Votes for this round may already have arrived (they are what made us
        // skip to it); a precommit quorum among them commits immediately.
        events.extend(self.check_quorum());
        events
    }

    /// A proposal is acceptable only from the round's proposer, with a valid
    /// signature. Checked before it may influence any state.
    fn proposal_is_authentic(&self, proposal: &Proposal) -> bool {
        if proposal.proposer != self.select_proposer(proposal.height, proposal.round) {
            tracing::warn!("Proposal from wrong proposer");
            return false;
        }
        if !proposal.verify_signature() {
            tracing::warn!("Invalid proposal signature");
            return false;
        }
        true
    }

    pub fn handle_proposal(&mut self, proposal: Proposal) -> Vec<BftEvent> {
        if proposal.height > self.height {
            return vec![BftEvent::BehindSync(proposal.height)];
        }
        if proposal.height != self.height || proposal.round < self.round {
            return vec![];
        }
        if !self.proposal_is_authentic(&proposal) {
            return vec![];
        }

        // A proposal alone never moves us to a higher round (anyone could
        // replay an old signed one); keep it until +1/3 of the stake is there.
        if proposal.round > self.round {
            if proposal.round <= self.round + MAX_FUTURE_ROUNDS {
                self.future_proposals.insert(proposal.round, proposal);
            }
            return vec![];
        }

        if self.step != Step::Propose {
            return vec![];
        }

        // Cache the valid proposal so a later precommit quorum on this block can
        // be finalized even if we are currently tracking a different proposal.
        if self.proposals_by_hash.len() >= 512 {
            self.proposals_by_hash.clear();
        }
        self.proposals_by_hash
            .insert(proposal.block.hash(), proposal.clone());

        self.proposal = Some(proposal.clone());
        self.step = Step::Prevote;
        self.start_timeout(Step::Prevote);

        let mut events = Vec::new();
        let vote_hash = self.prevote_hash(&proposal.block.hash());
        if let Some(vote) = self.sign_vote(Step::Prevote, vote_hash) {
            self.add_vote(vote.clone());
            events.push(BftEvent::BroadcastVote(vote));
        }
        events.extend(self.check_quorum());
        events
    }

    /// Decide what to prevote for a proposed block under the Tendermint lock
    /// rule: if we are locked on a different block, prevote nil unless there is
    /// a proof-of-lock-change (a prevote quorum for the new block this round).
    fn prevote_hash(&self, proposed_hash: &Hash) -> Option<Hash> {
        match &self.locked_block {
            Some((locked, _lock_round)) if locked.hash() != *proposed_hash => {
                if self.has_quorum(self.round, Step::Prevote) == Some(Some(*proposed_hash)) {
                    Some(*proposed_hash)
                } else {
                    tracing::info!("Locked on different block — prevoting nil");
                    None
                }
            }
            _ => Some(*proposed_hash),
        }
    }

    pub fn create_proposal(&mut self, block: Block) -> Vec<BftEvent> {
        if !self.is_proposer(self.height, self.round) || !self.may_sign(Step::Propose) {
            return vec![];
        }

        // Re-propose the block we are locked on / saw a quorum for, if any.
        let propose_block = match (&self.locked_block, &self.valid_block) {
            (Some((lb, _)), _) => lb.clone(),
            (None, Some((vb, _))) => vb.clone(),
            _ => block,
        };

        let mut proposal = Proposal {
            height: self.height,
            round: self.round,
            block: propose_block.clone(),
            signature: vec![],
            proposer: self.public_key.clone(),
        };
        proposal.signature = self.signing_key.sign(&proposal.signing_bytes());
        self.last_signed = Some((self.height, self.round, Step::Propose));

        self.proposal = Some(proposal.clone());
        self.own_proposal = Some(proposal.clone());
        self.proposals_by_hash
            .insert(propose_block.hash(), proposal.clone());
        self.step = Step::Prevote;
        self.start_timeout(Step::Prevote);

        let mut events = vec![BftEvent::BroadcastProposal(proposal)];
        let vote_hash = self.prevote_hash(&propose_block.hash());
        if let Some(vote) = self.sign_vote(Step::Prevote, vote_hash) {
            self.add_vote(vote.clone());
            events.push(BftEvent::BroadcastVote(vote));
        }
        events.extend(self.check_quorum());
        events
    }

    pub fn handle_vote(&mut self, vote: Vote) -> Vec<BftEvent> {
        if vote.height > self.height {
            // A peer is already voting at a future height: we are behind and
            // must sync the contiguous chain before we can participate.
            return vec![BftEvent::BehindSync(vote.height)];
        }
        if vote.height != self.height || vote.round < self.round {
            return vec![];
        }
        if vote.round > self.round + MAX_FUTURE_ROUNDS {
            return vec![];
        }
        if !self.validators.contains_key(&vote.voter) {
            return vec![];
        }
        // Authenticate before the vote can influence anything (including the
        // round we are in).
        if !vote.verify_signature() {
            tracing::warn!("Invalid vote signature");
            return vec![];
        }

        let vote_round = vote.round;
        let mut events = Vec::new();
        if let Some(evidence) = self.add_vote(vote) {
            events.push(BftEvent::Equivocation(evidence));
        }

        // Round skip (Tendermint): follow a higher round only once validators
        // holding more than 1/3 of the stake are in it — at least one of them
        // is honest, so a single (or a few) Byzantine validators cannot drag
        // the network forward.
        if vote_round > self.round {
            if is_one_third_plus(self.stake_in_round(vote_round), self.total_stake) {
                tracing::info!(
                    "Round-sync: +1/3 stake at round {} (was {}) — skipping ahead",
                    vote_round,
                    self.round
                );
                events.extend(self.start_round(vote_round));
            }
            return events;
        }

        events.extend(self.check_quorum());
        events
    }

    /// Stake of the distinct validators with any vote in `round`.
    fn stake_in_round(&self, round: u64) -> u128 {
        let mut voters = HashSet::new();
        for step in [Step::Prevote, Step::Precommit] {
            if let Some(votes) = self.votes.get(&(round, step)) {
                voters.extend(votes.keys());
            }
        }
        voters
            .into_iter()
            .filter_map(|v| self.validators.get(v))
            .fold(0u128, |acc, v| acc.saturating_add(v.stake))
    }

    pub fn add_vote_public(&mut self, vote: Vote) -> Option<DoubleSignEvidence> {
        self.add_vote(vote)
    }

    pub fn check_quorum_public(&mut self) -> Vec<BftEvent> {
        self.check_quorum()
    }

    /// Record a vote. The first vote per (round, step, voter) is kept; a later
    /// vote for a different block is returned as equivocation evidence instead
    /// of silently replacing it.
    fn add_vote(&mut self, vote: Vote) -> Option<DoubleSignEvidence> {
        let round_votes = self.votes.entry((vote.round, vote.step)).or_default();

        if let Some(existing) = round_votes.get(&vote.voter) {
            if existing.block_hash != vote.block_hash {
                tracing::warn!(
                    "Equivocation by {} at height={} round={} step={:?}",
                    hex::encode(&vote.voter[..vote.voter.len().min(6)]),
                    vote.height,
                    vote.round,
                    vote.step
                );
                return Some(DoubleSignEvidence {
                    first: existing.clone(),
                    second: vote,
                });
            }
            return None;
        }

        if round_votes.len() >= self.max_votes_per_round {
            tracing::warn!(
                "Too many votes for round {}, step {:?}",
                vote.round,
                vote.step
            );
            return None;
        }

        round_votes.insert(vote.voter.clone(), vote);
        None
    }

    fn check_quorum(&mut self) -> Vec<BftEvent> {
        let mut events = vec![];

        if self.step == Step::Prevote {
            if let Some(hash) = self.has_quorum(self.round, Step::Prevote) {
                self.step = Step::Precommit;
                self.start_timeout(Step::Precommit);

                // Lock on the block that got a prevote quorum this round.
                if let Some(block_hash) = &hash {
                    if let Some(proposal) = self.proposals_by_hash.get(block_hash) {
                        let current_lock_round =
                            self.locked_block.as_ref().map(|(_, r)| *r).unwrap_or(0);
                        if self.round >= current_lock_round {
                            let block = proposal.block.clone();
                            let round = self.round;
                            self.valid_block = Some((block.clone(), round));
                            self.locked_block = Some((block, round));
                        }
                    }
                }

                if let Some(vote) = self.sign_vote(Step::Precommit, hash) {
                    self.add_vote(vote.clone());
                    events.push(BftEvent::BroadcastVote(vote));
                }
            }
        }

        // Commit rule: +2/3 precommits for a block in this round finalize it,
        // whatever step we are in.
        if self.step != Step::Commit {
            if let Some(Some(hash)) = self.has_quorum(self.round, Step::Precommit) {
                // It may not be the proposal we are currently tracking (e.g. a
                // delayed proposer), so look it up from the proposals we saw.
                let finalized = self
                    .proposals_by_hash
                    .get(&hash)
                    .map(|p| (p.block.clone(), p.proposer.clone()))
                    .or_else(|| {
                        self.proposal
                            .as_ref()
                            .filter(|p| p.block.hash() == hash)
                            .map(|p| (p.block.clone(), p.proposer.clone()))
                    });

                match finalized {
                    Some((mut block, proposer)) => {
                        block.commit = Some(self.commit_certificate(self.round, hash));
                        self.step = Step::Commit;
                        events.push(BftEvent::FinalizeBlock(block, proposer));

                        self.height += 1;
                        self.round = 0;
                        self.step = Step::Propose;
                        self.proposal = None;
                        self.own_proposal = None;
                        self.locked_block = None;
                        self.valid_block = None;
                        self.votes.clear();
                        self.proposals_by_hash.clear();
                        self.future_proposals.clear();
                        events.push(BftEvent::NewRound(self.height, 0));
                    }
                    None => {
                        // The network reached 2/3 precommit quorum for a block we
                        // have not seen the proposal for. Request it from a peer
                        // instead of getting stuck or resetting the engine.
                        tracing::info!(
                            "Precommit quorum for a block not held locally — requesting sync"
                        );
                        events.push(BftEvent::NeedBlock(hash, self.height));
                    }
                }
            }
        }

        events
    }

    /// The precommits for `hash` in `round` at the current height.
    fn commit_certificate(&self, round: u64, hash: Hash) -> CommitCertificate {
        let mut precommits: Vec<Vote> = self
            .votes
            .get(&(round, Step::Precommit))
            .map(|votes| {
                votes
                    .values()
                    .filter(|v| v.block_hash == Some(hash))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        precommits.sort_by(|a, b| a.voter.cmp(&b.voter));
        CommitCertificate {
            height: self.height,
            round,
            block_hash: hash,
            precommits,
        }
    }

    pub fn has_quorum_public(&self, round: u64, step: Step) -> Option<Option<Hash>> {
        self.has_quorum(round, step)
    }

    fn has_quorum(&self, round: u64, step: Step) -> Option<Option<Hash>> {
        let votes = self.votes.get(&(round, step))?;

        let mut counts: HashMap<Option<Hash>, u128> = HashMap::new();
        for vote in votes.values() {
            let stake = self
                .validators
                .get(&vote.voter)
                .map(|v| v.stake)
                .unwrap_or(0);
            let entry = counts.entry(vote.block_hash).or_default();
            *entry = entry.saturating_add(stake);
        }

        counts
            .into_iter()
            .find(|(_, stake)| is_quorum(*stake, self.total_stake))
            .map(|(hash, _)| hash)
    }

    fn select_proposer(&self, height: u64, round: u64) -> Vec<u8> {
        let set = self
            .validators
            .values()
            .map(|v| (v.public_key.as_slice(), v.stake));
        match common::consensus_types::select_proposer(set, height, round) {
            Some(pk) => pk,
            None => {
                tracing::error!("No validators with stake configured!");
                self.public_key.clone()
            }
        }
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
        let timeout_ms = base_timeout_ms
            .saturating_add(self.timeout_config.timeout_delta_ms.saturating_mul(round))
            .min(self.timeout_config.max_timeout_ms.max(base_timeout_ms));
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
        tracing::info!(
            "Propose timeout at height={} round={} — voting nil",
            self.height,
            self.round
        );
        self.step = Step::Prevote;
        self.start_timeout(Step::Prevote);

        let mut events = Vec::new();
        if let Some(vote) = self.sign_vote(Step::Prevote, None) {
            self.add_vote(vote.clone());
            events.push(BftEvent::BroadcastVote(vote));
        }
        events.extend(self.check_quorum());
        events
    }

    pub fn handle_timeout_prevote(&mut self) -> Vec<BftEvent> {
        tracing::info!(
            "Prevote timeout at height={} round={} — precommitting nil",
            self.height,
            self.round
        );
        self.step = Step::Precommit;
        self.start_timeout(Step::Precommit);

        let mut events = Vec::new();
        if let Some(vote) = self.sign_vote(Step::Precommit, None) {
            self.add_vote(vote.clone());
            events.push(BftEvent::BroadcastVote(vote));
        }
        events.extend(self.check_quorum());
        events
    }

    pub fn handle_timeout_precommit(&mut self) -> Vec<BftEvent> {
        tracing::info!(
            "Precommit timeout at height={} round={} — advancing round",
            self.height,
            self.round
        );
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

    /// Current validator set as (public key, stake), sorted by key.
    pub fn validator_set(&self) -> Vec<(Vec<u8>, u128)> {
        let mut set: Vec<(Vec<u8>, u128)> = self
            .validators
            .values()
            .map(|v| (v.public_key.clone(), v.stake))
            .collect();
        set.sort();
        set
    }

    /// Replace the validator set. Call only between heights (after a block is
    /// finalized): changing it mid-height changes what counts as a quorum.
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

    /// Reset consensus state to begin voting at a new height after re-syncing
    /// the chain from peers. The lock is kept when staying at the same height
    /// (dropping it there would allow voting against it), and the signing
    /// watermark is never reset.
    pub fn reset_to_height(&mut self, new_height: u64) -> Vec<BftEvent> {
        let new_height = new_height.max(1);
        if new_height != self.height {
            self.locked_block = None;
            self.valid_block = None;
            self.round = 0;
        }
        self.height = new_height;
        self.step = Step::Propose;
        self.proposal = None;
        self.own_proposal = None;
        self.votes.clear();
        self.proposals_by_hash.clear();
        self.future_proposals.clear();
        self.current_timeout = None;
        self.timeout_step = None;
        let round = self.round;
        self.start_round(round)
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
                last_commit_hash: [0; 32],
                proposer: [0; 32],
            },
            extrinsics: vec![],
            commit: None,
            last_commit: None,
        }
    }

    struct Validator {
        sk: crypto::SigningKey,
        pk: Vec<u8>,
    }

    impl Validator {
        fn new() -> Self {
            let sk = crypto::SigningKey::generate();
            let pk = sk.public_key();
            Self { sk, pk }
        }

        fn key(&self) -> crypto::SigningKey {
            crypto::SigningKey::from_bytes(&self.sk.to_bytes()).unwrap()
        }

        fn vote(&self, height: u64, round: u64, step: Step, hash: Option<Hash>) -> Vote {
            let mut vote = Vote {
                height,
                round,
                step,
                block_hash: hash,
                signature: vec![],
                voter: self.pk.clone(),
            };
            vote.signature = self.sk.sign(&vote.signing_bytes());
            vote
        }

        fn proposal(&self, height: u64, round: u64, block: Block) -> Proposal {
            let mut proposal = Proposal {
                height,
                round,
                block,
                signature: vec![],
                proposer: self.pk.clone(),
            };
            proposal.signature = self.sk.sign(&proposal.signing_bytes());
            proposal
        }
    }

    fn validator_set(n: usize, stake: u128) -> (Vec<Validator>, Vec<ValidatorInfo>) {
        let vals: Vec<Validator> = (0..n).map(|_| Validator::new()).collect();
        let infos = vals
            .iter()
            .map(|v| ValidatorInfo {
                public_key: v.pk.clone(),
                stake,
                slashed: false,
            })
            .collect();
        (vals, infos)
    }

    /// Engine run by `vals[me]`.
    fn engine_for(vals: &[Validator], infos: &[ValidatorInfo], me: usize) -> BftEngine {
        BftEngine::new(vals[me].pk.clone(), infos.to_vec(), 1, vals[me].key())
    }

    /// Index of the proposer for (height, round).
    fn proposer_index(
        vals: &[Validator],
        infos: &[ValidatorInfo],
        height: u64,
        round: u64,
    ) -> usize {
        let engine = engine_for(vals, infos, 0);
        let pk = engine.select_proposer(height, round);
        vals.iter().position(|v| v.pk == pk).unwrap()
    }

    #[test]
    fn round_timeouts_grow_linearly_and_are_capped() {
        let (vals, infos) = validator_set(1, 100);
        let engine = engine_for(&vals, &infos, 0);
        let t = |round| engine.calculate_timeout(Step::Propose, round).as_millis();
        assert_eq!(t(0), 3_000);
        assert_eq!(t(4), 5_000);
        assert_eq!(t(29), 17_500);
        assert_eq!(t(10_000), 30_000);
    }

    #[test]
    fn test_bft_single_validator_full_flow() {
        let (vals, infos) = validator_set(1, 100);
        let mut engine = engine_for(&vals, &infos, 0);

        let events = engine.start_round(0);
        assert!(matches!(events[0], BftEvent::NewRound(1, 0)));

        let events = engine.create_proposal(make_block(1));
        assert!(events
            .iter()
            .any(|e| matches!(e, BftEvent::BroadcastProposal(_))));
        assert!(events
            .iter()
            .any(|e| matches!(e, BftEvent::BroadcastVote(v) if v.step == Step::Precommit)));
        let finalized = events.iter().find_map(|e| match e {
            BftEvent::FinalizeBlock(b, _) => Some(b.clone()),
            _ => None,
        });
        let block = finalized.expect("single validator finalizes its own proposal");
        let cert = block
            .commit
            .as_ref()
            .expect("finalized block carries a certificate");
        assert_eq!(
            cert.verify(&block.hash(), 1, &infos).unwrap(),
            vec![vals[0].pk.clone()]
        );

        assert_eq!(engine.height, 2);
        assert_eq!(engine.round, 0);
        assert_eq!(engine.step, Step::Propose);
    }

    #[test]
    fn test_proposal_must_be_signed() {
        let (vals, infos) = validator_set(1, 100);
        let mut engine = engine_for(&vals, &infos, 0);
        engine.start_round(0);

        let bad_proposal = Proposal {
            height: 1,
            round: 0,
            block: make_block(1),
            signature: vec![],
            proposer: vals[0].pk.clone(),
        };
        assert!(engine.handle_proposal(bad_proposal).is_empty());
    }

    #[test]
    fn test_timeout_propose_votes_nil() {
        let (vals, infos) = validator_set(1, 100);
        let mut engine = engine_for(&vals, &infos, 0);
        engine.start_round(0);

        let events = engine.handle_timeout_propose();
        match &events[0] {
            BftEvent::BroadcastVote(v) => {
                assert_eq!(v.step, Step::Prevote);
                assert!(v.block_hash.is_none());
            }
            other => panic!("Expected BroadcastVote, got {:?}", other),
        }
        assert!(!events
            .iter()
            .any(|e| matches!(e, BftEvent::FinalizeBlock(..))));
    }

    #[test]
    fn test_four_validators_full_flow() {
        let (vals, infos) = validator_set(4, 100);
        let p = proposer_index(&vals, &infos, 1, 0);
        let me = (p + 1) % 4;
        let mut engine = engine_for(&vals, &infos, me);
        engine.start_round(0);

        let block = make_block(1);
        let hash = block.hash();
        let events = engine.handle_proposal(vals[p].proposal(1, 0, block));
        assert!(events
            .iter()
            .any(|e| matches!(e, BftEvent::BroadcastVote(v) if v.step == Step::Prevote)));

        // Our prevote + the proposer's = 2 of 4: not a quorum.
        engine.handle_vote(vals[p].vote(1, 0, Step::Prevote, Some(hash)));
        assert_eq!(engine.step, Step::Prevote);

        // A third prevote is a quorum: we precommit.
        let other = (0..4).find(|i| *i != p && *i != me).unwrap();
        let events = engine.handle_vote(vals[other].vote(1, 0, Step::Prevote, Some(hash)));
        assert!(events
            .iter()
            .any(|e| matches!(e, BftEvent::BroadcastVote(v) if v.step == Step::Precommit)));

        // Precommits: ours + 2 more finalize, with a 3-signer certificate.
        engine.handle_vote(vals[p].vote(1, 0, Step::Precommit, Some(hash)));
        let events = engine.handle_vote(vals[other].vote(1, 0, Step::Precommit, Some(hash)));
        let block = events
            .iter()
            .find_map(|e| match e {
                BftEvent::FinalizeBlock(b, _) => Some(b.clone()),
                _ => None,
            })
            .expect("3 of 4 precommits finalize");
        let cert = block.commit.unwrap();
        assert_eq!(cert.verify(&hash, 1, &infos).unwrap().len(), 3);
    }

    /// Regression: the threshold was `>= floor(2N/3)`, so 2 of 4 equal-stake
    /// validators could finalize — two disjoint pairs could finalize
    /// conflicting blocks.
    #[test]
    fn test_two_of_four_is_not_a_quorum() {
        let (vals, infos) = validator_set(4, 100);
        let mut engine = engine_for(&vals, &infos, 0);
        let hash = make_block(1).hash();

        engine.add_vote_public(vals[0].vote(1, 0, Step::Prevote, Some(hash)));
        engine.add_vote_public(vals[1].vote(1, 0, Step::Prevote, Some(hash)));
        assert_eq!(engine.has_quorum_public(0, Step::Prevote), None);

        engine.add_vote_public(vals[2].vote(1, 0, Step::Prevote, Some(hash)));
        assert_eq!(engine.has_quorum_public(0, Step::Prevote), Some(Some(hash)));
    }

    #[test]
    fn test_two_of_three_is_not_a_quorum() {
        // n = 3 tolerates no faults: all three are needed.
        let (vals, infos) = validator_set(3, 100);
        let mut engine = engine_for(&vals, &infos, 0);
        let hash = make_block(1).hash();
        engine.add_vote_public(vals[0].vote(1, 0, Step::Prevote, Some(hash)));
        engine.add_vote_public(vals[1].vote(1, 0, Step::Prevote, Some(hash)));
        assert_eq!(engine.has_quorum_public(0, Step::Prevote), None);
        engine.add_vote_public(vals[2].vote(1, 0, Step::Prevote, Some(hash)));
        assert_eq!(engine.has_quorum_public(0, Step::Prevote), Some(Some(hash)));
    }

    #[test]
    fn test_wei_scale_stakes_require_real_two_thirds() {
        // Stakes live in the wei range; 2/3 of 2.4e24 must be exceeded.
        let vals: Vec<Validator> = (0..3).map(|_| Validator::new()).collect();
        let stakes = [
            1_000_000_000_000_000_000_000_000u128, // 1e24
            800_000_000_000_000_000_000_000u128,   // 8e23
            600_000_000_000_000_000_000_000u128,   // 6e23
        ];
        let infos: Vec<ValidatorInfo> = vals
            .iter()
            .zip(stakes)
            .map(|(v, stake)| ValidatorInfo {
                public_key: v.pk.clone(),
                stake,
                slashed: false,
            })
            .collect();
        let mut engine = engine_for(&vals, &infos, 0);
        let hash = make_block(1).hash();

        engine.add_vote_public(vals[0].vote(1, 0, Step::Prevote, Some(hash)));
        assert_eq!(engine.has_quorum_public(0, Step::Prevote), None);
        // 1e24 + 8e23 = 1.8e24 > 1.6e24.
        engine.add_vote_public(vals[1].vote(1, 0, Step::Prevote, Some(hash)));
        assert_eq!(engine.has_quorum_public(0, Step::Prevote), Some(Some(hash)));
    }

    /// Regression: the round used to jump before the signature was checked,
    /// so one forged vote with a huge round stalled every node.
    #[test]
    fn test_forged_high_round_vote_does_not_move_round() {
        let (vals, infos) = validator_set(4, 100);
        let mut engine = engine_for(&vals, &infos, 0);
        engine.start_round(0);

        let mut forged = vals[1].vote(1, 50, Step::Prevote, None);
        forged.signature = vals[2].sk.sign(&forged.signing_bytes()); // wrong signer
        assert!(engine.handle_vote(forged).is_empty());
        assert_eq!(engine.round, 0);

        let far = vals[1].vote(1, u64::MAX, Step::Prevote, None);
        assert!(engine.handle_vote(far).is_empty());
        assert_eq!(engine.round, 0);
    }

    #[test]
    fn test_round_skip_needs_more_than_one_third_stake() {
        let (vals, infos) = validator_set(4, 100);
        let mut engine = engine_for(&vals, &infos, 0);
        engine.start_round(0);

        // One validator (1/4) at round 3: not enough to follow.
        engine.handle_vote(vals[1].vote(1, 3, Step::Prevote, None));
        assert_eq!(engine.round, 0);

        // A second one (2/4 > 1/3): follow to round 3.
        let events = engine.handle_vote(vals[2].vote(1, 3, Step::Precommit, None));
        assert_eq!(engine.round, 3);
        assert!(events.iter().any(|e| matches!(e, BftEvent::NewRound(1, 3))));

        // Votes from lower rounds are now ignored.
        assert!(engine
            .handle_vote(vals[3].vote(1, 1, Step::Prevote, None))
            .is_empty());
    }

    #[test]
    fn test_future_proposal_waits_for_round() {
        let (vals, infos) = validator_set(4, 100);
        let p = proposer_index(&vals, &infos, 1, 2);
        let me = (p + 1) % 4;
        let mut engine = engine_for(&vals, &infos, me);
        engine.start_round(0);

        let block = make_block(1);
        assert!(engine
            .handle_proposal(vals[p].proposal(1, 2, block.clone()))
            .is_empty());
        assert_eq!(engine.round, 0, "a proposal alone never moves the round");

        // Entering round 2 replays the stored proposal: we prevote it.
        let events = engine.start_round(2);
        assert!(events.iter().any(|e| matches!(
            e,
            BftEvent::BroadcastVote(v) if v.step == Step::Prevote && v.block_hash == Some(block.hash())
        )));
    }

    /// Regression: a second, conflicting vote silently replaced the first.
    #[test]
    fn test_equivocation_produces_evidence() {
        let (vals, infos) = validator_set(4, 100);
        let mut engine = engine_for(&vals, &infos, 0);
        engine.start_round(0);

        let a = make_block(1).hash();
        let b = make_block(2).hash();
        let first = vals[1].vote(1, 0, Step::Prevote, Some(a));
        let second = vals[1].vote(1, 0, Step::Prevote, Some(b));

        assert!(engine.handle_vote(first.clone()).is_empty());
        // Re-delivery of the same vote is not equivocation.
        assert!(engine.handle_vote(first.clone()).is_empty());

        let events = engine.handle_vote(second.clone());
        let evidence = events
            .iter()
            .find_map(|e| match e {
                BftEvent::Equivocation(ev) => Some(ev.clone()),
                _ => None,
            })
            .expect("conflicting vote yields evidence");
        assert!(evidence.is_valid());
        assert_eq!(evidence.first, first);
        assert_eq!(evidence.second, second);

        // The first vote is the one that counts.
        let kept = &engine.votes[&(0, Step::Prevote)][&vals[1].pk];
        assert_eq!(kept.block_hash, Some(a));
    }

    /// Regression: the lock was persisted as `None`, so after a restart a
    /// validator could prevote against its lock; there was also no guard
    /// against signing twice for the same height/round/step.
    #[test]
    fn test_safety_state_survives_restart() {
        let (vals, infos) = validator_set(4, 100);
        let mut engine = engine_for(&vals, &infos, 0);
        engine.start_round(0);
        let locked = make_block(1);
        engine.locked_block = Some((locked.clone(), 0));
        engine.handle_timeout_propose(); // signs a nil prevote at (1, 0)

        let saved = serde_json::to_string(&engine.safety_state()).unwrap();

        // "Restart": a fresh engine at the same height.
        let mut restarted = engine_for(&vals, &infos, 0);
        restarted.restore_safety_state(serde_json::from_str(&saved).unwrap());
        assert_eq!(restarted.locked_block, Some((locked.clone(), 0)));

        // Already prevoted at (1, 0): must not sign another prevote there.
        restarted.start_round(0);
        assert!(restarted.handle_timeout_propose().is_empty());

        // Next round: locked on `locked`, so a different block gets nil.
        restarted.start_round(1);
        let p = proposer_index(&vals, &infos, 1, 1);
        if p != 0 {
            let other = make_block(9);
            let events = restarted.handle_proposal(vals[p].proposal(1, 1, other));
            assert!(events.iter().any(|e| matches!(
                e,
                BftEvent::BroadcastVote(v) if v.step == Step::Prevote && v.block_hash.is_none()
            )));
        }
    }

    #[test]
    fn test_commit_certificate_rejects_tampering() {
        let (vals, infos) = validator_set(4, 100);
        let hash = make_block(1).hash();
        let precommits: Vec<Vote> = vals[..3]
            .iter()
            .map(|v| v.vote(1, 0, Step::Precommit, Some(hash)))
            .collect();
        let cert = CommitCertificate {
            height: 1,
            round: 0,
            block_hash: hash,
            precommits: precommits.clone(),
        };
        assert!(cert.verify(&hash, 1, &infos).is_ok());

        // Wrong block / height.
        assert!(cert.verify(&make_block(2).hash(), 1, &infos).is_err());
        assert!(cert.verify(&hash, 2, &infos).is_err());

        // Only 2 of 4 signers.
        let short = CommitCertificate {
            precommits: precommits[..2].to_vec(),
            ..cert.clone()
        };
        assert!(short.verify(&hash, 1, &infos).is_err());

        // Same signer counted twice.
        let dup = CommitCertificate {
            precommits: vec![
                precommits[0].clone(),
                precommits[0].clone(),
                precommits[1].clone(),
            ],
            ..cert.clone()
        };
        assert!(dup.verify(&hash, 1, &infos).is_err());

        // Bad signature.
        let mut forged = cert.clone();
        forged.precommits[2].signature = vals[3].sk.sign(&forged.precommits[2].signing_bytes());
        assert!(forged.verify(&hash, 1, &infos).is_err());
    }

    #[test]
    fn test_re_propose_rebroadcasts_current_proposal() {
        let (vals, infos) = validator_set(3, 100);
        let p = proposer_index(&vals, &infos, 1, 0);
        let mut engine = engine_for(&vals, &infos, p);
        engine.start_round(0);
        engine.create_proposal(make_block(0));

        let events = engine.re_propose();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], BftEvent::BroadcastProposal(_)));

        // After moving to a new round the old proposal must not be re-broadcast.
        engine.start_round(1);
        assert!(engine.re_propose().is_empty());
    }

    #[test]
    fn test_reset_to_height_clears_lock_and_votes() {
        let (vals, infos) = validator_set(1, 100);
        let mut engine = engine_for(&vals, &infos, 0);
        engine.start_round(0);

        engine.locked_block = Some((make_block(5), 0));
        engine.handle_timeout_propose();
        engine.handle_timeout_prevote();

        // Same height: the lock must survive the reset.
        engine.reset_to_height(1);
        assert!(engine.locked_block.is_some());

        let events = engine.reset_to_height(42);
        assert_eq!(engine.height, 42);
        assert_eq!(engine.round, 0);
        assert_eq!(engine.step, Step::Propose);
        assert!(engine.locked_block.is_none());
        assert!(engine.valid_block.is_none());
        assert!(engine.proposal.is_none());
        assert!(engine.votes.is_empty());
        assert!(events
            .iter()
            .any(|e| matches!(e, BftEvent::NewRound(42, 0))));
    }

    #[test]
    fn test_votes_retained_across_rounds() {
        let (vals, infos) = validator_set(1, 100);
        let mut engine = engine_for(&vals, &infos, 0);
        engine.start_round(0);
        engine.handle_timeout_propose();
        engine.start_round(1);
        assert!(!engine.collect_past_round_votes().is_empty());
    }

    /// Fuzz-style: a stream of random votes and proposals — mostly validly
    /// signed by real validators, with arbitrary heights, rounds, steps and
    /// hashes, plus garbage signatures and unknown voters — must never panic
    /// the engine, and it must never sign twice for the same slot.
    #[test]
    fn test_engine_survives_random_messages() {
        struct Rng(u64);
        impl Rng {
            fn next(&mut self) -> u64 {
                self.0 ^= self.0 << 13;
                self.0 ^= self.0 >> 7;
                self.0 ^= self.0 << 17;
                self.0
            }
            fn below(&mut self, n: u64) -> u64 {
                self.next() % n.max(1)
            }
        }
        let (vals, infos) = validator_set(4, 100);
        let outsider = Validator::new();
        let mut engine = engine_for(&vals, &infos, 0);
        engine.start_round(0);
        let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
        let mut signed: HashSet<(u64, u64, Step)> = HashSet::new();

        for _ in 0..5_000 {
            let who = rng.below(5) as usize;
            let signer = if who == 4 { &outsider } else { &vals[who] };
            let height = match rng.below(4) {
                0 => rng.next(),
                _ => engine.height.saturating_add(rng.below(2)),
            };
            let round = match rng.below(4) {
                0 => rng.next(),
                _ => engine.round + rng.below(3),
            };
            let hash = match rng.below(3) {
                0 => None,
                _ => Some(make_block(rng.below(3)).hash()),
            };
            let step = [Step::Propose, Step::Prevote, Step::Precommit, Step::Commit]
                [rng.below(4) as usize];
            let events = if rng.below(4) == 0 {
                let mut p = signer.proposal(height, round, make_block(height.wrapping_sub(1)));
                if rng.below(3) == 0 {
                    p.signature = vec![rng.next() as u8; rng.below(80) as usize];
                }
                engine.handle_proposal(p)
            } else {
                let mut v = signer.vote(height, round, step, hash);
                if rng.below(3) == 0 {
                    v.signature = vec![rng.next() as u8; rng.below(80) as usize];
                }
                engine.handle_vote(v)
            };
            let mut events = events;
            if rng.below(10) == 0 {
                events.extend(match rng.below(3) {
                    0 => engine.handle_timeout_propose(),
                    1 => engine.handle_timeout_prevote(),
                    _ => engine.handle_timeout_precommit(),
                });
            }
            for e in &events {
                if let BftEvent::BroadcastVote(v) = e {
                    assert!(
                        signed.insert((v.height, v.round, v.step)),
                        "signed twice at {:?}",
                        (v.height, v.round, v.step)
                    );
                }
            }
        }
    }
}
