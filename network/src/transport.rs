use libp2p::{core::upgrade::Version, noise, tcp, yamux, Transport};
use std::time::Duration;

pub fn build_transport(
    keypair: &libp2p::identity::Keypair,
) -> std::io::Result<
    libp2p::core::transport::Boxed<(libp2p::PeerId, libp2p::core::muxing::StreamMuxerBox)>,
> {
    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true));

    Ok(tcp_transport
        .upgrade(Version::V1)
        .authenticate(
            noise::Config::new(keypair).expect("signing libp2p-noise static DH keypair failed"),
        )
        .multiplex(yamux::Config::default())
        .timeout(Duration::from_secs(20))
        .boxed())
}
