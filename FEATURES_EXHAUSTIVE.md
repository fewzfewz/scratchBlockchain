# Exhaustive Feature List

Based on actual source code analysis (not documentation claims).

---

## `common` crate (types & crypto)

| Feature | Struct/Trait/File | Details |
|---|---|---|
| Block | `Block` (`types.rs:68`) | Header + extrinsics + hash() |
| Header | `Header` (`types.rs:45`) | slot, parent_hash, state_root, extrinsics_root, proposer, signature |
| Transaction | `Transaction` (`types.rs:114`) | sender, nonce, payload, signature, gas_limit, max_fee_per_gas, max_priority_fee_per_gas |
| Receipt | `TransactionReceipt` (`types.rs:171`) | tx_hash, block_hash, block_height, gas_used/limit, status, from, to |
| Account | `Account` (`types.rs:209`) | nonce, balance |
| GenesisConfig | `GenesisConfig` (`types.rs:233`) | accounts list, validators list |
| EIP-1559 fields | in `Transaction` | max_fee_per_gas, max_priority_fee_per_gas |
| Consensus types | `consensus_types.rs` | Vote (round, hash, validator, sig), Proposal, ConsensusMessage (enum), Step (Propose/Prevote/Precommit), ValidatorInfo |
| Consensus trait | `Consensus` trait (`traits.rs`) | start_round, validate_proposal, create_proposal, process_vote, etc. |
| Storage trait | `Storage` trait (`traits.rs`) | get/put/contains |
| Executor trait | `Executor` trait (`traits.rs`) | execute_transaction, execute_block |
| Mempool trait | `Mempool` trait (`traits.rs`) | submit, get_batch, remove |
| BlockProducer trait | `BlockProducer` trait (`traits.rs`) | produce_block |
| MerkleTree | `MerkleTree` (`merkle.rs`) | new, root, proof, verify, batch_verify |
| MerkleProof | `MerkleProof` (`merkle.rs`) | single + multi proof verification |
| SigningKey | `SigningKey` (`crypto.rs`) | generate, sign, verify, public_key (Ed25519) |
| TransactionValidator | `TransactionValidator` (`validation.rs`) | validate (chain ID, gas, nonce, signature, balance) |

## `consensus` crate (BFT + finality)

| Feature | Struct/Trait/File | Details |
|---|---|---|
| BftEngine | `BftEngine` (`bft.rs`) | propose/prevote/precommit rounds, view-change, lock/unlock, timeout config |
| ConsensusMessage | `EnhancedConsensus` (`lib.rs`) | message types for network |
| FinalityGadget | `FinalityGadget` (`lib.rs`) | finalize rounds, change sets, skip proposals |
| SlashingCondition | `SlashingCondition` enum (`lib.rs`) | double-sign, invalid state transition |
| Validator power | `validator_power` fn (`lib.rs`) | calculate voting power |
| SlashingTracker | `SlashingTracker` (`slashing.rs`) | track missed blocks, double-sign evidence |
| SlashingConfig | `SlashingConfig` (`slashing.rs`) | slash percentages, thresholds |
| Fuzz targets | (`consensus/fuzz/`) | fuzzing for BFT libp2p messages |

## `network` crate (P2P)

| Feature | Struct/Trait/File | Details |
|---|---|---|
| NetworkService | `NetworkService` (`lib.rs`) | gossipsub (tx/block/consensus topics), Kademlia DHT, request-response sync |
| Gossip topics | in `lib.rs` | TX_TOPIC, BLOCK_TOPIC, CONSENSUS_TOPIC |
| Connection limits | in `lib.rs` | max peers, incoming/outgoing limits |
| RateLimiter | `RateLimiter` (`rate_limiter.rs`) | per-peer token buckets, auto-ban, 3 message types |
| PeerReputation | `PeerReputation` (`reputation.rs`) | score -100 to 100, persistence to disk, ban at -50 |
| PeerStore | `PeerStore` (`peer_store.rs`) | save/load peers from disk, periodic re-dial |
| NetworkProtocol | `NetworkProtocol` (`protocol.rs`) | block request/response codec |
| NodeBehaviour | `NodeBehaviour` (`behaviour.rs`) | composite libp2p NetworkBehaviour |
| Transport | `build_transport` (`transport.rs`) | TCP + noise + yamux |
| SyncManager | `SyncManager` (`sync.rs`) | block range sync from peers, batch download + process |
| SyncConfig | `SyncConfig` (`sync.rs`) | sync mode (full, header, light) |

