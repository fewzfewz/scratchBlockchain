# MEV — Maximal Extractable Value

MEV protection system with commit-reveal, Proposer-Builder Separation (PBS), auctions, and threshold-encrypted mempool.

## Components

| Component | Description |
|---|---|
| `CommitRevealScheme` | Commit (hash), reveal after configurable delay — front-running protection |
| `BlockBuilder` / `BuildStrategy` | MEV-aware block construction: gas maximization, MEV extraction, balanced, user priority |
| `MEVAuction` | Sealed-bid auction where builders compete by bidding extracted MEV value |
| `ThresholdEncryption` | Encrypted mempool with threshold decryption (simplified XOR in current impl) |
| `MevManager` | Orchestrates all subsystems |

## Node Integration (August 2026)

MEV is **wired into the running node** via `MevMempool` inside `TxPool`:

| RPC | Purpose |
|-----|---------|
| `POST /mev/commit` | Submit transaction commitment |
| `POST /mev/reveal` | Reveal committed transaction into mempool |
| `POST /mev/encrypted` | Submit threshold-encrypted transaction |
| `POST /mev/decryption_share` | Validator submits decryption share |

Block producer includes decrypted + revealed + regular transactions via `get_all_ready_transactions()`.

## Known Limitation

`ThresholdEncryption` in `mev/src/lib.rs` uses a simplified XOR scheme, not full Shamir's Secret Sharing over GF(2⁸). Production deployment should replace with real threshold crypto.
