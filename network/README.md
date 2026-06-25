# Network

Peer-to-peer networking layer using libp2p.

## Components

| Module | Description |
|---|---|
| `src/lib.rs` | `NetworkService` — libp2p `Swarm` with Gossipsub (tx/blocks/consensus topics), Kademlia DHT, request-response protocol |
| `src/behaviour.rs` | `NodeBehaviour` — `#[derive(NetworkBehaviour)]` composing gossipsub, kademlia, request-response, connection_limits |
| `src/transport.rs` | TCP + DNS + Noise encryption + Yamux multiplexing transport |
| `src/rate_limiter.rs` | Token-bucket per-peer rate limiting (tx: 10/s, block: 5/s, consensus: 20/s), auto-ban at 10 violations |
| `src/reputation.rs` | Peer scoring (−100 to +100), ban at −50 (1 hour), score decay toward zero |
| `src/peer_store.rs` | Persistent JSON-backed known peer addresses |
| `src/protocol.rs` | `BlockExchangeProtocol` — length-prefixed JSON block sync codec |
| `src/sync.rs` | Sync manager: Full, Fast (parallel state snapshot), Warp (checkpoint), Light (headers only) modes |

## Features

- 3 Gossipsub topics with strict validation, 64 KB max message, 1s heartbeat
- Max 50 incoming/outgoing connections, max 5 per peer
- Asynchronous event loop via `mpsc` channels (`NetworkCommand` / `NetworkEvent`)
