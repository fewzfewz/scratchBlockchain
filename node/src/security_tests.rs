use std::sync::Arc;

#[tokio::test]
async fn test_rate_limiting() {
    use governor::{Quota, RateLimiter};
    use governor::clock::DefaultClock;
    use governor::state::keyed::DefaultKeyedStateStore;
    use std::num::NonZeroU32;
    use std::net::IpAddr;

    let rate_limiter = Arc::new(RateLimiter::<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>::keyed(
        Quota::per_second(NonZeroU32::new(10).unwrap()),
    ));

    let ip: IpAddr = "127.0.0.1".parse().unwrap();

    for _ in 0..10 {
        assert!(rate_limiter.check_key(&ip).is_ok());
    }

    assert!(rate_limiter.check_key(&ip).is_err());
}

#[test]
fn test_peer_reputation() {
    use libp2p::PeerId;
    use network::reputation::PeerReputation;

    let mut reputation = PeerReputation::new();
    let peer = PeerId::random();

    assert_eq!(reputation.get_score(&peer), 0);
    reputation.report_good_behavior(peer);
    assert_eq!(reputation.get_score(&peer), 1);
    reputation.report_bad_behavior(peer, 10);
    assert_eq!(reputation.get_score(&peer), -9);
    reputation.report_bad_behavior(peer, 50);
    assert!(reputation.is_banned(&peer));
}