## `storage` crate (persistence)

| Feature | Struct/Trait/File | Details |
|---|---|---|
| KeyValueStore | `KeyValueStore` trait (`db.rs`) | get/put/delete/contains/write_batch/iter/scan/flush |
| RocksDB | `RocksDb` (`db.rs`) | ColumnFamily (Blocks, BlockHeights, State, Receipts, Meta), WriteBatch, atomic commit |
| MemDb | `MemDb` (`db.rs`) | in-memory with cache + metrics, for testing |
| ChainStore | `ChainStore` (`db.rs`) | put_block, get_block, get_latest_height, set_latest_height, get_block_by_height, block_exists |
| WriteBatch | `WriteBatch` (`db.rs`) | atomic batch with put/delete |
| DbMetrics | `DbMetrics` (`db.rs`) | reads/writes/deletes, latencies, cache hit rate |
| DbError | `DbError` (`db.rs`) | typed errors |
| MerklePatriciaTrie | `MerklePatriciaTrie` (`trie.rs`) | put/get/contains/root_hash, proof generation, extension/branch/leaf/empty nodes |
| Nibble types | `Nibble`, `NibblePath`, `NodeHash` (`trie.rs`) | hex-nibble trie paths |
| ReceiptStore | `ReceiptStore` (`receipt_store.rs`) | put_receipt/get_receipt/has_receipt (sled-backed) |
| MemStore | `MemStore` (`lib.rs`) | Storage trait impl over MemDb |
| PersistentStore | `PersistentStore` (`lib.rs`) | Storage trait impl over sled (secondary backend) |
| StateStore | `StateStore` (`lib.rs`) | account CRUD, root_hash, initialize_genesis (sled-backed) |
| TrieStateStore | `TrieStateStore` (`lib.rs`) | account CRUD via Patricia Trie, proof generation |
| BlockStore | `BlockStore` (`lib.rs`) | block persistence (sled-backed) |

## `execution` crate

| Feature | Struct/Trait/File | Details |
|---|---|---|
| NativeExecutor | `NativeExecutor` (`lib.rs`) | native transaction execution |
| EvmExecutor | `EvmExecutor` (`evm.rs`) | revm-based EVM, deploy_call, execute_transaction, persistent state via EvmStore trait |
| EvmStore trait | `EvmStore` trait (`evm.rs`) | get/put account, storage, code |
| InMemoryStore | `InMemoryStore` (`evm.rs`) | in-memory EvmStore impl |
| StoredAccount | `StoredAccount` (`evm.rs`) | balance, nonce, code_hash |
| WasmExecutor | `WasmExecutor` (`lib.rs`) | placeholder (stub) |
| ParallelExecutor | `ParallelExecutor` (`lib.rs`) | rayon-based parallel tx execution |
| UserOperation | `UserOperation` (`account_abstraction.rs`) | ERC-4337 style, hash(), to_transaction() |
| AccountAbstraction | `AccountAbstraction` (`account_abstraction.rs`) | handle_user_ops, verify paymaster, entry point logic |
| GasCosts | `GasCosts` (`gas.rs`) | Ethereum-style gas costs (SLOAD, SSTORE, CALL, CREATE, etc.) |
| GasMeter | `GasMeter` (`gas.rs`) | gas_limit, gas_used, gas_refund, EIP-1559 base fee calc |
| EIP-1559 gas pricing | `GasMeter` (`gas.rs`) | calculate_base_fee, MaxPriorityFee, MaxFeePerGas |

## `mempool` crate

| Feature | Struct/Trait/File | Details |
|---|---|---|
| Mempool | `Mempool` (`lib.rs`) | BinaryHeap priority queue, per-sender limits, validation, eviction, get_batch |
| MempoolConfig | `MempoolConfig` (`lib.rs`) | max_size, max_tx_size, min_fee_per_gas, chain_id |
| MempoolMEV | `MempoolMEV` (`mev_protection.rs`) | commit-reveal scheme, private mempool |

