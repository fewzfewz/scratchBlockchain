//! Benchmarks of the node's real hot paths.
//!
//! - `execute_block`: a block of signed native transfers through
//!   `BlockExecutor::execute_and_commit` on the on-disk store, staged exactly
//!   as the node stages a block, including the LtHash over the changed state
//!   (the state-root work). Throughput is reported in transactions/second.
//! - `mempool_admit`: `TxPool::add_transaction`, which verifies each signature.
//! - `commit_certificate_verify`: checking a block's +2/3 precommits (what
//!   sync, proposal validation and light clients do per block).
//! - `bft_height`: CPU cost of one BFT height across N in-process validators
//!   (proposal, prevotes, precommits, all signed and verified), i.e. the
//!   lower bound on finality latency before network delay.

use common::consensus_types::{CommitCertificate, ConsensusMessage, Step, ValidatorInfo, Vote};
use common::crypto::SigningKey;
use common::types::{Block, Header, Transaction};
use consensus::bft::{BftEngine, BftEvent};
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use execution::evm::EvmExecutor;
use mempool::MempoolConfig;
use node::block_producer::BlockExecutor;
use node::evm_store::ChainStoreEvmStore;
use node::tx_pool::TxPool;
use std::collections::VecDeque;
use std::sync::Arc;
use storage::{ChainStore, KeyValueStore, StagedStore};

const CHAIN_ID: u64 = 1;

fn signed_transfers(n: usize, nonce: u64) -> Vec<Transaction> {
    (0..n)
        .map(|i| {
            let mut seed = [7u8; 32];
            seed[..8].copy_from_slice(&(i as u64).to_le_bytes());
            let key = ed25519_dalek::SigningKey::from_bytes(&seed);
            let mut tx = Transaction::simple_transfer([0; 20], [9; 20], 1, nonce, CHAIN_ID);
            tx.max_priority_fee_per_gas = 1_000_000_000;
            tx.sign_with(&key);
            tx
        })
        .collect()
}

fn execute_block(c: &mut Criterion) {
    let mut group = c.benchmark_group("execute_block");
    group.sample_size(10);
    for n in [100usize, 1_000, 5_000] {
        let dir = tempfile::tempdir().unwrap();
        let base = ChainStore::open(dir.path()).unwrap().inner().clone();
        let staged = Arc::new(StagedStore::new(base));
        let kv: Arc<dyn KeyValueStore> = staged.clone();
        let chain_store = Arc::new(ChainStore::new(kv));
        let txs = signed_transfers(n, 0);
        for tx in &txs {
            node::accounts::store(&chain_store, &tx.sender, u128::MAX / 4, 0).unwrap();
        }
        let evm = EvmExecutor::with_store(
            Arc::new(ChainStoreEvmStore::new(chain_store.clone())),
            CHAIN_ID,
        );
        let mut executor = BlockExecutor::new(evm, chain_store);
        let block = Block::new(Header::new([0; 32], 1), txs);

        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &block, |b, block| {
            b.iter(|| {
                // Stage, execute, hash the new state, then discard so every
                // iteration replays the same block from the same state.
                staged.begin().unwrap();
                let receipts = executor.execute_and_commit(block).unwrap();
                assert!(receipts.iter().all(|r| r.success));
                let root = staged.staged_lthash().unwrap().checksum();
                staged.discard();
                root
            })
        });
    }
    group.finish();
}

fn mempool_admit(c: &mut Criterion) {
    let mut group = c.benchmark_group("mempool_admit");
    let n = 1_000usize;
    let txs = signed_transfers(n, 0);
    group.throughput(Throughput::Elements(n as u64));
    group.bench_function(BenchmarkId::from_parameter(n), |b| {
        b.iter_batched(
            || {
                let config = MempoolConfig {
                    max_capacity: 100_000,
                    chain_id: Some(CHAIN_ID),
                    ..MempoolConfig::default()
                };
                (TxPool::new(config, vec![]), txs.clone())
            },
            |(pool, txs)| {
                for tx in txs {
                    pool.add_transaction(tx).unwrap();
                }
                pool
            },
            BatchSize::LargeInput,
        )
    });
    group.finish();
}

