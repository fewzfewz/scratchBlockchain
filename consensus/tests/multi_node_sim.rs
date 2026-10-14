//! Deterministic multi-node BFT simulation.
//!
//! Runs several `BftEngine`s over a simulated gossip network with seeded
//! random delivery order, and checks the two properties that matter:
//!
//! - **safety**: no two nodes ever finalize different blocks at one height;
//! - **liveness**: with more than 2/3 of the stake honest and connected, the
//!   chain keeps finalizing.
//!
//! Scenarios: all honest, a 2|2 network partition that heals, a validator
//! that crashes mid-round and restarts from its persisted safety state, and a
//! Byzantine validator that equivocates (conflicting proposals and votes to
//! different peers).
//!
//! Time is simulated: when no message is in flight, every node's current
//! step times out (and proposers re-broadcast, as the node's main loop does).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::consensus_types::{
    ConsensusMessage, DoubleSignEvidence, Proposal, Step, ValidatorInfo, Vote,
};
use common::crypto::SigningKey;
use common::types::{Block, Hash, Header};
use consensus::bft::{BftEngine, BftEvent};
use std::collections::{BTreeMap, HashSet, VecDeque};

/// Small deterministic PRNG (xorshift64*), so failures reproduce from a seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn msg_id(msg: &ConsensusMessage) -> Hash {
    let bytes = match msg {
        ConsensusMessage::Vote(v) => [v.signing_bytes(), v.signature.clone()].concat(),
        ConsensusMessage::Proposal(p) => [p.signing_bytes(), p.signature.clone()].concat(),
    };
    common::crypto::hash(&bytes)
}

/// A distinct block per (height, round, author, variant).
fn make_block(height: u64, round: u64, author: usize, variant: u8) -> Block {
    let mut header = Header::new([0; 32], height.saturating_sub(1));
    let mut tag = Vec::new();
    tag.extend_from_slice(&height.to_le_bytes());
    tag.extend_from_slice(&round.to_le_bytes());
    tag.extend_from_slice(&(author as u64).to_le_bytes());
    tag.push(variant);
    header.extrinsics_root = common::crypto::hash(&tag);
    Block::new(header, vec![])
}

fn hex8(h: &Hash) -> String {
    h[..4].iter().map(|b| format!("{b:02x}")).collect()
}

struct SimNode {
    key_bytes: Vec<u8>,
    engine: BftEngine,
    online: bool,
    /// Finalized block hash per BFT height.
    finalized: BTreeMap<u64, Hash>,
    seen: HashSet<Hash>,
    proposed: HashSet<(u64, u64)>,
    evidence: Vec<DoubleSignEvidence>,
}

struct Sim {
    nodes: Vec<SimNode>,
    infos: Vec<ValidatorInfo>,
    /// (from, to, message)
    queue: VecDeque<(usize, usize, ConsensusMessage)>,
    rng: Rng,
    /// Links that drop messages: `blocked(from, to)`.
    partition: Option<Box<dyn Fn(usize, usize) -> bool>>,
    /// Index of a Byzantine validator driven by the simulator, not an engine.
    byzantine: Option<usize>,
    byz_acted: HashSet<(u64, u64)>,
    rebroadcast_done: bool,
}