## `governance` crate

| Feature | Struct/Trait/File | Details |
|---|---|---|
| InflationSchedule | `InflationSchedule` (`lib.rs`) | halving mechanism, fee burn %, calculate_reward |
| Delegation | `Delegation` (`lib.rs`) | delegator, validator, amount, rewards, created_at |
| ValidatorMetadata | `ValidatorMetadata` (`lib.rs`) | commission, total_delegated, delegator_count, blocks produced/missed |
| UnbondingRequest | `UnbondingRequest` (`lib.rs`) | delayed unstaking with completion_height |
| SlashingReason | `SlashingReason` enum (`lib.rs`) | DoubleSign (5%), Downtime (0.1%), InvalidStateTransition (10%) |
| SlashingEvent | `SlashingEvent` (`lib.rs`) | record of slashing |
| Treasury | `Treasury` (`lib.rs`) | deposit, spend, balance tracking |
| StakingContract | `StakingContract` (`lib.rs`) | register_validator, delegate, undelegate, process_unbonding, slash, distribute_rewards, record_missed_block, get_active_validators |
| Proposal | `Proposal` (`lib.rs`) | id, proposer, type, voting period, yes/no votes, status |
| ProposalType | `ProposalType` enum (`lib.rs`) | ParameterChange, SoftwareUpgrade, TextProposal |
| ProposalStatus | `ProposalStatus` enum (`lib.rs`) | Active, Passed, Rejected, Executed |
| Governance | `Governance` (`lib.rs`) | create_proposal, vote, tally_votes |
| GovernanceAction | `GovernanceAction` enum (`lib.rs`) | SetParameter, UpdateValidatorSet, TreasurySpend, RuntimeUpgrade, UpdateInflation |
| GovernanceExecutor | `GovernanceExecutor` (`lib.rs`) | execute approved actions, set_parameter, update_validator_set |

## `mev` crate

| Feature | Struct/Trait/File | Details |
|---|---|---|
| CommitRevealScheme | `CommitRevealScheme` (`lib.rs`) | commit/reveal, min delay, max age, cleanup expired |
| TransactionCommitment | `TransactionCommitment` (`lib.rs`) | commitment hash, height, sender, nonce |
| RevealedTransaction | `RevealedTransaction` (`lib.rs`) | tx + secret + commitment |
| BuilderBid | `BuilderBid` (`lib.rs`) | builder pubkey, block, bid_amount, signature, tx_root, timestamp, mev_value |
| BlockBuilder | `BlockBuilder` (`lib.rs`) | build blocks with strategy (GasMaximization, MevExtraction, Balanced, UserPriority), select_transactions, calculate_mev |
| BuildStrategy | `BuildStrategy` enum (`lib.rs`) | 4 strategies |
| BuilderPerformance | `BuilderPerformance` (`lib.rs`) | blocks_built, bids, avg_bid, total_mev |
| MEVAuction | `MEVAuction` (`lib.rs`) | submit_bid, select_winner, verify_bid_signature (Ed25519), history |
| AuctionStats | `AuctionStats` (`lib.rs`) | total_auctions, avg_winning_bid, total_mev_extracted |
| EncryptedTransaction | `EncryptedTransaction` (`lib.rs`) | encrypted_data, nonce, threshold, validator_set_id |
| DecryptionShare | `DecryptionShare` (`lib.rs`) | nonce, validator pubkey, share, signature |
| ThresholdEncryption | `ThresholdEncryption` (`lib.rs`) | encrypt/submit/decrypt, threshold-based decryption, Ed25519 signature verification |
| MevManager | `MevManager` (`lib.rs`) | orchestrates commit-reveal + auction + threshold encryption |
| MevStats | `MevStats` (`lib.rs`) | aggregated MEV statistics |

## `zk` crate (zero-knowledge proofs)

