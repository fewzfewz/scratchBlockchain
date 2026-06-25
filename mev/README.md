# MEV — Maximal Extractable Value

MEV protection system with commit-reveal, Proposer-Builder Separation (PBS), auctions, and threshold-encrypted mempool.

## Components

| Component | Description |
|---|---|
| `CommitRevealScheme` | Commit (hash), reveal after configurable delay — front-running protection |
| `BlockBuilder` / `BuildStrategy` | MEV-aware block construction: gas maximization, MEV extraction, balanced, user priority |
| `MEVAuction` | Sealed-bid auction where builders compete by bidding extracted MEV value |
| `ThresholdEncryption` | Shamir's Secret Sharing over GF(2⁸) for private encrypted mempool |
| `MevManager` | Orchestrates all subsystems |