impl Sim {
    fn new(n: usize, seed: u64) -> Self {
        let keys: Vec<SigningKey> = (0..n)
            .map(|i| {
                let mut seed_bytes = [0u8; 32];
                seed_bytes[0] = i as u8 + 1;
                seed_bytes[1] = seed as u8;
                SigningKey::from_bytes(&seed_bytes).unwrap()
            })
            .collect();
        let infos: Vec<ValidatorInfo> = keys
            .iter()
            .map(|k| ValidatorInfo {
                public_key: k.public_key(),
                stake: 100,
                slashed: false,
            })
            .collect();
        let nodes = keys
            .iter()
            .map(|k| {
                let key_bytes = k.to_bytes();
                let engine = BftEngine::new(
                    k.public_key(),
                    infos.clone(),
                    1,
                    SigningKey::from_bytes(&key_bytes).unwrap(),
                );
                SimNode {
                    key_bytes,
                    engine,
                    online: true,
                    finalized: BTreeMap::new(),
                    seen: HashSet::new(),
                    proposed: HashSet::new(),
                    evidence: Vec::new(),
                }
            })
            .collect();
        let mut sim = Self {
            nodes,
            infos,
            queue: VecDeque::new(),
            rng: Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1),
            partition: None,
            byzantine: None,
            byz_acted: HashSet::new(),
            rebroadcast_done: false,
        };
        for i in 0..n {
            let events = sim.nodes[i].engine.start_round(0);
            sim.handle_events(i, events);
        }
        sim
    }

    fn honest(&self, i: usize) -> bool {
        self.byzantine != Some(i)
    }

    fn linked(&self, from: usize, to: usize) -> bool {
        !self.partition.as_ref().is_some_and(|cut| cut(from, to))
    }

    fn broadcast(&mut self, from: usize, msg: ConsensusMessage) {
        self.nodes[from].seen.insert(msg_id(&msg));
        for to in 0..self.nodes.len() {
            if to != from {
                self.queue.push_back((from, to, msg.clone()));
            }
        }
    }

    fn handle_events(&mut self, i: usize, events: Vec<BftEvent>) {
        for event in events {
            match event {
                BftEvent::BroadcastVote(v) => self.broadcast(i, ConsensusMessage::Vote(v)),
                BftEvent::BroadcastProposal(p) => self.broadcast(i, ConsensusMessage::Proposal(p)),
                BftEvent::FinalizeBlock(block, _) => {
                    let cert = block.commit.as_ref().expect("finalized block has a cert");
                    let prev = self.nodes[i].finalized.insert(cert.height, block.hash());
                    assert!(
                        prev.is_none() || prev == Some(block.hash()),
                        "node {i} finalized two blocks at height {}",
                        cert.height
                    );
                }
                BftEvent::Equivocation(e) => self.nodes[i].evidence.push(e),
                BftEvent::BehindSync(_) | BftEvent::NeedBlock(_, _) => self.sync(i),
                BftEvent::NewRound(_, _) | BftEvent::Timeout(_) => {}
            }
        }
    }

    /// Catch up from the furthest reachable honest peer, as block sync would
    /// (the real node verifies each block's commit certificate).
    fn sync(&mut self, i: usize) {
        let ours = self.nodes[i].engine.height;
        let best = (0..self.nodes.len())
            .filter(|&j| j != i && self.honest(j) && self.nodes[j].online)
            .filter(|&j| self.linked(j, i))
            .max_by_key(|&j| self.nodes[j].engine.height);
        let Some(j) = best else { return };
        let target = self.nodes[j].engine.height;
        if target <= ours {
            return;
        }
        let blocks: Vec<(u64, Hash)> = self.nodes[j]
            .finalized
            .range(ours..target)
            .map(|(h, b)| (*h, *b))
            .collect();
        for (h, b) in blocks {
            let prev = self.nodes[i].finalized.insert(h, b);
            assert!(prev.is_none() || prev == Some(b), "sync conflicts at {h}");
        }
        let events = self.nodes[i].engine.reset_to_height(target);
        self.handle_events(i, events);
    }

    fn maybe_propose(&mut self, i: usize) {
        let (h, r) = (self.nodes[i].engine.height, self.nodes[i].engine.round);
        let node = &self.nodes[i];
        if node.engine.step == Step::Propose
            && node.engine.is_proposer(h, r)
            && !node.proposed.contains(&(h, r))
        {
            self.nodes[i].proposed.insert((h, r));
            let events = self.nodes[i].engine.create_proposal(make_block(h, r, i, 0));
            self.handle_events(i, events);
        }
    }

    /// The Byzantine validator equivocates in every (height, round) it sees:
    /// as proposer it sends block A to one half of the peers and block B to
    /// the other, and it prevotes and precommits A to one half and B to the
    /// other.
    fn byzantine_act(&mut self, h: u64, r: u64) {
        let Some(b) = self.byzantine else { return };
        if !self.byz_acted.insert((h, r)) {
            return;
        }
        let sk = SigningKey::from_bytes(&self.nodes[b].key_bytes).unwrap();
        let pk = sk.public_key();
        let blocks = [make_block(h, r, b, 1), make_block(h, r, b, 2)];
        let is_proposer = self.nodes[b].engine.is_proposer(h, r);
        let peers: Vec<usize> = (0..self.nodes.len()).filter(|&j| j != b).collect();
        for (k, to) in peers.into_iter().enumerate() {
            let block = &blocks[k % 2];
            if is_proposer {
                let mut p = Proposal {
                    height: h,
                    round: r,
                    block: block.clone(),
                    signature: vec![],
                    proposer: pk.clone(),
                };
                p.signature = sk.sign(&p.signing_bytes());
                self.queue.push_back((b, to, ConsensusMessage::Proposal(p)));
            }
            for step in [Step::Prevote, Step::Precommit] {
                let mut v = Vote {
                    height: h,
                    round: r,
                    step,
                    block_hash: Some(block.hash()),
                    signature: vec![],
                    voter: pk.clone(),
                };
                v.signature = sk.sign(&v.signing_bytes());
                self.queue.push_back((b, to, ConsensusMessage::Vote(v)));
            }
        }
    }

    fn deliver(&mut self, from: usize, to: usize, msg: ConsensusMessage) {
        if !self.nodes[to].online || !self.linked(from, to) || !self.honest(to) {
            return;
        }
        // Gossip: each node handles (and relays) a message once.
        if !self.nodes[to].seen.insert(msg_id(&msg)) {
            return;
        }
        let events = match msg.clone() {
            ConsensusMessage::Vote(v) => self.nodes[to].engine.handle_vote(v),
            ConsensusMessage::Proposal(p) => self.nodes[to].engine.handle_proposal(p),
        };
        for peer in 0..self.nodes.len() {
            if peer != to && peer != from {
                self.queue.push_back((to, peer, msg.clone()));
            }
        }
        self.handle_events(to, events);
    }

    /// Nothing in flight: time passes. First proposers re-broadcast (as the
    /// node's main loop does) and gossip caches expire; once that has been
    /// delivered too, every online node's current step times out.
    fn tick(&mut self) {
        if !self.rebroadcast_done {
            self.rebroadcast_done = true;
            for i in 0..self.nodes.len() {
                self.nodes[i].seen.clear();
                if self.nodes[i].online && self.honest(i) {
                    let events = self.nodes[i].engine.re_propose();
                    self.handle_events(i, events);
                }
            }
            if !self.queue.is_empty() {
                return;
            }
        }
        self.rebroadcast_done = false;
        for i in 0..self.nodes.len() {
            if !self.nodes[i].online || !self.honest(i) {
                continue;
            }
            let engine = &mut self.nodes[i].engine;
            let events = match engine.step {
                Step::Propose => engine.handle_timeout_propose(),
                Step::Prevote => engine.handle_timeout_prevote(),
                Step::Precommit => engine.handle_timeout_precommit(),
                Step::Commit => vec![],
            };
            self.handle_events(i, events);
        }
    }

    fn step(&mut self) {
        for i in 0..self.nodes.len() {
            if self.nodes[i].online && self.honest(i) {
                self.maybe_propose(i);
                let (h, r) = (self.nodes[i].engine.height, self.nodes[i].engine.round);
                self.byzantine_act(h, r);
            }
        }
        if self.queue.is_empty() {
            self.tick();
            return;
        }
        // Random delivery order: messages are reordered arbitrarily.
        let idx = self.rng.below(self.queue.len());
        if let Some((from, to, msg)) = self.queue.remove(idx) {
            self.deliver(from, to, msg);
        }
    }

    fn min_honest_height(&self) -> u64 {
        (0..self.nodes.len())
            .filter(|&i| self.honest(i) && self.nodes[i].online)
            .map(|i| self.nodes[i].engine.height)
            .min()
            .unwrap_or(0)
    }

    /// Step until every online honest node is at `height`, or panic.
    fn run_until_height(&mut self, height: u64, max_steps: usize) {
        for _ in 0..max_steps {
            if self.min_honest_height() >= height {
                return;
            }
            self.step();
        }
        let state: Vec<String> = self
            .nodes
            .iter()
            .map(|n| {
                let s = n.engine.safety_state();
                format!(
                    "h{} r{} {:?} locked={:?} last_signed={:?}",
                    n.engine.height,
                    n.engine.round,
                    n.engine.step,
                    s.locked.as_ref().map(|(b, r)| (hex8(&b.hash()), *r)),
                    s.last_signed
                )
            })
            .collect();
        panic!("no liveness: wanted height {height}, at {state:#?}");
    }

    fn run_steps(&mut self, steps: usize) {
        for _ in 0..steps {
            self.step();
        }
    }

    /// No two nodes finalized different blocks at the same height.
    fn assert_safety(&self) {
        let mut canonical: BTreeMap<u64, Hash> = BTreeMap::new();
        for (i, node) in self.nodes.iter().enumerate() {
            if !self.honest(i) {
                continue;
            }
            for (h, b) in &node.finalized {
                let first = canonical.entry(*h).or_insert(*b);
                assert_eq!(first, b, "fork at height {h}: node {i} disagrees");
            }
        }
    }

    /// Crash node `i` (in-memory state lost) and restart it from the safety
    /// state it persisted (lock + last-signed watermark), as the node does.
    fn restart(&mut self, i: usize) {
        let safety = self.nodes[i].engine.safety_state();
        let height = self.nodes[i].engine.height;
        let key = SigningKey::from_bytes(&self.nodes[i].key_bytes).unwrap();
        let mut engine = BftEngine::new(key.public_key(), self.infos.clone(), height, key);
        engine.restore_safety_state(safety);
        self.nodes[i].engine = engine;
        self.nodes[i].seen.clear();
        let events = self.nodes[i].engine.reset_to_height(height);
        self.handle_events(i, events);
    }
}

