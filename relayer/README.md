# nebula-relayer

Standalone Ethereum → Nebula bridge relayer. Run one per relayer key in the
genesis relayer set (`bridge.relayers`); a lock is minted on Nebula once
`bridge.threshold` of them have attested to it. No single relayer — and no
RPC node — can mint on its own: consensus checks every attestation.

## How it works

1. **Watch.** Follows `Bridge.sol` with `eth_getLogs` for `TokensLocked`
   events and only considers a lock once it has `ETH_CONFIRMATIONS` blocks.
   Only native-ETH locks are bridged; amount and recipient come from the event.
2. **Attest.** Signs `NEBULA_BRIDGE_MINT_V1 ‖ chain id ‖ ETH tx hash ‖
   recipient ‖ amount` with its Ed25519 key and posts it to every configured
   node (`POST /bridge/attestations`). Nodes keep only attestations with a
   valid signature from a genesis relayer.
3. **Submit.** When `threshold` attestations are pooled
   (`GET /bridge/attestations/<eth tx>`), relayers take turns per lock: the
   first in line submits the `bridge_mint` transaction from its Nebula
   account (`SUBMITTER_KEY`, default the relayer key — it needs NBL for
   fees); the others wait and see it minted. A duplicate mint is rejected by
   consensus (the ETH tx hash is recorded in state).
4. **Resume.** Progress (next block, unminted locks) is saved to
   `RELAYER_STATE_FILE`, so a restarted relayer picks up where it stopped and
   re-posts attestations for locks that are still pending.

## Configuration

| Variable / flag | Default | Meaning |
|---|---|---|
| `ETH_RPC_URL` | `http://127.0.0.1:9545` | Ethereum JSON-RPC |
| `ETH_BRIDGE_ADDRESS` | — | `Bridge.sol` address |
| `NEBULA_RPC_URLS` | `http://127.0.0.1:8545` | Comma-separated Nebula nodes |
| `RELAYER_KEY` | — | Hex Ed25519 seed (must be in the genesis relayer set) |
| `SUBMITTER_KEY` | relayer key | Hex seed of the account paying for mints |
| `NEBULA_CHAIN_ID` | `1` | Chain id bound into attestations |
| `ETH_CONFIRMATIONS` | `2` | Blocks a lock must be buried under |
| `ETH_START_BLOCK` | `0` | First block when there is no saved state (use the bridge's deployment block) |
| `RELAYER_STATE_FILE` | `relayer-state.json` | Progress file |
| `RELAYER_POLL_MS` | `2000` | Poll interval |

```bash
cargo run --release -p nebula-relayer -- --help
```

## Local devnet

`deployment/local/docker-compose.yml` runs `relayer-a` and `relayer-b`; with
the node's dev-mode relayer key the devnet set is 2-of-3. Their keys are
devnet-only (`sha256("nebula-devnet-relayer-a" / "-b")`) and their accounts
are funded in genesis. Hardhat mines one block per transaction, so the devnet
uses `ETH_CONFIRMATIONS=1`.

## Not covered

Nebula → Ethereum unlocks are relayed by `interop/scripts/eth-relayer-daemon.js`
(it holds several relayer keys in one process). Relayer-set changes need a
new genesis.
