//! # Peer Reputation Module
//!
//! Tracks peer behavior and automatically bans malicious peers.
//! Good behavior (valid messages) increases score.
//! Bad behavior (invalid messages, rate limits) decreases score.
//! Score range: -100 to 100, ban threshold: -50

use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::info;

/// Peer reputation score
/// Range: -100 to 100
/// Initial score: 0
/// Ban threshold: -50
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReputationScore(i32);

impl Default for ReputationScore {
    fn default() -> Self {
        Self::new()
    }
}

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

/// Serializable reputation data for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReputationData {
    scores: HashMap<String, i32>,       // PeerId string -> score
    banned_until: HashMap<String, u64>, // PeerId string -> ban expiration timestamp
}

pub struct PeerReputation {
    scores: HashMap<PeerId, ReputationScore>,
    last_update: HashMap<PeerId, Instant>,
    banned_peers: HashMap<PeerId, Instant>, // PeerId -> Ban expiration
}

impl Default for PeerReputation {
    fn default() -> Self {
        Self::new()
    }
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
        let score = self.scores.entry(peer).or_default();
        score.update(1);
        self.last_update.insert(peer, Instant::now());
    }

    pub fn report_bad_behavior(&mut self, peer: PeerId, severity: i32) {
        let score = self.scores.entry(peer).or_default();
        score.update(-severity);
        self.last_update.insert(peer, Instant::now());

        if score.is_banned() {
            // Ban for 1 hour
            let ban_duration = Duration::from_secs(3600);
            self.banned_peers
                .insert(peer, Instant::now() + ban_duration);
            info!(
                "🚫 Peer {} banned for {:?} (score: {})",
                peer,
                ban_duration,
                score.value()
            );
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

    /// Export reputation data for persistence
    pub fn export(&self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut scores = HashMap::new();
        let mut banned_until = HashMap::new();

        for (peer, score) in &self.scores {
            scores.insert(peer.to_string(), score.value());
        }

        for (peer, expiration) in &self.banned_peers {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + expiration.duration_since(Instant::now()).as_secs();
            banned_until.insert(peer.to_string(), timestamp);
        }

        Ok(serde_json::to_value(ReputationData {
            scores,
            banned_until,
        })?)
    }

    /// Import reputation data from persistence
    pub fn import(&mut self, data: serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
        let data: ReputationData = serde_json::from_value(data)?;

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for (peer_str, score_value) in data.scores {
            if let Ok(peer) = peer_str.parse::<PeerId>() {
                self.scores.insert(peer, ReputationScore(score_value));
            }
        }

        for (peer_str, until_secs) in data.banned_until {
            if until_secs > now_secs {
                if let Ok(peer) = peer_str.parse::<PeerId>() {
                    let duration = Duration::from_secs(until_secs - now_secs);
                    self.banned_peers.insert(peer, Instant::now() + duration);
                }
            }
        }

        Ok(())
    }

    /// Export metrics for monitoring
    pub fn get_metrics(&self) -> serde_json::Value {
        let total_peers = self.scores.len();
        let banned_count = self.banned_peers.len();
        let avg_score: f64 =
            self.scores.values().map(|s| s.value() as f64).sum::<f64>() / total_peers.max(1) as f64;

        serde_json::json!({
            "total_peers": total_peers,
            "banned_peers": banned_count,
            "average_score": avg_score,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_score() {
        let mut score = ReputationScore::new();
        assert_eq!(score.value(), 0);

        score.update(10);
        assert_eq!(score.value(), 10);

        score.update(-60);
        assert_eq!(score.value(), -50); // Clamped to -50
        assert!(score.is_banned());
    }

    #[test]
    fn test_good_behavior() {
        let mut rep = PeerReputation::new();
        let peer = PeerId::random();

        rep.report_good_behavior(peer);
        assert_eq!(rep.get_score(&peer), 1);
        assert!(!rep.is_banned(&peer));
    }

    #[test]
    fn test_bad_behavior_ban() {
        let mut rep = PeerReputation::new();
        let peer = PeerId::random();

        // Report enough bad behavior to get banned
        for _ in 0..60 {
            rep.report_bad_behavior(peer, 1);
        }

        assert!(rep.get_score(&peer) <= -50);
        assert!(rep.is_banned(&peer));
    }

    #[test]
    fn test_export_import() {
        let mut rep = PeerReputation::new();
        let peer = PeerId::random();

        rep.report_good_behavior(peer);
        rep.report_bad_behavior(peer, 5);

        let exported = rep.export().unwrap();

        let mut rep2 = PeerReputation::new();
        rep2.import(exported).unwrap();

        assert_eq!(rep2.get_score(&peer), rep.get_score(&peer));
    }
}