| Feature | Struct/Trait/File | Details |
|---|---|---|
| StateTransitionConfig | `StateTransitionConfig` (`lib.rs`) | Halo2 circuit configuration |
| StateTransitionCircuit | `StateTransitionCircuit` (`lib.rs`) | Halo2 circuit impl, state transition gate |
| AggregatedProof | `AggregatedProof` (`lib.rs`) | multiple proofs with aggregate root |
| ProofAggregator | `ProofAggregator` (`lib.rs`) | batch proof aggregation |
| ZkProver | `ZkProver` (`lib.rs`) | prove_state_transition, verify_state_transition, verify_aggregated, KZG params + cache |
| ParamsKZG setup | in `ZkProver::new()` | Bn256 KZG trusted setup (20 degree) |

## `da` crate (data availability)

| Feature | Struct/Trait/File | Details |
|---|---|---|
| KzgCommitment | `KzgCommitment` (`lib.rs`) | simplified SHA256-based commitment (placeholder for real KZG) |
| DataBlob | `DataBlob` (`lib.rs`) | data + commitment + index, self-verify |
| ErasureChunk | `ErasureChunk` (`lib.rs`) | chunk data, index, total, Merkle proof |
| ErasureCoder | `ErasureCoder` (`lib.rs`) | XOR-based encode/decode, configurable data+parity chunks |
| AvailabilitySampler | `AvailabilitySampler` (`lib.rs`) | sample chunks, verify availability ratio |
| DataAvailability | `DataAvailability` (`lib.rs`) | submit_blob, get_blob, encode_blob, verify_availability |
| DaLightClient | `DaLightClient` (`lib.rs`) | trusted roots, sample_availability, verify_blob |
| AvailabilityProof | `AvailabilityProof` (`lib.rs`) | sample results + Merkle proofs |
| AvailabilityProver | `AvailabilityProver` (`lib.rs`) | generate proof from chunks |

## `rollup` crate

| Feature | Struct/Trait/File | Details |
|---|---|---|
| Batch | `Batch` (`lib.rs`) | transactions, state roots, optional ZK proof + DA commitment |
| FraudProof | `FraudProof` (`lib.rs`) | batch_index, tx_index, invalid/correct state roots, evidence, challenger |
| RollupType | `RollupType` enum (`lib.rs`) | Optimistic, ZkRollup |
| RollupNode | `RollupNode` (`lib.rs`) | submit_batch, verify_batch, generate_zk_proof, batch management |
| FraudVerifier | `FraudVerifier` (`lib.rs`) | verify_fraud_proof by re-executing |
| CrossRollupMessage | `CrossRollupMessage` (`lib.rs`) | inter-rollup message with optional proof |
| RollupBridge | `RollupBridge` (`lib.rs`) | send/receive/execute cross-rollup messages |
| Unit tests | 4 tests in `lib.rs` | optimistic, ZK, ZK verification, DA integration |

## `interop` crate (bridges)

| Feature | Struct/Trait/File | Details |
|---|---|---|
| CrossChainMessage | `CrossChainMessage` (`lib.rs`) | source/dest chain, nonce, sender, recipient, amount, payload |
| BridgeContract | `BridgeContract` (`lib.rs`) | lock_assets, unlock_assets, relayer management |
| EthereumBridge | `EthereumBridge` (`ethereum_bridge.rs`) | lock_tokens, unlock_tokens, verify_signatures, add/remove_relayer, is_processed |
| BridgeMessage | `BridgeMessage` (`ethereum_bridge.rs`) | full bridge message with signatures |
| Relayer | `Relayer` (`relayer.rs`) | chain_a ↔ chain_b relay loop (tokio) |
| TokenInfo | `TokenInfo` (`token_registry.rs`) | symbol, name, decimals, addresses, limits, enabled |
| TokenRegistry | `TokenRegistry` (`token_registry.rs`) | add/get/is_supported/set_enabled/validate_amount, default tokens (USDC, USDT, ETH) |

## `runtime` crate

