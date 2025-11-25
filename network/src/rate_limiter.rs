use libp2p::PeerId;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Configuration for rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub transactions_per_second: u32,
    pub block_requests_per_second: u32,
    pub consensus_msgs_per_second: u32,
    pub ban_duration_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            transactions_per_second: 10,
            block_requests_per_second: 5,
            consensus_msgs_per_second: 20,
            ban_duration_secs: 300, // 5 minutes
        }
    }
}

/// Type of message for rate limiting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageType {
    Transaction,
    BlockRequest,
    ConsensusMessage,
}

/// Token bucket for a single peer
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: u32, refill_rate: u32) -> Self {
        Self {
            tokens: capacity as f64,
            capacity: capacity as f64,
            refill_rate: refill_rate as f64,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        
        // Add tokens based on elapsed time
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }

    fn try_consume(&mut self, cost: f64) -> bool {
        self.refill();
        
        if self.tokens >= cost {
            self.tokens -= cost;
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    fn available_tokens(&self) -> f64 {
        self.tokens
    }
}

/// Banned peer information
#[derive(Debug, Clone)]
struct BannedPeer {
    until: Instant,
    reason: String,
    violation_count: u32,
}

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    config: RateLimitConfig,
    buckets: HashMap<(PeerId, MessageType), TokenBucket>,
    banned_peers: HashMap<PeerId, BannedPeer>,
    violation_counts: HashMap<PeerId, u32>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            buckets: HashMap::new(),
            banned_peers: HashMap::new(),
            violation_counts: HashMap::new(),
        }
    }

    /// Check if a peer can send a message and consume tokens if allowed
    pub fn check_and_consume(&mut self, peer: &PeerId, msg_type: MessageType) -> Result<(), String> {
        // Check if peer is banned
        if let Some(ban_info) = self.banned_peers.get(peer) {
            if Instant::now() < ban_info.until {
                return Err(format!(
                    "Peer is banned until {:?}. Reason: {}",
                    ban_info.until, ban_info.reason
                ));
            } else {
                // Ban expired, remove it
                self.banned_peers.remove(peer);
            }
        }

        // Get or create token bucket for this peer and message type
        let capacity = self.get_capacity(msg_type);
        let refill_rate = self.get_refill_rate(msg_type);
        
        let bucket = self.buckets
            .entry((*peer, msg_type))
            .or_insert_with(|| TokenBucket::new(capacity, refill_rate));

        // Try to consume a token
        if bucket.try_consume(1.0) {
            Ok(())
        } else {
            // Rate limit exceeded
            self.record_violation(peer, msg_type);
            Err(format!("Rate limit exceeded for {:?}", msg_type))
        }
    }

    /// Record a rate limit violation
    fn record_violation(&mut self, peer: &PeerId, msg_type: MessageType) {
        let count = self.violation_counts.entry(*peer).or_insert(0);
        *count += 1;

        // Ban peer after 10 violations
        if *count >= 10 {
            self.ban_peer(
                peer,
                Duration::from_secs(self.config.ban_duration_secs),
                format!("Repeated rate limit violations ({:?})", msg_type),
            );
        }
    }

    /// Manually ban a peer
    pub fn ban_peer(&mut self, peer: &PeerId, duration: Duration, reason: String) {
        let until = Instant::now() + duration;
        
        let ban_info = self.banned_peers.entry(*peer).or_insert(BannedPeer {
            until,
            reason: reason.clone(),
            violation_count: 0,
        });
        
        ban_info.until = until;
        ban_info.reason = reason;
        ban_info.violation_count += 1;
        
        tracing::warn!("Banned peer {:?} until {:?}", peer, until);
    }

    /// Check if a peer is currently banned
    pub fn is_banned(&mut self, peer: &PeerId) -> bool {
        if let Some(ban_info) = self.banned_peers.get(peer) {
            if Instant::now() < ban_info.until {
                true
            } else {
                // Ban expired
                self.banned_peers.remove(peer);
                false
            }
        } else {
            false
        }
    }

    /// Get the list of currently banned peers
    pub fn get_banned_peers(&self) -> Vec<PeerId> {
        let now = Instant::now();
        self.banned_peers
            .iter()
            .filter(|(_, ban)| now < ban.until)
            .map(|(peer, _)| *peer)
            .collect()
    }

    /// Unban a peer
    pub fn unban_peer(&mut self, peer: &PeerId) {
        self.banned_peers.remove(peer);
        self.violation_counts.remove(peer);
        tracing::info!("Unbanned peer {:?}", peer);
    }

    /// Clean up expired bans and old buckets
    pub fn cleanup(&mut self) {
        let now = Instant::now();
        
        // Remove expired bans
        self.banned_peers.retain(|_, ban| now < ban.until);
        
        // Remove old token buckets (not used in last 5 minutes)
        self.buckets.retain(|_, bucket| {
            now.duration_since(bucket.last_refill) < Duration::from_secs(300)
        });
    }

    /// Get statistics
    pub fn get_stats(&self) -> RateLimiterStats {
        RateLimiterStats {
            active_buckets: self.buckets.len(),
            banned_peers: self.banned_peers.len(),
            total_violations: self.violation_counts.values().sum(),
        }
    }

    // Helper methods
    fn get_capacity(&self, msg_type: MessageType) -> u32 {
        match msg_type {
            MessageType::Transaction => self.config.transactions_per_second * 2, // 2 second burst
            MessageType::BlockRequest => self.config.block_requests_per_second * 2,
            MessageType::ConsensusMessage => self.config.consensus_msgs_per_second * 2,
        }
    }

    fn get_refill_rate(&self, msg_type: MessageType) -> u32 {
        match msg_type {
            MessageType::Transaction => self.config.transactions_per_second,
            MessageType::BlockRequest => self.config.block_requests_per_second,
            MessageType::ConsensusMessage => self.config.consensus_msgs_per_second,
        }
    }
}