const SEEDS: [u64; 5] = [1, 7, 42, 1337, 9001];

#[test]
fn honest_network_finalizes_and_agrees() {
    for seed in SEEDS {
        let mut sim = Sim::new(4, seed);
        sim.run_until_height(15, 200_000);
        sim.assert_safety();
    }
}

#[test]
fn partition_halts_without_forking_and_recovers_after_heal() {
    for seed in SEEDS {
        let mut sim = Sim::new(4, seed);
        sim.run_until_height(4, 100_000);

        // 2 | 2: neither side has > 2/3 of the stake.
        sim.partition = Some(Box::new(|a, b| (a < 2) != (b < 2)));
        let max_before = sim.nodes.iter().map(|n| n.engine.height).max().unwrap();
        sim.run_steps(4_000);
        let max_during = sim.nodes.iter().map(|n| n.engine.height).max().unwrap();
        // A height already in flight may complete; nothing beyond it can.
        assert!(
            max_during <= max_before + 1,
            "seed {seed}: progressed during partition ({max_before} -> {max_during})"
        );
        sim.assert_safety();

        sim.partition = None;
        sim.run_until_height(max_during + 3, 200_000);
        sim.assert_safety();
    }
}

#[test]
fn validator_restarting_mid_round_keeps_safety() {
    for seed in SEEDS {
        let mut sim = Sim::new(4, seed);
        sim.run_until_height(3, 100_000);
        for crash in 0..4 {
            // Crash at an arbitrary point inside a round, then restart from
            // the persisted lock/watermark.
            let steps = 50 + sim.rng.below(400);
            sim.run_steps(steps);
            sim.nodes[crash].online = false;
            sim.run_steps(200);
            sim.nodes[crash].online = true;
            sim.restart(crash);
        }
        let target = sim.min_honest_height() + 5;
        sim.run_until_height(target, 200_000);
        sim.assert_safety();
    }
}

#[test]
fn equivocating_validator_cannot_fork_and_is_caught() {
    for seed in SEEDS {
        let mut sim = Sim::new(4, seed);
        sim.byzantine = Some(3);
        sim.run_until_height(12, 400_000);
        sim.assert_safety();

        let byz_pk = sim.infos[3].public_key.clone();
        for i in 0..3 {
            assert!(
                !sim.nodes[i].evidence.is_empty(),
                "seed {seed}: node {i} recorded no equivocation evidence"
            );
            for e in &sim.nodes[i].evidence {
                assert_eq!(e.first.voter, byz_pk, "evidence against an honest node");
            }
        }
    }
}