| Feature | Struct/Trait/File | Details |
|---|---|---|
| RuntimeVersion | `RuntimeVersion` (`upgrade/version.rs`) | major/minor/patch/spec/impl, is_compatible, can_upgrade_to |
| RuntimeMetadata | `RuntimeMetadata` (`upgrade/version.rs`) | version, code_hash, activated_at, description |
| StateSnapshot | `StateSnapshot` (`upgrade/snapshot.rs`) | id, version, block, state_root, compressed_state |
| SnapshotManager | `SnapshotManager` (`upgrade/snapshot.rs`) | create/restore/get/prune snapshots |
| StateMigration trait | `StateMigration` trait (`upgrade/migration.rs`) | migrate, validate |
| MigrationPlan | `MigrationPlan` (`upgrade/migration.rs`) | from/to version, migration names, estimated duration |
| StateMigrator | `StateMigrator` (`upgrade/migration.rs`) | register/execute migrations |
| UpgradeValidator | `UpgradeValidator` (`upgrade/validator.rs`) | validate_upgrade (version, code hash, resource), validate_post_upgrade |
| UpgradeCoordinator | `UpgradeCoordinator` (`upgrade/coordinator.rs`) | schedule/execute/rollback/cancel upgrades, snapshot + migration + validation |
| PendingUpgrade | `PendingUpgrade` (`upgrade/coordinator.rs`) | id, versions, code_hash, activation_block, state, migration_plan |
| UpgradeState | `UpgradeState` enum (`upgrade/coordinator.rs`) | Proposed, Scheduled, InProgress, Completed, Failed, Cancelled |
| UpgradeError | `UpgradeError` enum (`upgrade/coordinator.rs`) | typed errors for upgrade operations |

## `monitoring` crate

| Feature | Struct/Trait/File | Details |
|---|---|---|
| MetricsServer | `MetricsServer` (`lib.rs`) | axum-based HTTP server, `/metrics` + `/health` |
| BlockchainMetrics | `BlockchainMetrics` (`metrics.rs`) | 18 Prometheus metrics: block_height, block_time, blocks_produced, tx_total, tx_pending, tx_processing_time, validator_count, validator_stake_total, missed_blocks, peer_count, network_bytes sent/recv, consensus_rounds/time, finality_time, state_size_bytes, db read/write time |

## `node` crate (binary)

| Feature | Struct/Trait/File | Details |
|---|---|---|
| CLI (clap) | `Cli` / `Commands` enum (`main.rs`) | 8 subcommands: Start, KeyGen, SubmitTx, QueryBalance, GetBlock, ConnectPeer, Faucet, Status |
| Node wiring | `Node` struct (`main.rs`) | wires all 16+ components together |
| BftPersistedState | `BftPersistedState` (`main.rs`) | crash recovery for BFT state |
| NodeStats | `NodeStats` (`main.rs`) | throughput, blocks/tx processed |
| Event loop | `Node::run()` (`main.rs`) | tokio select: BFT events, network events, timeouts, ctrl-c shutdown |
| Genesis loading | in `Node::new()` | parse genesis.json, initialize accounts + genesis block |
| Key generation | `load_or_generate_key()` (`main.rs`) | Ed25519 key pair, JSON persistence |
| NodeConfig | `NodeConfig` (`config.rs`) | TOML config with 9 sections (network, consensus, validator, storage, api, metrics, logging, security) |
| NetworkConfig | `NetworkConfig` (`config.rs`) | chain_id, p2p/rpc/ws ports, bootstrap nodes, max_peers, timeout |
| ConsensusConfig | `ConsensusConfig` (`config.rs`) | block_time_ms, epoch_length, max_validators |
| BlockProducerConfig | `BlockProducerConfig` (`config.rs:28`) | max_tx_per_block, max_gas, target_utilization |
| BlockExecutor | `BlockExecutor` (`block_producer.rs`) | execute_and_commit: EVM → atomic WriteBatch |
| BlockProducer | `BlockProducer` (`block_producer.rs`) | produce block from mempool, submit to BFT, execute on finalize |
| RewardConfig | `RewardConfig` (`rewards.rs`) | base_block_reward, proposer_fee %, vote_reward, slash_penalty |
| RewardCalculator | `RewardCalculator` (`rewards.rs`) | calculate_block_rewards, track blocks/votes per validator |
| RewardManager | `RewardManager` (`rewards.rs`) | distribute_rewards, apply_slash, apply_inflation |
| SyncManager | re-export from `network::sync` (`sync.rs`) | block sync |
| CircuitBreaker | `CircuitBreaker` (`circuit_breaker.rs`) | 3 states (Closed/Open/HalfOpen), failure_threshold, auto-recovery |
| ForkChoice | `ForkChoice` (`fork_choice.rs`) | Accept / Reorg decisions, heaviest chain selection |
| SyncCommittee | `SyncCommittee` (`light_client.rs`) | members, aggregate_pubkey |
| LightClientUpdate | `LightClientUpdate` (`light_client.rs`) | attested header, next committee, bits, signature, finality branch |
| LightClientState | `LightClientState` (`light_client.rs`) | current header, sync committee, apply_update with 2/3 threshold |
| LightClient | `LightClient` (`light_client.rs`) | highest chain, get_state |
| RuntimeUpgradeManager | `RuntimeUpgradeManager` (`runtime_upgrade.rs`) | propose/approve/activate upgrades, version tracking |
| UpgradeProposal | `UpgradeProposal` (`runtime_upgrade.rs`) | id, version, code_hash, activation_height, proposer |
| RpcServer | `RpcServer` (`rpc.rs`) | warp HTTP server, 9+ endpoints |
| RPC endpoints | (`rpc.rs`) | health, status, get_block, get_balance, get_mempool, submit_tx, connect_peer, faucet |
| RPC economic endpoints | (`rpc_economic.rs`) | staking, rewards, delegation queries |
| Faucet | `Faucet` (`faucet.rs`) | drip_amount, cooldown, max_requests, request_tokens |
| Metrics | `Metrics` (`metrics.rs`) | node-level metrics tracking |

