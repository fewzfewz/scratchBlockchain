use libp2p::PeerId;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Peer reputation score
/// Range: -100 to 100
/// Initial score: 0
/// Ban threshold: -50
#[derive(Debug, Clone, Copy)]
pub struct ReputationScore(i32);

impl ReputationScore {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn value(&self) -> i32 {
        self.0
    }

    pub fn is_banned(&self) -> bool {
        self.0 <= -50
    }

    pub fn update(&mut self, change: i32) {
        self.0 = (self.0 + change).clamp(-100, 100);
    }
}

pub struct PeerReputation {
    scores: HashMap<PeerId, ReputationScore>,
    last_update: HashMap<PeerId, Instant>,
    banned_peers: HashMap<PeerId, Instant>, // PeerId -> Ban expiration
}

impl PeerReputation {
    pub fn new() -> Self {
        Self {
            scores: HashMap::new(),
            last_update: HashMap::new(),
            banned_peers: HashMap::new(),
        }
    }

    pub fn get_score(&self, peer: &PeerId) -> i32 {
        self.scores.get(peer).map(|s| s.value()).unwrap_or(0)
    }

    pub fn is_banned(&self, peer: &PeerId) -> bool {
        if let Some(expiration) = self.banned_peers.get(peer) {
            if Instant::now() < *expiration {
                return true;
            }
        }
        false
    }

    pub fn report_good_behavior(&mut self, peer: PeerId) {
        let score = self.scores.entry(peer).or_insert(ReputationScore::new());
        score.update(1);
        self.last_update.insert(peer, Instant::now());
    }

    pub fn report_bad_behavior(&mut self, peer: PeerId, severity: i32) {
        let score = self.scores.entry(peer).or_insert(ReputationScore::new());
        score.update(-severity);
        self.last_update.insert(peer, Instant::now());

        if score.is_banned() {
            // Ban for 1 hour
            self.banned_peers.insert(peer, Instant::now() + Duration::from_secs(3600));
        }
    }

    pub fn cleanup(&mut self) {
        // Remove expired bans
        let now = Instant::now();
        self.banned_peers.retain(|_, expiration| *expiration > now);
        
        // Decay scores over time (normalize towards 0)
        for (peer, last_update) in &self.last_update {
            if now.duration_since(*last_update) > Duration::from_secs(3600) {
                if let Some(score) = self.scores.get_mut(peer) {
                    if score.value() > 0 {
                        score.update(-1);
                    } else if score.value() < 0 {
                        score.update(1);
                    }
                }
            }
        }
    }
}
