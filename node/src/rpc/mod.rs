//! # JSON-RPC Server Module
//!
//! This module implements the HTTP/JSON-RPC API for the blockchain node.
//! It provides endpoints for:
//! - Submitting transactions
//! - Querying balances, blocks, and receipts
//! - Checking node status and mempool
//! - Estimating gas prices
//! - Connecting to peers
//! - Prometheus metrics
//!
//! ## Rate Limiting
//! Each IP is limited to 100 requests per second to prevent DoS attacks.
//!
//! ## CORS
//! CORS headers are enabled to allow web wallets and explorers to connect.

use crate::bridge_queue::{BridgeUnlockQueue, PendingUnlock};
use crate::tx_pool::TxPool;
use common::types::{Block, Transaction};
use consensus::slashing::SlashingTracker;
use execution::account_abstraction::UserOperation;
use governance::{ChainGovernance, Proposal, ProposalStatus};
pub use network::NetworkCommand;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use storage::trie::PatriciaTrie;
use storage::ChainStore;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use warp::Filter;

mod aa_mev;
mod attestations;
mod bridge;
mod chain;
mod dev;
mod fees;
mod security;
mod staking;
mod types;
use aa_mev::*;
use bridge::*;
use chain::*;
use dev::*;
use fees::*;
pub use security::RpcSecurity;
use security::*;
use staking::*;
use types::*;

pub struct RpcServer {
    tx_pool: Arc<TxPool>,
    chain_store: Arc<ChainStore>,
    state_trie: Arc<Mutex<PatriciaTrie>>,
    metrics: Arc<crate::metrics::Metrics>,
    network_cmd_sender: mpsc::Sender<NetworkCommand>,
    slashing_tracker: Arc<Mutex<SlashingTracker>>,
    rate_limit: u32,
    chain_id: u64,
    security: RpcSecurity,
}