## `tools/genesis-builder`

| Feature | Struct/Trait/File | Details |
|---|---|---|
| GenesisBuilder | `GenesisBuilder` (`main.rs`) | generate/validate genesis JSON from TOML config |

---

## Notable gaps and blockers

| Issue | Location | Details |
|---|---|---|
| Full MPT state root from EVM | `node/src/block_producer.rs` | Uses deterministic `SHA256(parent \|\| extrinsics)`; not full trie root from EVM diffs |
| BFT hot-reload after register | `node/src/main.rs` | `POST /validators/register` updates trie but not live BFT validator keys |
| ZK circuit simplified | `zk/src/lib.rs:75` | Constraint is `new = prev + tx` instead of a real hash function |
| KZG setup unsafe | `zk/src/lib.rs:176` | `OsRng` instead of a real trusted setup ceremony |
| DA erasure coding XOR-based | `da/src/lib.rs:116-132` | Parity chunks can't recover missing data; real RS not implemented |
| Threshold encryption XOR-based | `mev/src/lib.rs:590-597` | Real Shamir's Secret Sharing not implemented (RPC wired, crypto simplified) |
| Bridge signature verify stubbed | `interop/src/ethereum_bridge.rs:146-148` | Just checks count, not actual crypto |
| Fraud proof re-execution stubbed | `rollup/src/lib.rs:334-337` | `execute_transaction` returns state unchanged |
| Wasm executor stub | `execution/src/lib.rs` | Placeholder; AA (ERC-4337) is wired separately |
| OpenAPI spec lag | `docs/openapi.yaml` | New RPC routes documented in README/STATUS; Swagger update pending |

## Recently integrated (August 2026)

| Feature | Location | Details |
|---|---|---|
| TxPool | `node/src/tx_pool.rs` | MevMempool + AccountAbstractionExecutor |
| Account abstraction RPC | `node/src/rpc.rs` | `POST /submit_user_operation`, `GET /user_operations/pending` |
| MEV RPC | `node/src/rpc.rs` | `/mev/commit`, `/mev/reveal`, `/mev/encrypted`, `/mev/decryption_share` |
| Slashing RPC | `node/src/rpc.rs` | `GET /slashing/events`; tracker in node loop |
| Delegation / register | `node/src/governance_store.rs` | `POST /delegate`, `POST /validators/register` |
| WebSocket | `node/src/rpc.rs` | `GET /ws` — `newHead` events |
