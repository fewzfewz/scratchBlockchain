# Governance

On-chain governance, staking, delegation, slashing, treasury, and tokenomics.

## Components

| Module | Description |
|---|---|
| `ChainGovernance` | Persisted on-chain state (proposals, votes, treasury) — keyed `b"governance"` in state trie |
| `StakingContract` | Register/delegate/undelegate validators, slashing, reward distribution |
| `InflationSchedule` | Halving-based block rewards, 50% fee burn, 10% treasury |
| `GovernanceExecutor` | Parameter changes, validator set updates, treasury spend, runtime upgrades |
| `Treasury` | Deposit/spend with balance checks |
| `Slashing` | Double-sign, downtime, invalid state penalties |
| `Delegation` | Delegator/validator tracking with commission rates |

## Node Integration (August 2026)

| RPC / Module | Purpose |
|--------------|---------|
| `GET /governance` | Full on-chain governance state |
| `GET /proposal/{id}` | Single proposal with live status |
| `POST /delegate` | Delegate stake (`governance_store::apply_delegate`) |
| `POST /validators/register` | Add validator to dynamic set |
| `governance_store.rs` | Trie wiring, treasury fee collection on block finalize |
| Governance txs in blocks | `apply_extrinsics` for `vote` / `propose` payloads |

## Tests

`tests/economic_tests.rs` — integration tests covering inflation, staking, delegation, slashing, rewards, treasury.