impl RpcServer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tx_pool: Arc<TxPool>,
        chain_store: Arc<ChainStore>,
        state_trie: Arc<Mutex<PatriciaTrie>>,
        metrics: Arc<crate::metrics::Metrics>,
        network_cmd_sender: mpsc::Sender<NetworkCommand>,
        slashing_tracker: Arc<Mutex<SlashingTracker>>,
        rate_limit: u32,
        chain_id: u64,
    ) -> Self {
        Self {
            tx_pool,
            chain_store,
            state_trie,
            metrics,
            network_cmd_sender,
            slashing_tracker,
            rate_limit,
            chain_id,
            security: RpcSecurity::default(),
        }
    }

    /// Set dev-mode / admin-token / CORS policy. Defaults to the locked-down
    /// production policy (no state-writing routes, no cross-origin access).
    pub fn with_security(mut self, security: RpcSecurity) -> Self {
        self.security = security;
        self
    }

    pub async fn run(&self, port: u16) {
        use governor::clock::DefaultClock;
        use governor::state::keyed::DefaultKeyedStateStore;
        use governor::{Quota, RateLimiter};
        use std::net::IpAddr;
        use std::num::NonZeroU32;

        let tx_pool = self.tx_pool.clone();
        let chain_store = self.chain_store.clone();
        let state_trie = self.state_trie.clone();
        let metrics = self.metrics.clone();
        let network_cmd_sender = self.network_cmd_sender.clone();
        let slashing_tracker = self.slashing_tracker.clone();
        let chain_id = self.chain_id;
        let dev = dev_only(&self.security);
        let admin = admin_only(&self.security);
        let dev_signer: Option<Arc<DevSigner>> = self.security.dev_funding_key.map(|seed| {
            let key = ed25519_dalek::SigningKey::from_bytes(&seed);
            let address =
                common::crypto::derive_address_from_public_key(&key.verifying_key().to_bytes());
            Arc::new(DevSigner {
                key,
                address,
                next_nonce: Mutex::new(0),
                chain_store: chain_store.clone(),
                tx_pool: tx_pool.clone(),
                network_cmd_sender: network_cmd_sender.clone(),
                chain_id,
            })
        });
        let with_dev_signer = {
            let signer = dev_signer.clone();
            warp::any().map(move || signer.clone())
        };

        // Per-address faucet cooldown tracking (server-side, not client-enforced)
        let faucet_cooldowns: Arc<Mutex<HashMap<String, u64>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let bridge_minted: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
        let attestation_pool: Arc<Mutex<attestations::AttestationPool>> =
            Arc::new(Mutex::new(attestations::AttestationPool::default()));
        let bridge_unlock_queue: Arc<Mutex<BridgeUnlockQueue>> =
            Arc::new(Mutex::new(BridgeUnlockQueue::new()));

        // Rate limiter: requests/second per IP (configurable via node config)
        let rl = NonZeroU32::new(self.rate_limit).unwrap_or(NonZeroU32::MIN.saturating_add(199));
        let rate_limiter = Arc::new(RateLimiter::<
            IpAddr,
            DefaultKeyedStateStore<IpAddr>,
            DefaultClock,
        >::keyed(Quota::per_second(rl)));

        let with_rate_limit = warp::any()
            .map(move || rate_limiter.clone())
            .and(warp::addr::remote())
            .and_then(
                |limiter: Arc<
                    RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>,
                >,
                 addr: Option<std::net::SocketAddr>| async move {
                    if let Some(addr) = addr {
                        if limiter.check_key(&addr.ip()).is_err() {
                            return Err(warp::reject::custom(RateLimited));
                        }
                    }
                    Ok(())
                },
            );

        // 1MB request body limit
        let body_limit = warp::body::content_length_limit(1024 * 1024);

        // ====================================================================
        // Core Routes
        // ====================================================================

        let chain_id_rpc = self.chain_id;

        // GET /status - Node health and height information
        let status = warp::path("status")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and(with_arc(tx_pool.clone()))
            .and(with_arc(metrics.clone()))
            .and(warp::any().map(move || chain_id_rpc))
            .and_then(|_, chain_store, tx_pool, metrics, chain_id| {
                handle_status(chain_store, tx_pool, metrics, chain_id)
            });

        // GET /mempool - List pending transactions
        let mempool_route = warp::path("mempool")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(tx_pool.clone()))
            .and_then(|_, tx_pool| handle_mempool(tx_pool));

        // POST /submit_tx - Submit a new transaction
        let submit_tx = warp::path("submit_tx")
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(tx_pool.clone()))
            .and(with_arc(network_cmd_sender.clone()))
            .and(warp::any().map(move || chain_id))
            .and_then(
                |_,
                 tx: Transaction,
                 tx_pool: Arc<TxPool>,
                 network_cmd_sender: mpsc::Sender<NetworkCommand>,
                 chain_id: u64| async move {
                    handle_submit_tx(tx, tx_pool, network_cmd_sender, chain_id).await
                },
            );

        // POST /submit_user_operation - Account abstraction (ERC-4337 style)
        let submit_user_op = warp::path("submit_user_operation")
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(tx_pool.clone()))
            .and_then(|_, op: UserOperation, tx_pool: Arc<TxPool>| async move {
                handle_submit_user_operation(op, tx_pool).await
            });

        // GET /user_operations/pending - Pending AA operations count
        let pending_user_ops = warp::path!("user_operations" / "pending")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(tx_pool.clone()))
            .and_then(|_, tx_pool| handle_pending_user_operations(tx_pool));

        // POST /mev/commit - Commit-reveal: submit commitment
        let mev_commit =
            warp::path!("mev" / "commit")
                .and(with_rate_limit.clone())
                .and(warp::post())
                .and(body_limit)
                .and(warp::body::json())
                .and(with_arc(tx_pool.clone()))
                .and(with_arc(chain_store.clone()))
                .and_then(
                    |_,
                     req: MevCommitRequest,
                     tx_pool: Arc<TxPool>,
                     chain_store: Arc<ChainStore>| async move {
                        handle_mev_commit(req, tx_pool, chain_store).await
                    },
                );

        // POST /mev/reveal - Commit-reveal: reveal transaction
        let mev_reveal =
            warp::path!("mev" / "reveal")
                .and(with_rate_limit.clone())
                .and(warp::post())
                .and(body_limit)
                .and(warp::body::json())
                .and(with_arc(tx_pool.clone()))
                .and(with_arc(chain_store.clone()))
                .and_then(
                    |_,
                     req: MevRevealRequest,
                     tx_pool: Arc<TxPool>,
                     chain_store: Arc<ChainStore>| async move {
                        handle_mev_reveal(req, tx_pool, chain_store).await
                    },
                );

        // POST /mev/encrypted - Submit threshold-encrypted transaction
        let mev_encrypted = warp::path!("mev" / "encrypted")
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(tx_pool.clone()))
            .and_then(
                |_, enc: mev::EncryptedTransaction, tx_pool: Arc<TxPool>| async move {
                    handle_mev_encrypted(enc, tx_pool).await
                },
            );

        // POST /mev/decryption_share - Submit validator decryption share
        let mev_share = warp::path!("mev" / "decryption_share")
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(tx_pool.clone()))
            .and_then(
                |_, share: mev::DecryptionShare, tx_pool: Arc<TxPool>| async move {
                    handle_mev_decryption_share(share, tx_pool).await
                },
            );

        // GET /slashing/events - Slashed validators
        let slashing_events = warp::path!("slashing" / "events")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(slashing_tracker.clone()))
            .and_then(|_, tracker| handle_slashing_events(tracker));

        // POST /delegate, /validators/register — replaced by signed staking
        // transactions (they wrote any stake into one node's state, and with
        // a real state root that node would fork off the network).
        let delegate_route = warp::path("delegate").and(warp::post()).map(|| {
            gone(r#"Submit a signed transaction with payload {"action":"delegate","validator":"0x…","amount":"<wei>"} to /submit_tx"#)
        });
        let register_validator = warp::path!("validators" / "register")
            .and(warp::post())
            .map(|| {
                gone(r#"Submit a signed transaction with payload {"action":"register_validator","stake":"<wei>","commission_rate":10} to /submit_tx"#)
            });

        // GET /ws - WebSocket subscriptions (newHead events)
        let ws_route = warp::path("ws")
            .and(warp::ws())
            .and(with_arc(chain_store.clone()))
            .map(|ws: warp::ws::Ws, chain_store: Arc<ChainStore>| {
                ws.on_upgrade(move |socket| handle_websocket(socket, chain_store))
            });

        // GET /block/{height} - Get block by height
        let block_by_height = warp::path!("block" / u64)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and_then(|height, _, chain_store| handle_get_block_by_height(height, chain_store));

        // GET /block/hash/{hash} - Get block by hash
        let block_by_hash = warp::path!("block" / "hash" / String)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and_then(|hash, _, chain_store| handle_get_block_by_hash(hash, chain_store));

        // GET /balance/{address} - Get account balance and nonce
        let balance = warp::path!("balance" / String)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and_then(|address, _, chain_store| handle_get_balance(address, chain_store));

        // GET /tx/{hash} - Get transaction receipt
        let tx_receipt = warp::path!("tx" / String)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and_then(|hash, _, chain_store| handle_get_receipt(hash, chain_store));

        // GET /txs/{address} - On-chain transaction history for an address
        let txs_for_address = warp::path!("txs" / String)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(warp::query::<TxHistoryQuery>())
            .and(with_arc(chain_store.clone()))
            .and_then(|address, _, query, chain_store| {
                handle_txs_for_address(address, query, chain_store)
            });

        // ====================================================================
        // Economic Routes (Phase 9)
        // ====================================================================

        // GET /gas_price - Get current gas price suggestions (EIP-1559)
        let gas_price = warp::path("gas_price")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and_then(|_, chain_store| handle_gas_price(chain_store));

        // POST /estimate_gas - Estimate gas for a transaction
        let estimate_gas = warp::path("estimate_gas")
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(chain_store.clone()))
            .and_then(handle_estimate_gas);

        // GET /fee_history - Get historical fee data (EIP-1559)
        let fee_history = warp::path!("fee_history" / u64)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and_then(|block_count, _, chain_store| handle_fee_history(block_count, chain_store));

        // ====================================================================
        // Admin & Network Routes
        // ====================================================================

        // GET /metrics - Prometheus metrics endpoint
        let metrics_route = warp::path("metrics")
            .and(warp::get())
            .and(with_arc(metrics.clone()))
            .and_then(handle_metrics);

        // GET /health - Simple health check
        let health = warp::path("health")
            .and(warp::get())
            .and_then(handle_health);

        // POST /connect_peer - Connect to a peer by multiaddress
        let connect_peer = warp::path("connect_peer")
            .and(admin.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(network_cmd_sender.clone()))
            .and_then(handle_connect_peer);

        // GET /peers - List connected peers
        let peers = warp::path("peers")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(network_cmd_sender.clone()))
            .and_then(|_, _network_cmd| handle_peers());

        // GET /validators - List active validators
        let validators = warp::path("validators")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(state_trie.clone()))
            .and_then(|_, state_trie| handle_validators(state_trie));

        // GET /block/latest - Get the latest block
        let block_latest = warp::path!("block" / "latest")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and_then(|_, chain_store| handle_block_latest(chain_store));

        // GET /delegations/{address} - Get delegations for an address
        let delegations = warp::path!("delegations" / String)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and_then(|address, _, chain_store| handle_delegations(address, chain_store));

        // GET /governance - On-chain governance state (proposals, treasury, params)
        let governance = warp::path("governance")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(state_trie.clone()))
            .and(with_arc(chain_store.clone()))
            .and_then(|_, state_trie, chain_store| handle_governance(state_trie, chain_store));

        // GET /proposal/{id} - Single proposal with live status
        let proposal_by_id = warp::path!("proposal" / u64)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(state_trie.clone()))
            .and(with_arc(chain_store.clone()))
            .and_then(|id, _, state_trie, chain_store| {
                handle_proposal(id, state_trie, chain_store)
            });

        // GET /runtime/version — active runtime version
        let runtime_version = warp::path!("runtime" / "version")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(state_trie.clone()))
            .and_then(|_, state_trie: Arc<Mutex<PatriciaTrie>>| async move {
                handle_runtime_version(state_trie).await
            });

        // GET /runtime/upgrades — pending runtime upgrade proposals
        let runtime_upgrades = warp::path!("runtime" / "upgrades")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(state_trie.clone()))
            .and_then(|_, state_trie: Arc<Mutex<PatriciaTrie>>| async move {
                handle_runtime_upgrades(state_trie).await
            });

        // POST /runtime/propose, /runtime/approve — removed: runtime upgrades
        // go through governance ("software_upgrade" proposal, then "execute").
        let runtime_propose = warp::path!("runtime" / "propose")
            .and(warp::post())
            .map(|| gone("Propose a software_upgrade through governance (signed tx)"));
        let runtime_approve = warp::path!("runtime" / "approve")
            .and(warp::post())
            .map(|| gone("Runtime upgrades are approved by executing the governance proposal"));

        // POST /faucet/request - Request test tokens (directly credits the account)
        let faucet_request = warp::path!("faucet" / "request")
            .and(dev.clone())
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_dev_signer.clone())
            .and(with_arc(faucet_cooldowns.clone()))
            .and_then(
                |_,
                 req: serde_json::Value,
                 signer: Option<Arc<DevSigner>>,
                 faucet_cooldowns: Arc<Mutex<HashMap<String, u64>>>| async move {
                    handle_faucet_request(req, signer, faucet_cooldowns).await
                },
            );

        // POST /deploy_wasm — deploy a WASM contract module
        let deploy_wasm = warp::path("deploy_wasm")
            .and(dev.clone())
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_dev_signer.clone())
            .and_then(
                |_, req: DeployWasmRequest, signer: Option<Arc<DevSigner>>| async move {
                    handle_deploy_wasm(req, signer).await
                },
            );

        // POST /call_wasm — call a deployed WASM contract
        let call_wasm = warp::path("call_wasm")
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(chain_store.clone()))
            .and_then(
                |_, req: CallWasmRequest, chain_store: Arc<ChainStore>| async move {
                    handle_call_wasm(req, chain_store).await
                },
            );

        // POST /call_contract — read-only EVM call (eth_call equivalent)
        let call_contract = warp::path("call_contract")
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(chain_store.clone()))
            .and(warp::any().map(move || chain_id))
            .and_then(
                |_, req: CallContractRequest, chain_store: Arc<ChainStore>, chain_id: u64| async move {
                    handle_call_contract(req, chain_store, chain_id).await
                },
            );

        // GET /bridge/status — cross-chain bridge + relayer readiness
        let bridge_status = warp::path!("bridge" / "status")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(state_trie.clone()))
            .and(with_arc(bridge_minted.clone()))
            .and(with_arc(bridge_unlock_queue.clone()))
            .and(warp::any().map(move || chain_id))
            .and_then(
                |_,
                 state_trie: Arc<Mutex<PatriciaTrie>>,
                 bridge_minted: Arc<Mutex<HashSet<String>>>,
                 bridge_unlock_queue: Arc<Mutex<BridgeUnlockQueue>>,
                 chain_id: u64| async move {
                    handle_bridge_status(state_trie, bridge_minted, bridge_unlock_queue, chain_id)
                        .await
                },
            );

        // GET /bridge/pending_unlocks — relayer queue (Nebula lock → ETH unlock)
        let bridge_pending_unlocks = warp::path!("bridge" / "pending_unlocks")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(bridge_unlock_queue.clone()))
            .and_then(
                |_, bridge_unlock_queue: Arc<Mutex<BridgeUnlockQueue>>| async move {
                    handle_bridge_pending_unlocks(bridge_unlock_queue).await
                },
            );

        // POST /bridge/unlock/ack — relayer confirms ETH unlock submitted
        let bridge_unlock_ack = warp::path!("bridge" / "unlock" / "ack")
            .and(admin.clone())
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(bridge_unlock_queue.clone()))
            .and_then(
                |_,
                 req: BridgeUnlockAckRequest,
                 bridge_unlock_queue: Arc<Mutex<BridgeUnlockQueue>>| async move {
                    handle_bridge_unlock_ack(req, bridge_unlock_queue).await
                },
            );

        // POST /bridge/mint — relayer mint on Nebula after Ethereum lock
        let bridge_mint = warp::path!("bridge" / "mint")
            .and(admin.clone())
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(chain_store.clone()))
            .and(with_arc(state_trie.clone()))
            .and(with_dev_signer.clone())
            .and(with_arc(bridge_minted.clone()))
            .and(with_arc(attestation_pool.clone()))
            .and_then(
                |_,
                 req: BridgeMintRequest,
                 chain_store: Arc<ChainStore>,
                 state_trie: Arc<Mutex<PatriciaTrie>>,
                 signer: Option<Arc<DevSigner>>,
                 bridge_minted: Arc<Mutex<HashSet<String>>>,
                 pool: Arc<Mutex<attestations::AttestationPool>>| async move {
                    handle_bridge_mint(req, chain_store, state_trie, signer, bridge_minted, pool)
                        .await
                },
            );

        // POST /bridge/attestations — a relayer's signed attestation for an
        // Ethereum lock (public: only valid genesis-relayer signatures count)
        let bridge_attest = warp::path!("bridge" / "attestations")
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(state_trie.clone()))
            .and(with_arc(chain_store.clone()))
            .and(with_arc(attestation_pool.clone()))
            .and(warp::any().map(move || chain_id_rpc))
            .and_then(|_, sub, state_trie, chain_store, pool, chain_id| {
                attestations::handle_post_attestation(sub, state_trie, chain_store, pool, chain_id)
            });

        // GET /bridge/attestations/<eth tx hash> — pooled attestations + minted flag
        let bridge_attestations = warp::path!("bridge" / "attestations" / String)
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(state_trie.clone()))
            .and(with_arc(chain_store.clone()))
            .and(with_arc(attestation_pool.clone()))
            .and_then(|hash, _, state_trie, chain_store, pool| {
                attestations::handle_get_attestations(hash, state_trie, chain_store, pool)
            });

        // POST /bridge/unlock — relayer unlock on Ethereum after Nebula lock
        let bridge_unlock = warp::path!("bridge" / "unlock")
            .and(admin.clone())
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(chain_store.clone()))
            .and(with_arc(bridge_unlock_queue.clone()))
            .and_then(
                |_,
                 req: BridgeUnlockRequest,
                 chain_store: Arc<ChainStore>,
                 bridge_unlock_queue: Arc<Mutex<BridgeUnlockQueue>>| async move {
                    handle_bridge_unlock(req, chain_store, bridge_unlock_queue).await
                },
            );

        // GET /wasm/contracts — list deployed WASM contracts
        let wasm_contracts = warp::path!("wasm" / "contracts")
            .and(with_rate_limit.clone())
            .and(warp::get())
            .and(with_arc(chain_store.clone()))
            .and_then(|_, chain_store: Arc<ChainStore>| async move {
                handle_list_wasm(chain_store).await
            });

        // ====================================================================
        // Combine Routes with CORS
        // ====================================================================

        let routes = status
            .or(mempool_route)
            .or(submit_tx)
            .or(submit_user_op)
            .or(pending_user_ops)
            .or(mev_commit)
            .or(mev_reveal)
            .or(mev_encrypted)
            .or(mev_share)
            .or(slashing_events)
            .or(delegate_route)
            .or(register_validator)
            .or(ws_route)
            .or(block_by_height)
            .or(block_by_hash)
            .or(block_latest)
            .or(balance)
            .or(txs_for_address)
            .or(tx_receipt)
            .or(gas_price)
            .or(estimate_gas)
            .or(call_contract)
            .or(bridge_status)
            .or(bridge_mint)
            .or(bridge_attest)
            .or(bridge_attestations)
            .or(bridge_unlock)
            .or(bridge_pending_unlocks)
            .or(bridge_unlock_ack)
            .or(fee_history)
            .or(validators)
            .or(delegations)
            .or(governance)
            .or(proposal_by_id)
            .or(runtime_version)
            .or(runtime_upgrades)
            .or(runtime_propose)
            .or(runtime_approve)
            .or(faucet_request)
            .or(deploy_wasm)
            .or(call_wasm)
            .or(wasm_contracts)
            .or(metrics_route)
            .or(health)
            .or(connect_peer)
            .or(peers)
            .recover(handle_rejection);

        let cors = build_cors(&self.security.cors_origins, self.security.dev_mode);
        let routes_with_cors = routes.with(cors);

        tracing::info!("🚀 RPC server listening on 0.0.0.0:{}", port);
        warp::serve(routes_with_cors)
            .run(([0, 0, 0, 0], port))
            .await;
    }
}

