use libp2p::{core::upgrade::Version, dns, noise, tcp, yamux, Transport};
use std::time::Duration;

pub fn build_transport(
    keypair: &libp2p::identity::Keypair,
) -> std::io::Result<
    libp2p::core::transport::Boxed<(libp2p::PeerId, libp2p::core::muxing::StreamMuxerBox)>,
> {
    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true));
    
    // Wrap with DNS resolution to support /dns4 and /dns6 multiaddrs
    let dns_transport = dns::tokio::Transport::system(tcp_transport)?;

    Ok(dns_transport
        .upgrade(Version::V1)
        .authenticate(
            noise::Config::new(keypair).expect("signing libp2p-noise static DH keypair failed"),
        )
        .multiplex(yamux::Config::default())
        .timeout(Duration::from_secs(20))
        .boxed())
}