fn validators(n: usize) -> (Vec<SigningKey>, Vec<ValidatorInfo>) {
    let keys: Vec<SigningKey> = (0..n)
        .map(|i| {
            let mut seed = [3u8; 32];
            seed[..8].copy_from_slice(&(i as u64).to_le_bytes());
            SigningKey::from_bytes(&seed).unwrap()
        })
        .collect();
    let infos = keys
        .iter()
        .map(|k| ValidatorInfo {
            public_key: k.public_key(),
            stake: 100,
            slashed: false,
        })
        .collect();
    (keys, infos)
}

fn commit_certificate_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("commit_certificate_verify");
    for n in [4usize, 21, 100] {
        let (keys, infos) = validators(n);
        let hash = [5u8; 32];
        let precommits = keys
            .iter()
            .map(|k| {
                let mut v = Vote {
                    height: 10,
                    round: 0,
                    step: Step::Precommit,
                    block_hash: Some(hash),
                    signature: vec![],
                    voter: k.public_key(),
                };
                v.signature = k.sign(&v.signing_bytes());
                v
            })
            .collect();
        let cert = CommitCertificate {
            height: 10,
            round: 0,
            block_hash: hash,
            precommits,
        };
        group.bench_with_input(BenchmarkId::from_parameter(n), &cert, |b, cert| {
            b.iter(|| cert.verify(&hash, 10, &infos).unwrap())
        });
    }
    group.finish();
}

/// Drive `engines` through one height with synchronous delivery; returns
/// once every engine has finalized.
fn run_height(engines: &mut [BftEngine]) {
    let n = engines.len();
    let mut queue: VecDeque<(usize, ConsensusMessage)> = VecDeque::new();
    let mut finalized = vec![false; n];
    let height = engines[0].height;
    let enqueue = |from: usize, events: Vec<BftEvent>, q: &mut VecDeque<_>, fin: &mut Vec<bool>| {
        for e in events {
            match e {
                BftEvent::BroadcastVote(v) => q.push_back((from, ConsensusMessage::Vote(v))),
                BftEvent::BroadcastProposal(p) => {
                    q.push_back((from, ConsensusMessage::Proposal(p)))
                }
                BftEvent::FinalizeBlock(..) => fin[from] = true,
                _ => {}
            }
        }
    };
    for (i, engine) in engines.iter_mut().enumerate() {
        let ev = engine.start_round(0);
        enqueue(i, ev, &mut queue, &mut finalized);
    }
    let proposer = (0..n).find(|&i| engines[i].is_proposer(height, 0)).unwrap();
    let mut header = Header::new([0; 32], height - 1);
    header.extrinsics_root = [height as u8; 32];
    let ev = engines[proposer].create_proposal(Block::new(header, vec![]));
    enqueue(proposer, ev, &mut queue, &mut finalized);
    while let Some((from, msg)) = queue.pop_front() {
        for to in (0..n).filter(|&t| t != from) {
            let ev = match msg.clone() {
                ConsensusMessage::Vote(v) => engines[to].handle_vote(v),
                ConsensusMessage::Proposal(p) => engines[to].handle_proposal(p),
            };
            enqueue(to, ev, &mut queue, &mut finalized);
        }
    }
    assert!(finalized.iter().all(|f| *f), "height did not finalize");
}

fn bft_height(c: &mut Criterion) {
    let mut group = c.benchmark_group("bft_height");
    group.sample_size(20);
    for n in [4usize, 10, 31] {
        let (keys, infos) = validators(n);
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter_batched(
                || {
                    keys.iter()
                        .map(|k| {
                            BftEngine::new(
                                k.public_key(),
                                infos.clone(),
                                1,
                                SigningKey::from_bytes(&k.to_bytes()).unwrap(),
                            )
                        })
                        .collect::<Vec<_>>()
                },
                |mut engines| run_height(&mut engines),
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    execute_block,
    mempool_admit,
    commit_certificate_verify,
    bft_height
);
criterion_main!(benches);