fn with_arc<T: Clone + Send + Sync>(
    val: T,
) -> impl Filter<Extract = (T,), Error = Infallible> + Clone {
    warp::any().map(move || val.clone())
}

/// Custom rejection type for rate limiting (so we can distinguish from other errors)
#[derive(Debug)]
struct RateLimited;

impl warp::reject::Reject for RateLimited {}

async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, Infallible> {
    let (status, message) = if err.is_not_found() {
        (
            warp::http::StatusCode::NOT_FOUND,
            "Endpoint not found".to_string(),
        )
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        (
            warp::http::StatusCode::METHOD_NOT_ALLOWED,
            "Method not allowed".to_string(),
        )
    } else if err.find::<warp::reject::PayloadTooLarge>().is_some() {
        (
            warp::http::StatusCode::PAYLOAD_TOO_LARGE,
            "Request body too large (max 1MB)".to_string(),
        )
    } else if err.find::<warp::reject::InvalidQuery>().is_some() {
        (
            warp::http::StatusCode::BAD_REQUEST,
            "Invalid query parameters".to_string(),
        )
    } else if err.find::<Unauthorized>().is_some() {
        (
            warp::http::StatusCode::UNAUTHORIZED,
            "Admin token required (Authorization: Bearer <token>)".to_string(),
        )
    } else if err.find::<RateLimited>().is_some() {
        (
            warp::http::StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded".to_string(),
        )
    } else {
        #[cfg(debug_assertions)]
        tracing::warn!("Unhandled rejection: {:?}", err);
        (
            warp::http::StatusCode::BAD_REQUEST,
            "Bad request".to_string(),
        )
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&ErrorResponse { error: message }),
        status,
    ))
}