/// Statistics about rate limiting
#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    pub active_buckets: usize,
    pub banned_peers: usize,
    pub total_violations: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_token_bucket_basic() {
        let mut bucket = TokenBucket::new(10, 5);
        
        // Should be able to consume up to capacity
        for _ in 0..10 {
            assert!(bucket.try_consume(1.0));
        }
        
        // Should fail when empty
        assert!(!bucket.try_consume(1.0));
    }

    #[test]
    fn test_token_bucket_refill() {
        let mut bucket = TokenBucket::new(10, 10); // 10 tokens/sec
        
        // Consume all tokens
        for _ in 0..10 {
            assert!(bucket.try_consume(1.0));
        }
        assert!(!bucket.try_consume(1.0));
        
        // Wait for refill
        thread::sleep(Duration::from_millis(500)); // 0.5 seconds = 5 tokens
        
        // Should have ~5 tokens now
        assert!(bucket.try_consume(1.0));
        assert!(bucket.try_consume(1.0));
    }

    #[test]
    fn test_rate_limiter_basic() {
        let config = RateLimitConfig::default();
        let mut limiter = RateLimiter::new(config);
        let peer = PeerId::random();
        
        // Should allow up to capacity
        for _ in 0..20 { // 2x transactions_per_second
            assert!(limiter.check_and_consume(&peer, MessageType::Transaction).is_ok());
        }
        
        // Should fail when limit exceeded
        assert!(limiter.check_and_consume(&peer, MessageType::Transaction).is_err());
    }

    #[test]
    fn test_rate_limiter_different_types() {
        let config = RateLimitConfig::default();
        let mut limiter = RateLimiter::new(config);
        let peer = PeerId::random();
        
        // Each message type has independent limits
        assert!(limiter.check_and_consume(&peer, MessageType::Transaction).is_ok());
        assert!(limiter.check_and_consume(&peer, MessageType::BlockRequest).is_ok());
        assert!(limiter.check_and_consume(&peer, MessageType::ConsensusMessage).is_ok());
    }

    #[test]
    fn test_peer_banning() {
        let config = RateLimitConfig::default();
        let mut limiter = RateLimiter::new(config);
        let peer = PeerId::random();
        
        // Ban the peer
        limiter.ban_peer(&peer, Duration::from_secs(1), "Test ban".to_string());
        
        // Should be banned
        assert!(limiter.is_banned(&peer));
        assert!(limiter.check_and_consume(&peer, MessageType::Transaction).is_err());
        
        // Wait for ban to expire
        thread::sleep(Duration::from_secs(2));
        
        // Should no longer be banned
        assert!(!limiter.is_banned(&peer));
    }

    #[test]
    fn test_violation_auto_ban() {
        let config = RateLimitConfig::default();
        let mut limiter = RateLimiter::new(config);
        let peer = PeerId::random();
        
        // Exhaust tokens
        for _ in 0..20 {
            let _ = limiter.check_and_consume(&peer, MessageType::Transaction);
        }
        
        // Trigger violations
        for _ in 0..10 {
            let _ = limiter.check_and_consume(&peer, MessageType::Transaction);
        }
        
        // Should be auto-banned after 10 violations
        assert!(limiter.is_banned(&peer));
    }

    #[test]
    fn test_cleanup() {
        let config = RateLimitConfig::default();
        let mut limiter = RateLimiter::new(config);
        let peer = PeerId::random();
        
        // Create some buckets
        let _ = limiter.check_and_consume(&peer, MessageType::Transaction);
        
        // Ban a peer temporarily
        limiter.ban_peer(&peer, Duration::from_millis(100), "Test".to_string());
        
        assert_eq!(limiter.get_banned_peers().len(), 1);
        
        // Wait for ban to expire
        thread::sleep(Duration::from_millis(200));
        
        // Cleanup should remove expired ban
        limiter.cleanup();
        assert_eq!(limiter.get_banned_peers().len(), 0);
    }
}
