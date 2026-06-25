# Governance

On-chain governance, staking, delegation, slashing, treasury, and tokenomics.

## Components

| Module | Description |
|---|---|
| `StakingContract` | Register/delegate/undelegate validators, auto-slash at 100 missed blocks, reward distribution |
| `InflationSchedule` | Halving-based block rewards (64 halvings, ~4yr intervals at 6s blocks), 50% fee burn, 10% treasury |
| `Governance` | Proposal lifecycle (create/vote/tally/execute), validator-only proposals, simple majority |
| `GovernanceExecutor` | Parameter changes, validator set updates, treasury spend, runtime upgrades, inflation adjustment |
| `Treasury` | Deposit/spend with balance checks |
| `Slashing` | Double-sign (5%), downtime (0.1%), invalid state transition (10%) penalties |
| `Delegation` | Delegator/validator/rewards tracking with commission rates (0–100%) |
| `UnbondingRequest` | Timelocked unstaking |

## Tests

`tests/economic_tests.rs` — 14 integration tests covering inflation, staking, delegation, slashing, rewards, treasury.
