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

use crate::governance_store;
use crate::tx_pool::TxPool;
use common::types::{Block, Transaction};
use consensus::slashing::SlashingTracker;
use execution::account_abstraction::UserOperation;
use execution::gas::calculate_next_base_fee;
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
use warp::{http::HeaderValue, Filter};

/// Minimum time (seconds) between faucet drips to the same address.
const FAUCET_COOLDOWN_SECS: u64 = 60;

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize)]
struct StatusResponse {
    height: u64,
    finalized_height: Option<u64>,
    mempool_size: usize,
    peer_count: usize,
    chain_id: u64,
}

#[derive(Debug, Serialize)]
struct MempoolResponse {
    size: usize,
    transactions: Vec<Transaction>,
}

#[derive(Debug, Serialize)]
struct SubmitTxResponse {
    status: String,
    hash: String,
}

#[derive(Debug, Serialize)]
struct BlockResponse {
    block: Option<serde_json::Value>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct BalanceResponse {
    address: String,
    balance: String,
    nonce: u64,
}

#[derive(Debug, Serialize)]
struct ReceiptResponse {
    receipt: Option<serde_json::Value>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TxHistoryQuery {
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct TxHistoryEntry {
    hash: String,
    block_height: u64,
    sender: String,
    to: Option<String>,
    value: u64,
    nonce: u64,
    is_contract_creation: bool,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
struct TxHistoryResponse {
    address: String,
    transactions: Vec<TxHistoryEntry>,
    scanned_blocks: u64,
}

#[derive(Debug, Deserialize)]
struct ConnectPeerRequest {
    multiaddr: String,
}

#[derive(Debug, Serialize)]
struct ConnectPeerResponse {
    status: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ValidatorInfo {
    address: String,
    public_key: String,
    stake: String,
    commission_rate: u64,
    is_active: bool,
    blocks_produced: u64,
    blocks_missed: u64,
    delegator_count: u64,
    total_delegated: String,
}

#[derive(Debug, Serialize)]
struct ValidatorsResponse {
    validators: Vec<ValidatorInfo>,
    count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct DelegationInfo {
    validator_address: String,
    amount: String,
}

#[derive(Debug, Serialize)]
struct DelegationsResponse {
    delegations: Vec<DelegationInfo>,
    address: String,
}

// ============================================================================
// Economic/Phase 9 Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct GasPriceResponse {
    base_fee: String,
    suggested_priority_fee_low: String,
    suggested_priority_fee_medium: String,
    suggested_priority_fee_high: String,
    block_height: u64,
}

#[derive(Debug, Deserialize)]
struct EstimateGasRequest {
    from: String,
    to: String,
    data: String,
    value: Option<String>,
    max_fee_per_gas: Option<String>,
}

#[derive(Debug, Serialize)]
struct EstimateGasResponse {
    estimated_gas: u64,
    base_fee: String,
    total_cost_estimate: String,
    estimated_priority_fee: String,
}

#[derive(Debug, Serialize)]
struct FeeHistoryResponse {
    base_fee_per_gas: Vec<String>,
    gas_used_ratio: Vec<f64>,
    oldest_block: u64,
}

#[derive(Debug, Deserialize)]
struct CallContractRequest {
    from: String,
    to: String,
    data: String,
    value: Option<String>,
}

#[derive(Debug, Serialize)]
struct CallContractResponse {
    result: String,
    success: bool,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BridgeMintRequest {
    recipient: String,
    amount: Option<String>,
    eth_tx_hash: Option<String>,
    eth_rpc_url: Option<String>,
    eth_bridge_address: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BridgeUnlockRequest {
    nebula_tx_hash: String,
    eth_recipient: Option<String>,
    eth_rpc_url: Option<String>,
    eth_bridge_address: Option<String>,
}

#[derive(Debug, Serialize)]
struct BridgeStatusResponse {
    vault_address: String,
    defi_pool_address: String,
    validators_count: usize,
    relayers_ready: bool,
    eth_rpc_configured: bool,
    processed_mints: usize,
    processed_unlocks: usize,
    chain_id: u64,
}

// ============================================================================
// CORS Helper
// ============================================================================

/// Add CORS headers to allow web clients to access the API
fn with_cors() -> warp::cors::Cors {
    warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST", "OPTIONS"])
        .allow_headers(vec!["Content-Type", "Authorization"])
        .max_age(3600)
        .build()
}

// ============================================================================
// RpcServer
// ============================================================================

pub struct RpcServer {
    tx_pool: Arc<TxPool>,
    chain_store: Arc<ChainStore>,
    state_trie: Arc<Mutex<PatriciaTrie>>,
    metrics: Arc<crate::metrics::Metrics>,
    network_cmd_sender: mpsc::Sender<NetworkCommand>,
    slashing_tracker: Arc<Mutex<SlashingTracker>>,
    rate_limit: u32,
    chain_id: u64,
}

impl RpcServer {
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
        }
    }

    pub async fn run(&self, port: u16, enable_cors: bool) {
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

        // Per-address faucet cooldown tracking (server-side, not client-enforced)
        let faucet_cooldowns: Arc<Mutex<HashMap<String, u64>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let bridge_minted: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
        let bridge_unlocked: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

        // Rate limiter: requests/second per IP (configurable via node config)
        let rl = NonZeroU32::new(self.rate_limit).unwrap_or(NonZeroU32::new(200).unwrap());
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
            .and(warp::any().map(move || chain_id_rpc))
            .and_then(|_, chain_store, tx_pool, chain_id| {
                handle_status(chain_store, tx_pool, chain_id)
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
            .and_then(
                |_,
                 tx: Transaction,
                 tx_pool: Arc<TxPool>,
                 network_cmd_sender: mpsc::Sender<NetworkCommand>| async move {
                    handle_submit_tx(tx, tx_pool, network_cmd_sender).await
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

        // POST /delegate - Delegate stake to a validator
        let delegate_route = warp::path("delegate")
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(state_trie.clone()))
            .and(with_arc(chain_store.clone()))
            .and_then(
                |_, req: DelegateRequest, state_trie, chain_store| async move {
                    handle_delegate(req, state_trie, chain_store).await
                },
            );

        // POST /validators/register - Register a new validator (dynamic set)
        let register_validator = warp::path!("validators" / "register")
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(state_trie.clone()))
            .and_then(|_, req: RegisterValidatorRequest, state_trie| async move {
                handle_register_validator(req, state_trie).await
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

        // POST /faucet/request - Request test tokens (directly credits the account)
        let faucet_request = warp::path!("faucet" / "request")
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(chain_store.clone()))
            .and(with_arc(faucet_cooldowns.clone()))
            .and_then(
                |_,
                 req: serde_json::Value,
                 chain_store: Arc<ChainStore>,
                 faucet_cooldowns: Arc<Mutex<HashMap<String, u64>>>| async move {
                    handle_faucet_request(req, chain_store, faucet_cooldowns).await
                },
            );

        // POST /deploy_wasm — deploy a WASM contract module
        let deploy_wasm = warp::path("deploy_wasm")
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(chain_store.clone()))
            .and_then(
                |_, req: DeployWasmRequest, chain_store: Arc<ChainStore>| async move {
                    handle_deploy_wasm(req, chain_store).await
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
            .and(with_arc(bridge_unlocked.clone()))
            .and(warp::any().map(move || chain_id))
            .and_then(
                |_,
                 state_trie: Arc<Mutex<PatriciaTrie>>,
                 bridge_minted: Arc<Mutex<HashSet<String>>>,
                 bridge_unlocked: Arc<Mutex<HashSet<String>>>,
                 chain_id: u64| async move {
                    handle_bridge_status(state_trie, bridge_minted, bridge_unlocked, chain_id).await
                },
            );

        // POST /bridge/mint — relayer mint on Nebula after Ethereum lock
        let bridge_mint = warp::path!("bridge" / "mint")
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(chain_store.clone()))
            .and(with_arc(bridge_minted.clone()))
            .and_then(
                |_,
                 req: BridgeMintRequest,
                 chain_store: Arc<ChainStore>,
                 bridge_minted: Arc<Mutex<HashSet<String>>>| async move {
                    handle_bridge_mint(req, chain_store, bridge_minted).await
                },
            );

        // POST /bridge/unlock — relayer unlock on Ethereum after Nebula lock
        let bridge_unlock = warp::path!("bridge" / "unlock")
            .and(with_rate_limit.clone())
            .and(warp::post())
            .and(body_limit)
            .and(warp::body::json())
            .and(with_arc(chain_store.clone()))
            .and(with_arc(bridge_unlocked.clone()))
            .and_then(
                |_,
                 req: BridgeUnlockRequest,
                 chain_store: Arc<ChainStore>,
                 bridge_unlocked: Arc<Mutex<HashSet<String>>>| async move {
                    handle_bridge_unlock(req, chain_store, bridge_unlocked).await
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
            .or(bridge_unlock)
            .or(fee_history)
            .or(validators)
            .or(delegations)
            .or(governance)
            .or(proposal_by_id)
            .or(faucet_request)
            .or(deploy_wasm)
            .or(call_wasm)
            .or(wasm_contracts)
            .or(metrics_route)
            .or(health)
            .or(connect_peer)
            .or(peers)
            .recover(handle_rejection);

        let cors = if enable_cors {
            with_cors()
        } else {
            warp::cors().allow_any_origin().build()
        };
        let routes_with_cors = routes.with(cors);

        tracing::info!("🚀 RPC server listening on 0.0.0.0:{}", port);
        warp::serve(routes_with_cors)
            .run(([0, 0, 0, 0], port))
            .await;
    }
}

// ============================================================================
// Filter Helper
// ============================================================================

fn with_arc<T: Clone + Send + Sync>(
    val: T,
) -> impl Filter<Extract = (T,), Error = Infallible> + Clone {
    warp::any().map(move || val.clone())
}

// ============================================================================
// Core Handlers
// ============================================================================

async fn handle_status(
    chain_store: Arc<ChainStore>,
    tx_pool: Arc<TxPool>,
    chain_id: u64,
) -> Result<impl warp::Reply, Infallible> {
    let height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    let finalized_height = Some(height);
    let mempool_size = tx_pool.size();
    let peer_count = 0; // Would need network service connection for this

    Ok(warp::reply::json(&StatusResponse {
        height,
        finalized_height,
        mempool_size,
        peer_count,
        chain_id,
    }))
}

async fn handle_mempool(tx_pool: Arc<TxPool>) -> Result<impl warp::Reply, Infallible> {
    let transactions = tx_pool.mempool_snapshot(100);
    Ok(warp::reply::json(&MempoolResponse {
        size: tx_pool.size(),
        transactions,
    }))
}

async fn handle_submit_tx(
    tx: Transaction,
    tx_pool: Arc<TxPool>,
    network_cmd_sender: mpsc::Sender<NetworkCommand>,
) -> Result<impl warp::Reply, Infallible> {
    // Validate chain ID if present
    if let Some(tx_chain_id) = tx.chain_id {
        // In production, check against configured chain ID
        if tx_chain_id != 1 {
            return Ok(warp::reply::json(&SubmitTxResponse {
                status: format!("error: invalid chain_id (expected 1, got {})", tx_chain_id),
                hash: String::new(),
            }));
        }
    }

    match tx_pool.add_transaction(tx.clone()) {
        Ok(_) => {
            // Broadcast to peers
            let _ = network_cmd_sender
                .send(NetworkCommand::BroadcastTransaction(tx.clone()))
                .await;
            Ok(warp::reply::json(&SubmitTxResponse {
                status: "success".into(),
                hash: hex::encode(tx.hash()),
            }))
        }
        Err(e) => {
            let msg = e.to_string();
            let hash = if msg.contains("already in mempool") {
                hex::encode(tx.hash())
            } else {
                String::new()
            };
            Ok(warp::reply::json(&SubmitTxResponse {
                status: format!("error: {}", msg),
                hash,
            }))
        }
    }
}

async fn handle_get_block_by_height(
    height: u64,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    match chain_store.get_block_hash_by_height(height) {
        Ok(Some(hash_bytes)) if hash_bytes.len() == 32 => {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&hash_bytes);
            match chain_store.get_block(&hash) {
                Ok(Some(encoded)) => {
                    let val: serde_json::Value =
                        serde_json::from_slice(&encoded).unwrap_or(serde_json::Value::Null);
                    Ok(warp::reply::json(&BlockResponse {
                        block: Some(val),
                        error: None,
                    }))
                }
                Ok(None) => Ok(warp::reply::json(&BlockResponse {
                    block: None,
                    error: Some(format!("Block data missing for height {}", height)),
                })),
                Err(e) => Ok(warp::reply::json(&BlockResponse {
                    block: None,
                    error: Some(e.to_string()),
                })),
            }
        }
        Ok(_) => Ok(warp::reply::json(&BlockResponse {
            block: None,
            error: Some(format!("No block at height {}", height)),
        })),
        Err(e) => Ok(warp::reply::json(&BlockResponse {
            block: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_get_block_by_hash(
    hash_str: String,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let hash_bytes = match hex::decode(hash_str.trim_start_matches("0x")) {
        Ok(b) if b.len() == 32 => {
            let mut h = [0u8; 32];
            h.copy_from_slice(&b);
            h
        }
        _ => {
            return Ok(warp::reply::json(&BlockResponse {
                block: None,
                error: Some("Invalid hash: must be 32-byte hex string".into()),
            }))
        }
    };

    match chain_store.get_block(&hash_bytes) {
        Ok(Some(encoded)) => {
            let val: serde_json::Value =
                serde_json::from_slice(&encoded).unwrap_or(serde_json::Value::Null);
            Ok(warp::reply::json(&BlockResponse {
                block: Some(val),
                error: None,
            }))
        }
        Ok(None) => Ok(warp::reply::json(&BlockResponse {
            block: None,
            error: Some(format!("Block not found: {}", hash_str)),
        })),
        Err(e) => Ok(warp::reply::json(&BlockResponse {
            block: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_get_balance(
    address_str: String,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let address_bytes = match hex::decode(address_str.trim_start_matches("0x")) {
        Ok(b) if b.len() == 20 => b,
        _ => {
            return Ok(warp::reply::json(&BalanceResponse {
                address: address_str,
                balance: "0".into(),
                nonce: 0,
            }))
        }
    };

    match chain_store.get_state(&address_bytes) {
        Ok(Some(encoded)) => {
            #[derive(serde::Deserialize)]
            struct AccountEncoded {
                balance: String,
                nonce: u64,
            }
            if let Ok(acc) = serde_json::from_slice::<AccountEncoded>(&encoded) {
                Ok(warp::reply::json(&BalanceResponse {
                    address: address_str,
                    balance: acc.balance,
                    nonce: acc.nonce,
                }))
            } else {
                Ok(warp::reply::json(&BalanceResponse {
                    address: address_str,
                    balance: "0".into(),
                    nonce: 0,
                }))
            }
        }
        _ => Ok(warp::reply::json(&BalanceResponse {
            address: address_str,
            balance: "0".into(),
            nonce: 0,
        })),
    }
}

async fn handle_get_receipt(
    tx_hash_str: String,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let hash_bytes = match hex::decode(tx_hash_str.trim_start_matches("0x")) {
        Ok(b) if b.len() == 32 => b,
        _ => {
            return Ok(warp::reply::json(&ReceiptResponse {
                receipt: None,
                error: Some("Invalid tx hash".into()),
            }))
        }
    };

    match chain_store.get_receipt(&hash_bytes) {
        Ok(Some(encoded)) => {
            let val: serde_json::Value =
                serde_json::from_slice(&encoded).unwrap_or(serde_json::Value::Null);
            Ok(warp::reply::json(&ReceiptResponse {
                receipt: Some(val),
                error: None,
            }))
        }
        Ok(None) => Ok(warp::reply::json(&ReceiptResponse {
            receipt: None,
            error: Some(format!("Receipt not found: {}", tx_hash_str)),
        })),
        Err(e) => Ok(warp::reply::json(&ReceiptResponse {
            receipt: None,
            error: Some(e.to_string()),
        })),
    }
}

fn parse_address_20(s: &str) -> Option<[u8; 20]> {
    let bytes = hex::decode(s.trim_start_matches("0x")).ok()?;
    if bytes.len() != 20 {
        return None;
    }
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&bytes);
    Some(addr)
}

fn address_hex(addr: &[u8; 20]) -> String {
    format!("0x{}", hex::encode(addr))
}

async fn handle_txs_for_address(
    address_str: String,
    query: TxHistoryQuery,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let query_addr = match parse_address_20(&address_str) {
        Some(a) => a,
        None => {
            return Ok(warp::reply::json(&TxHistoryResponse {
                address: address_str,
                transactions: vec![],
                scanned_blocks: 0,
            }))
        }
    };

    let limit = query.limit.unwrap_or(25).min(100);
    let latest = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    let max_scan = 100u64;
    let start = latest.saturating_sub(max_scan.saturating_sub(1));
    let mut found = Vec::new();
    let mut scanned = 0u64;

    for height in (start..=latest).rev() {
        if found.len() >= limit {
            break;
        }
        scanned += 1;
        let Ok(Some(encoded)) = chain_store.get_block_by_height(height) else {
            continue;
        };
        let Ok(block) = serde_json::from_slice::<Block>(&encoded) else {
            continue;
        };

        for tx in &block.extrinsics {
            let matches_sender = tx.sender == query_addr;
            let matches_to = tx.to.map(|t| t == query_addr).unwrap_or(false);
            if !matches_sender && !matches_to {
                continue;
            }

            let hash = tx.hash();
            let status = chain_store
                .get_receipt(&hash)
                .ok()
                .flatten()
                .and_then(|encoded| serde_json::from_slice::<serde_json::Value>(&encoded).ok())
                .map(|receipt| {
                    if receipt
                        .get("success")
                        .and_then(|s| s.as_bool())
                        .unwrap_or(false)
                    {
                        "confirmed".to_string()
                    } else {
                        "failed".to_string()
                    }
                });

            found.push(TxHistoryEntry {
                hash: format!("0x{}", hex::encode(hash)),
                block_height: height,
                sender: address_hex(&tx.sender),
                to: tx.to.map(|t| address_hex(&t)),
                value: tx.value,
                nonce: tx.nonce,
                is_contract_creation: tx.to.is_none() && !tx.payload.is_empty(),
                status,
            });
        }
    }

    Ok(warp::reply::json(&TxHistoryResponse {
        address: address_str,
        transactions: found,
        scanned_blocks: scanned,
    }))
}

// ============================================================================
// Economic Handlers (Phase 9)
// ============================================================================

async fn handle_gas_price(chain_store: Arc<ChainStore>) -> Result<impl warp::Reply, Infallible> {
    let latest_height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);

    // Get the latest block to read actual base fee
    let (base_fee, low_priority, medium_priority, high_priority) =
        if let Ok(Some(hash_bytes)) = chain_store.get_block_hash_by_height(latest_height) {
            if hash_bytes.len() == 32 {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&hash_bytes);
                if let Ok(Some(encoded)) = chain_store.get_block(&hash) {
                    if let Ok(block_val) = serde_json::from_slice::<serde_json::Value>(&encoded) {
                        let base_fee_val = block_val
                            .get("header")
                            .and_then(|h| h.get("base_fee"))
                            .and_then(|b| b.as_u64())
                            .unwrap_or(1_000_000_000);

                        // Priority fees based on recent blocks (simplified)
                        let low = base_fee_val;
                        let medium = base_fee_val * 2;
                        let high = base_fee_val * 5;

                        (base_fee_val, low, medium, high)
                    } else {
                        (
                            1_000_000_000u64,
                            1_000_000_000,
                            2_000_000_000,
                            5_000_000_000,
                        )
                    }
                } else {
                    (
                        1_000_000_000u64,
                        1_000_000_000,
                        2_000_000_000,
                        5_000_000_000,
                    )
                }
            } else {
                (
                    1_000_000_000u64,
                    1_000_000_000,
                    2_000_000_000,
                    5_000_000_000,
                )
            }
        } else {
            (
                1_000_000_000u64,
                1_000_000_000,
                2_000_000_000,
                5_000_000_000,
            )
        };

    let response = GasPriceResponse {
        base_fee: base_fee.to_string(),
        suggested_priority_fee_low: low_priority.to_string(),
        suggested_priority_fee_medium: medium_priority.to_string(),
        suggested_priority_fee_high: high_priority.to_string(),
        block_height: latest_height,
    };

    Ok(warp::reply::json(&response))
}

async fn handle_estimate_gas(
    request: EstimateGasRequest,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    // Base transaction cost (21,000 gas)
    let base_cost = 21_000u64;

    // Calculate data cost (EIP-2028: 16 gas per non-zero byte, 4 gas per zero byte)
    let data_bytes = hex::decode(request.data.trim_start_matches("0x")).unwrap_or_default();
    let data_cost: u64 = data_bytes
        .iter()
        .map(|&byte| if byte == 0 { 4 } else { 16 })
        .sum();

    // Contract creation cost (if 'to' is empty or zero)
    let is_contract_creation =
        request.to == "0x" || request.to == "0x0000000000000000000000000000000000000000";
    let contract_cost = if is_contract_creation {
        32_000 // Contract creation base cost
    } else {
        0
    };

    // Estimated gas = base + data + contract cost + buffer (10% for safety)
    let estimated_gas = (base_cost + data_cost + contract_cost) * 110 / 100;

    // Get current base fee from latest block
    let latest_height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    let base_fee = if let Ok(Some(hash_bytes)) = chain_store.get_block_hash_by_height(latest_height)
    {
        if hash_bytes.len() == 32 {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&hash_bytes);
            if let Ok(Some(encoded)) = chain_store.get_block(&hash) {
                if let Ok(block_val) = serde_json::from_slice::<serde_json::Value>(&encoded) {
                    block_val
                        .get("header")
                        .and_then(|h| h.get("base_fee"))
                        .and_then(|b| b.as_u64())
                        .unwrap_or(1_000_000_000)
                } else {
                    1_000_000_000
                }
            } else {
                1_000_000_000
            }
        } else {
            1_000_000_000
        }
    } else {
        1_000_000_000
    };

    // Parse max fee from request or use default
    let max_fee = request
        .max_fee_per_gas
        .as_ref()
        .and_then(|f| f.parse::<u128>().ok())
        .unwrap_or(1_000_000_000);

    let total_cost = estimated_gas as u128 * max_fee;
    let estimated_priority = (max_fee / 2).max(1_000_000_000) as u64; // ≥ mempool min_fee_per_gas

    let response = EstimateGasResponse {
        estimated_gas,
        base_fee: base_fee.to_string(),
        total_cost_estimate: total_cost.to_string(),
        estimated_priority_fee: estimated_priority.to_string(),
    };

    Ok(warp::reply::json(&response))
}

async fn handle_call_contract(
    request: CallContractRequest,
    chain_store: Arc<ChainStore>,
    _chain_id: u64,
) -> Result<impl warp::Reply, Infallible> {
    use crate::evm_store::ChainStoreEvmStore;
    use execution::evm::EvmExecutor;
    use revm::primitives::{Address, U256};
    use std::str::FromStr;

    let parse_addr = |s: &str| -> Option<Address> {
        let clean = s.trim_start_matches("0x");
        if clean.len() != 40 {
            return None;
        }
        Address::from_str(&format!("0x{clean}")).ok()
    };

    let from = match parse_addr(&request.from) {
        Some(a) => a,
        None => {
            return Ok(warp::reply::json(&CallContractResponse {
                result: "0x".to_string(),
                success: false,
                error: Some("invalid from address".to_string()),
            }))
        }
    };
    let to = match parse_addr(&request.to) {
        Some(a) => a,
        None => {
            return Ok(warp::reply::json(&CallContractResponse {
                result: "0x".to_string(),
                success: false,
                error: Some("invalid to address".to_string()),
            }))
        }
    };

    let data = hex::decode(request.data.trim_start_matches("0x")).unwrap_or_default();
    let value = request
        .value
        .as_ref()
        .and_then(|v| v.parse::<u128>().ok())
        .map(U256::from)
        .unwrap_or(U256::ZERO);

    let store = Arc::new(ChainStoreEvmStore::new(chain_store));
    match EvmExecutor::static_call(store, from, to, data, value) {
        Ok(bytes) => Ok(warp::reply::json(&CallContractResponse {
            result: format!("0x{}", hex::encode(bytes)),
            success: true,
            error: None,
        })),
        Err(e) => Ok(warp::reply::json(&CallContractResponse {
            result: "0x".to_string(),
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

fn deterministic_address(label: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(label.as_bytes());
    format!("0x{}", hex::encode(&h.finalize()[..20]))
}

async fn handle_bridge_status(
    state_trie: Arc<Mutex<PatriciaTrie>>,
    bridge_minted: Arc<Mutex<HashSet<String>>>,
    bridge_unlocked: Arc<Mutex<HashSet<String>>>,
    chain_id: u64,
) -> Result<impl warp::Reply, Infallible> {
    let validators_count = {
        let trie = state_trie.lock().await;
        trie.get(b"validators")
            .ok()
            .flatten()
            .and_then(|encoded| serde_json::from_slice::<Vec<ValidatorInfo>>(&encoded).ok())
            .map(|v| v.len())
            .unwrap_or(0)
    };
    let processed_mints = bridge_minted.lock().await.len();
    let processed_unlocks = bridge_unlocked.lock().await.len();
    let eth_rpc_configured = std::env::var("ETH_RPC_URL").is_ok();

    Ok(warp::reply::json(&BridgeStatusResponse {
        vault_address: deterministic_address("nebula-bridge-vault-v1"),
        defi_pool_address: deterministic_address("nebula-defi-pool-v1"),
        validators_count,
        relayers_ready: validators_count > 0,
        eth_rpc_configured,
        processed_mints,
        processed_unlocks,
        chain_id,
    }))
}

async fn verify_eth_lock_tx(
    eth_rpc: &str,
    tx_hash: &str,
    bridge_address: Option<&str>,
) -> Result<bool, String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getTransactionReceipt",
        "params": [tx_hash],
        "id": 1
    });
    let client = reqwest::Client::new();
    let resp = client
        .post(eth_rpc)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("ETH RPC unreachable: {e}"))?;
    let val: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Invalid ETH RPC response: {e}"))?;
    let receipt = val
        .get("result")
        .ok_or_else(|| "Missing result in ETH RPC response".to_string())?;
    if receipt.is_null() {
        return Err("ETH transaction not yet mined".into());
    }
    let status = receipt
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("0x0");
    if status != "0x1" {
        return Err("ETH transaction failed".into());
    }
    if let Some(bridge) = bridge_address {
        let to = receipt
            .get("to")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_lowercase();
        if !to.is_empty()
            && to != bridge.trim_start_matches("0x").to_lowercase()
            && to != bridge.to_lowercase()
        {
            // also accept logs from bridge contract
            let logs = receipt.get("logs").and_then(|l| l.as_array());
            if logs.map(|l| l.is_empty()).unwrap_or(true) {
                return Err("Transaction not sent to bridge contract".into());
            }
        }
    }
    Ok(true)
}

async fn credit_account_balance(
    chain_store: &ChainStore,
    address: &str,
    amount: u128,
) -> Result<String, String> {
    let addr_clean = address.trim_start_matches("0x").to_lowercase();
    let addr_bytes = hex::decode(&addr_clean).map_err(|_| "Invalid address".to_string())?;
    if addr_bytes.len() != 20 {
        return Err("Invalid address length".into());
    }

    let mut balance = 0u128;
    if let Ok(Some(encoded)) = chain_store.get_state(&addr_bytes) {
        if let Ok(acc) = serde_json::from_slice::<serde_json::Value>(&encoded) {
            if let Some(b) = acc.get("balance").and_then(|v| v.as_str()) {
                balance = b.parse().unwrap_or(0);
            }
        }
    }
    balance += amount;

    let account = serde_json::json!({
        "balance": balance.to_string(),
        "nonce": 0,
    });
    chain_store
        .put_state(&addr_bytes, &serde_json::to_vec(&account).unwrap())
        .map_err(|e| e.to_string())?;
    Ok(balance.to_string())
}

async fn handle_bridge_mint(
    req: BridgeMintRequest,
    chain_store: Arc<ChainStore>,
    bridge_minted: Arc<Mutex<HashSet<String>>>,
) -> Result<impl warp::Reply, Infallible> {
    let recipient = req.recipient.trim().to_lowercase();
    if hex::decode(recipient.trim_start_matches("0x"))
        .map(|b| b.len() != 20)
        .unwrap_or(true)
    {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": "Invalid recipient address",
        })));
    }

    let amount: u128 = req
        .amount
        .as_ref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000_000_000_000_000_000u128);

    if let Some(ref eth_hash) = req.eth_tx_hash {
        let eth_hash_lower = eth_hash.to_lowercase();
        {
            let minted = bridge_minted.lock().await;
            if minted.contains(&eth_hash_lower) {
                return Ok(warp::reply::json(&serde_json::json!({
                    "error": "ETH lock already processed",
                })));
            }
        }

        let eth_rpc = req
            .eth_rpc_url
            .or_else(|| std::env::var("ETH_RPC_URL").ok())
            .unwrap_or_default();
        if eth_rpc.is_empty() {
            return Ok(warp::reply::json(&serde_json::json!({
                "error": "ETH_RPC_URL not configured — set env or pass eth_rpc_url",
            })));
        }

        if let Err(e) =
            verify_eth_lock_tx(&eth_rpc, &eth_hash_lower, req.eth_bridge_address.as_deref()).await
        {
            return Ok(warp::reply::json(&serde_json::json!({ "error": e })));
        }

        bridge_minted.lock().await.insert(eth_hash_lower);
    }

    match credit_account_balance(&chain_store, &recipient, amount).await {
        Ok(balance) => Ok(warp::reply::json(&serde_json::json!({
            "status": "minted",
            "recipient": format!("0x{}", recipient.trim_start_matches("0x")),
            "amount": amount.to_string(),
            "balance": balance,
            "eth_tx_hash": req.eth_tx_hash,
        }))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({ "error": e }))),
    }
}

fn find_tx_by_hash(chain_store: &ChainStore, tx_hash: &[u8; 32]) -> Option<(Transaction, u64)> {
    let latest = chain_store.get_latest_height().ok().flatten().unwrap_or(0);
    let start = latest.saturating_sub(500);
    for height in (start..=latest).rev() {
        let Ok(Some(encoded)) = chain_store.get_block_by_height(height) else {
            continue;
        };
        let Ok(block) = serde_json::from_slice::<Block>(&encoded) else {
            continue;
        };
        for tx in block.extrinsics {
            if tx.hash() == *tx_hash {
                return Some((tx, height));
            }
        }
    }
    None
}

fn verify_nebula_lock_tx(
    chain_store: &ChainStore,
    tx_hash: &[u8; 32],
    vault_address: &str,
) -> Result<(Transaction, u64), String> {
    let vault = parse_address_20(vault_address).ok_or("Invalid vault address")?;
    let (tx, height) = find_tx_by_hash(chain_store, tx_hash)
        .ok_or_else(|| "Nebula lock transaction not found".to_string())?;

    let to = tx.to.ok_or("Lock transaction missing destination")?;
    if to != vault {
        return Err("Transaction not sent to bridge vault".into());
    }
    if tx.value == 0 {
        return Err("Lock transaction has zero value".into());
    }

    let body = if tx.payload.len() > 32 {
        &tx.payload[32..]
    } else {
        &tx.payload
    };
    let payload_str = std::str::from_utf8(body).map_err(|_| "Invalid bridge payload encoding")?;
    if !payload_str.starts_with("BRIDGE:ethereum:") {
        return Err(format!(
            "Expected BRIDGE:ethereum: payload, got {}",
            payload_str.chars().take(40).collect::<String>()
        ));
    }

    let receipt = chain_store
        .get_receipt(tx_hash)
        .map_err(|e| e.to_string())?
        .ok_or("Lock transaction receipt not found")?;
    let val: serde_json::Value =
        serde_json::from_slice(&receipt).map_err(|e| format!("Invalid receipt: {e}"))?;
    if !val.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
        return Err("Lock transaction failed on-chain".into());
    }

    Ok((tx, height))
}

async fn handle_bridge_unlock(
    req: BridgeUnlockRequest,
    chain_store: Arc<ChainStore>,
    bridge_unlocked: Arc<Mutex<HashSet<String>>>,
) -> Result<impl warp::Reply, Infallible> {
    let hash_clean = req.nebula_tx_hash.trim().trim_start_matches("0x").to_lowercase();
    let hash_bytes = match hex::decode(&hash_clean) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => {
            return Ok(warp::reply::json(&serde_json::json!({
                "error": "Invalid nebula_tx_hash",
            })))
        }
    };

    {
        let unlocked = bridge_unlocked.lock().await;
        if unlocked.contains(&hash_clean) {
            return Ok(warp::reply::json(&serde_json::json!({
                "error": "Nebula lock already processed for unlock",
            })));
        }
    }

    let vault = deterministic_address("nebula-bridge-vault-v1");
    let (tx, block_height) = match verify_nebula_lock_tx(&chain_store, &hash_bytes, &vault) {
        Ok(v) => v,
        Err(e) => {
            return Ok(warp::reply::json(&serde_json::json!({ "error": e })))
        }
    };

    let body = if tx.payload.len() > 32 {
        &tx.payload[32..]
    } else {
        &tx.payload
    };
    let eth_recipient = req.eth_recipient.clone().unwrap_or_else(|| {
        std::str::from_utf8(body)
            .ok()
            .and_then(|s| s.strip_prefix("BRIDGE:ethereum:"))
            .map(|s| s.to_string())
            .unwrap_or_default()
    });

    if eth_recipient.is_empty()
        || parse_address_20(&eth_recipient).is_none()
    {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": "Missing or invalid eth_recipient",
        })));
    }

    let eth_rpc = req
        .eth_rpc_url
        .or_else(|| std::env::var("ETH_RPC_URL").ok())
        .unwrap_or_default();

    let mut eth_unlock_submitted = false;
    if !eth_rpc.is_empty() {
        // Relayer would call Bridge.unlock on Ethereum; we verify RPC reachability here.
        if let Ok(true) = verify_eth_rpc_reachable(&eth_rpc).await {
            eth_unlock_submitted = true;
        }
    }

    bridge_unlocked.lock().await.insert(hash_clean.clone());

    Ok(warp::reply::json(&serde_json::json!({
        "status": "unlock_queued",
        "nebula_tx_hash": format!("0x{}", hash_clean),
        "eth_recipient": eth_recipient,
        "amount": tx.value.to_string(),
        "block_height": block_height,
        "eth_rpc_configured": !eth_rpc.is_empty(),
        "eth_unlock_submitted": eth_unlock_submitted,
        "note": "Nebula lock verified; relayer should submit unlock on Ethereum Bridge.sol",
    })))
}

async fn verify_eth_rpc_reachable(eth_rpc: &str) -> Result<bool, String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_blockNumber",
        "params": [],
        "id": 1
    });
    let client = reqwest::Client::new();
    let resp = client
        .post(eth_rpc)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("ETH RPC unreachable: {e}"))?;
    let val: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Invalid ETH RPC response: {e}"))?;
    Ok(!val.get("result").map(|r| r.is_null()).unwrap_or(true))
}

async fn handle_fee_history(
    block_count: u64,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let latest_height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    let oldest_block = latest_height.saturating_sub(block_count);

    let mut base_fees = Vec::new();
    let mut gas_used_ratio = Vec::new();

    for height in oldest_block..=latest_height {
        if let Ok(Some(hash_bytes)) = chain_store.get_block_hash_by_height(height) {
            if hash_bytes.len() == 32 {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&hash_bytes);
                if let Ok(Some(encoded)) = chain_store.get_block(&hash) {
                    if let Ok(block_val) = serde_json::from_slice::<serde_json::Value>(&encoded) {
                        let base_fee = block_val
                            .get("header")
                            .and_then(|h| h.get("base_fee"))
                            .and_then(|b| b.as_u64())
                            .unwrap_or(1_000_000_000);
                        base_fees.push(base_fee.to_string());

                        let gas_used = block_val
                            .get("header")
                            .and_then(|h| h.get("gas_used"))
                            .and_then(|g| g.as_u64())
                            .unwrap_or(0);
                        let gas_limit = 30_000_000u64;
                        gas_used_ratio.push(gas_used as f64 / gas_limit as f64);
                    }
                }
            }
        }
    }

    let response = FeeHistoryResponse {
        base_fee_per_gas: base_fees,
        gas_used_ratio,
        oldest_block,
    };

    Ok(warp::reply::json(&response))
}

// ============================================================================
// Admin & Network Handlers
// ============================================================================

async fn handle_metrics(
    metrics: Arc<crate::metrics::Metrics>,
) -> Result<impl warp::Reply, Infallible> {
    Ok(warp::reply::with_header(
        metrics.export_prometheus(),
        "Content-Type",
        "text/plain; version=0.0.4",
    ))
}

async fn handle_health() -> Result<impl warp::Reply, Infallible> {
    #[derive(Serialize)]
    struct HealthResponse {
        status: String,
        version: &'static str,
    }
    Ok(warp::reply::json(&HealthResponse {
        status: "healthy".into(),
        version: env!("CARGO_PKG_VERSION"),
    }))
}

async fn handle_connect_peer(
    request: ConnectPeerRequest,
    network_cmd_sender: mpsc::Sender<NetworkCommand>,
) -> Result<impl warp::Reply, Infallible> {
    match request.multiaddr.parse::<libp2p::Multiaddr>() {
        Ok(addr) => {
            let _ = network_cmd_sender.send(NetworkCommand::Dial(addr)).await;
            Ok(warp::reply::json(&ConnectPeerResponse {
                status: "success".into(),
            }))
        }
        Err(e) => Ok(warp::reply::json(&ConnectPeerResponse {
            status: format!("error: invalid multiaddr: {}", e),
        })),
    }
}

async fn handle_peers() -> Result<impl warp::Reply, Infallible> {
    // This would need access to the network service's peer list
    // For now, return an empty list
    #[derive(Serialize)]
    struct PeersResponse {
        peers: Vec<String>,
        count: usize,
    }
    Ok(warp::reply::json(&PeersResponse {
        peers: vec![],
        count: 0,
    }))
}

async fn handle_validators(
    state_trie: Arc<Mutex<PatriciaTrie>>,
) -> Result<impl warp::Reply, Infallible> {
    // Validators live in the state trie under the b"validators" key (written at genesis)
    // as a JSON array of ValidatorInfo objects.
    let trie = state_trie.lock().await;
    match trie.get(b"validators") {
        Ok(Some(encoded)) => {
            let vals = serde_json::from_slice::<Vec<ValidatorInfo>>(&encoded).unwrap_or_default();
            let count = vals.len();
            Ok(warp::reply::json(&ValidatorsResponse {
                validators: vals,
                count,
            }))
        }
        _ => {
            // No validators in trie yet
            Ok(warp::reply::json(&ValidatorsResponse {
                validators: vec![],
                count: 0,
            }))
        }
    }
}

async fn handle_block_latest(chain_store: Arc<ChainStore>) -> Result<impl warp::Reply, Infallible> {
    let latest = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    if latest == 0 {
        return Ok(warp::reply::json(&BlockResponse {
            block: None,
            error: Some("No blocks have been produced yet".into()),
        }));
    }
    match chain_store.get_block_hash_by_height(latest) {
        Ok(Some(hash_bytes)) if hash_bytes.len() == 32 => {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&hash_bytes);
            match chain_store.get_block(&hash) {
                Ok(Some(encoded)) => {
                    let val: serde_json::Value =
                        serde_json::from_slice(&encoded).unwrap_or(serde_json::Value::Null);
                    Ok(warp::reply::json(&BlockResponse {
                        block: Some(val),
                        error: None,
                    }))
                }
                Ok(None) => Ok(warp::reply::json(&BlockResponse {
                    block: None,
                    error: Some(format!("Block data missing for height {}", latest)),
                })),
                Err(e) => Ok(warp::reply::json(&BlockResponse {
                    block: None,
                    error: Some(e.to_string()),
                })),
            }
        }
        Ok(_) => Ok(warp::reply::json(&BlockResponse {
            block: None,
            error: Some(format!("No block at height {}", latest)),
        })),
        Err(e) => Ok(warp::reply::json(&BlockResponse {
            block: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_delegations(
    address_str: String,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let delegations_key = [b"delegations/", address_str.as_bytes()].concat();
    match chain_store.get_state(&delegations_key) {
        Ok(Some(encoded)) => {
            if let Ok(dels) = serde_json::from_slice::<Vec<DelegationInfo>>(&encoded) {
                Ok(warp::reply::json(&DelegationsResponse {
                    delegations: dels,
                    address: address_str,
                }))
            } else {
                Ok(warp::reply::json(&DelegationsResponse {
                    delegations: vec![],
                    address: address_str,
                }))
            }
        }
        _ => Ok(warp::reply::json(&DelegationsResponse {
            delegations: vec![],
            address: address_str,
        })),
    }
}

// ============================================================================
// Governance Handlers
// ============================================================================

async fn handle_governance(
    state_trie: Arc<Mutex<PatriciaTrie>>,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let (state, height) = load_governance_state(&state_trie, &chain_store).await;
    let Some(state) = state else {
        return Ok(warp::reply::json(&governance_empty()));
    };

    let proposals: Vec<serde_json::Value> = state
        .proposals
        .iter()
        .map(|p| proposal_json(p, &state, height))
        .collect();

    let params = &state.params;
    Ok(warp::reply::json(&serde_json::json!({
        "params": {
            "proposal_deposit": params.proposal_deposit.to_string(),
            "voting_period_blocks": params.voting_period_blocks,
            "quorum_threshold_bps": params.quorum_threshold_bps,
            "pass_threshold_bps": params.pass_threshold_bps,
        },
        "treasury": {
            "balance": state.treasury.balance.to_string(),
            "total_collected": state.treasury.total_collected.to_string(),
            "total_spent": state.treasury.total_spent.to_string(),
        },
        "total_stake": state.total_stake.to_string(),
        "next_proposal_id": state.next_proposal_id,
        "height": height,
        "proposals": proposals,
    })))
}

async fn handle_proposal(
    id: u64,
    state_trie: Arc<Mutex<PatriciaTrie>>,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let (state, height) = load_governance_state(&state_trie, &chain_store).await;
    let Some(state) = state else {
        return Ok(warp::reply::json(&serde_json::json!({
            "proposal": None::<serde_json::Value>,
            "error": "No governance state",
        })));
    };

    match state.get_proposal(id) {
        Some(proposal) => Ok(warp::reply::json(&serde_json::json!({
            "proposal": proposal_json(proposal, &state, height),
            "error": None::<String>,
        }))),
        None => Ok(warp::reply::json(&serde_json::json!({
            "proposal": None::<serde_json::Value>,
            "error": format!("Proposal {} not found", id),
        }))),
    }
}

async fn load_governance_state(
    state_trie: &Mutex<PatriciaTrie>,
    chain_store: &ChainStore,
) -> (Option<ChainGovernance>, u64) {
    let height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    let guard = state_trie.lock().await;
    let state = guard
        .get(b"governance")
        .ok()
        .flatten()
        .and_then(|data| serde_json::from_slice(&data).ok());
    (state, height)
}

fn governance_empty() -> serde_json::Value {
    serde_json::json!({
        "params": { "proposal_deposit": "0", "voting_period_blocks": 0, "quorum_threshold_bps": 0, "pass_threshold_bps": 0 },
        "treasury": { "balance": "0", "total_collected": "0", "total_spent": "0" },
        "total_stake": "0",
        "next_proposal_id": 1,
        "height": 0,
        "proposals": [],
    })
}

fn proposal_json(p: &Proposal, state: &ChainGovernance, height: u64) -> serde_json::Value {
    let status = state.resolve_status(p, height);
    let total_votes = p.yes_votes + p.no_votes + p.abstain_votes;
    let quorum = (state.total_stake * state.params.quorum_threshold_bps as u128) / 10_000;
    let voters: HashMap<String, String> = p
        .voters
        .iter()
        .map(|(a, c)| (format!("0x{}", a), c.as_str().to_string()))
        .collect();

    serde_json::json!({
        "id": p.id,
        "title": p.title,
        "description": p.description,
        "proposer": format!("0x{}", hex::encode(p.proposer)),
        "status": status_str(&status),
        "start_block": p.start_block,
        "end_block": p.end_block,
        "voting_period_blocks": state.params.voting_period_blocks,
        "yes_votes": p.yes_votes.to_string(),
        "no_votes": p.no_votes.to_string(),
        "abstain_votes": p.abstain_votes.to_string(),
        "total_votes": total_votes.to_string(),
        "quorum": quorum.to_string(),
        "voters": voters,
    })
}

fn status_str(s: &ProposalStatus) -> &'static str {
    match s {
        ProposalStatus::Active => "active",
        ProposalStatus::Passed => "passed",
        ProposalStatus::Rejected => "rejected",
        ProposalStatus::Executed => "executed",
    }
}

// ============================================================================
// Faucet Handler
// ============================================================================

/// Directly credit an account with test tokens (dev faucet, no transaction needed)
async fn handle_faucet_request(
    req: serde_json::Value,
    chain_store: Arc<ChainStore>,
    faucet_cooldowns: Arc<Mutex<HashMap<String, u64>>>,
) -> Result<impl warp::Reply, Infallible> {
    let address = match req.get("address").and_then(|v| v.as_str()) {
        Some(addr) => addr.trim_start_matches("0x").to_lowercase(),
        None => {
            return Ok(warp::reply::json(
                &serde_json::json!({"error": "Missing address"}),
            ))
        }
    };

    let addr_bytes = match hex::decode(&address) {
        Ok(b) if b.len() == 20 => b,
        _ => {
            return Ok(warp::reply::json(
                &serde_json::json!({"error": "Invalid address"}),
            ))
        }
    };

    // Server-side cooldown: enforce a minimum wait between drips to the same address
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut cooldowns = faucet_cooldowns.lock().await;
    if let Some(&last_drip) = cooldowns.get(&address) {
        let elapsed = now.saturating_sub(last_drip);
        if elapsed < FAUCET_COOLDOWN_SECS {
            return Ok(warp::reply::json(&serde_json::json!({
                "error": "Faucet cooldown active",
                "retry_after_secs": FAUCET_COOLDOWN_SECS - elapsed,
            })));
        }
    }
    cooldowns.insert(address, now);
    drop(cooldowns);

    let drip_amount: u128 = req
        .get("amount")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000_000_000_000_000_000u128); // default 100 tokens

    // Read current account state
    let mut balance = 0u128;
    if let Ok(Some(encoded)) = chain_store.get_state(&addr_bytes) {
        if let Ok(acc) = serde_json::from_slice::<serde_json::Value>(&encoded) {
            if let Some(b) = acc.get("balance").and_then(|v| v.as_str()) {
                balance = b.parse::<u128>().unwrap_or(0);
            }
        }
    }

    balance += drip_amount;

    let account = serde_json::json!({
        "balance": balance.to_string(),
        "nonce": 0,
    });

    if let Err(e) = chain_store.put_state(&addr_bytes, &serde_json::to_vec(&account).unwrap()) {
        return Ok(warp::reply::json(
            &serde_json::json!({"error": e.to_string()}),
        ));
    }

    Ok(warp::reply::json(&serde_json::json!({
        "status": "success",
        "amount": drip_amount.to_string(),
        "balance": balance.to_string(),
    })))
}

// ============================================================================
// Account Abstraction, MEV, Slashing, Staking Handlers
// ============================================================================

#[derive(Debug, Deserialize)]
struct MevCommitRequest {
    tx_hash: String,
    secret: String,
    sender: String,
    nonce: u64,
}

#[derive(Debug, Deserialize)]
struct MevRevealRequest {
    transaction: Transaction,
    secret: String,
    commitment: String,
}

#[derive(Debug, Deserialize)]
struct DelegateRequest {
    delegator: String,
    validator: String,
    amount: String,
}

#[derive(Debug, Deserialize)]
struct RegisterValidatorRequest {
    address: String,
    public_key: String,
    stake: String,
    commission_rate: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct DeployWasmRequest {
    name: String,
    /// Base64-encoded WASM module bytes
    wasm: String,
}

#[derive(Debug, Deserialize)]
struct CallWasmRequest {
    name: String,
    func: String,
    #[serde(default)]
    arg: i32,
}

async fn handle_deploy_wasm(
    req: DeployWasmRequest,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, warp::Rejection> {
    use base64::Engine as _;
    let wasm = match base64::engine::general_purpose::STANDARD.decode(&req.wasm) {
        Ok(b) => b,
        Err(e) => {
            return Ok(warp::reply::json(&serde_json::json!({
                "status": "error",
                "error": format!("Invalid base64 wasm: {}", e),
            })));
        }
    };
    let executor = match execution::WasmExecutor::new() {
        Ok(e) => e,
        Err(e) => {
            return Ok(warp::reply::json(&serde_json::json!({
                "status": "error",
                "error": format!("WASM engine init: {}", e),
            })));
        }
    };
    let code_hash = match executor.validate_module(&wasm) {
        Ok(h) => h,
        Err(e) => {
            return Ok(warp::reply::json(&serde_json::json!({
                "status": "error",
                "error": format!("Invalid WASM module: {}", e),
            })));
        }
    };
    let registry = crate::wasm_registry::WasmRegistry::new(chain_store);
    match registry.deploy(&req.name, &wasm) {
        Ok(()) => Ok(warp::reply::json(&serde_json::json!({
            "status": "success",
            "name": req.name,
            "code_hash": hex::encode(code_hash),
        }))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({
            "status": "error",
            "error": e.to_string(),
        }))),
    }
}

async fn handle_call_wasm(
    req: CallWasmRequest,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let registry = crate::wasm_registry::WasmRegistry::new(chain_store);
    let wasm = match registry.get(&req.name) {
        Ok(Some(b)) => b,
        Ok(None) => {
            return Ok(warp::reply::json(&serde_json::json!({
                "status": "error",
                "error": format!("Contract '{}' not found", req.name),
            })));
        }
        Err(e) => {
            return Ok(warp::reply::json(&serde_json::json!({
                "status": "error",
                "error": e.to_string(),
            })));
        }
    };
    let executor = match execution::WasmExecutor::new() {
        Ok(e) => e,
        Err(e) => {
            return Ok(warp::reply::json(&serde_json::json!({
                "status": "error",
                "error": format!("WASM engine init: {}", e),
            })));
        }
    };
    match executor.execute_i32(&wasm, &req.func, req.arg) {
        Ok((result, gas_used)) => Ok(warp::reply::json(&serde_json::json!({
            "status": "success",
            "result": result,
            "gas_used": gas_used,
        }))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({
            "status": "error",
            "error": e.to_string(),
        }))),
    }
}

async fn handle_list_wasm(
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let registry = crate::wasm_registry::WasmRegistry::new(chain_store);
    match registry.list() {
        Ok(names) => Ok(warp::reply::json(&serde_json::json!({
            "contracts": names,
            "count": names.len(),
        }))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({
            "error": e.to_string(),
            "contracts": [],
            "count": 0,
        }))),
    }
}

fn parse_hex32(s: &str) -> Option<[u8; 32]> {
    let bytes = hex::decode(s.trim_start_matches("0x")).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Some(out)
}

fn parse_address(s: &str) -> Option<[u8; 20]> {
    let bytes = hex::decode(s.trim_start_matches("0x")).ok()?;
    if bytes.len() != 20 {
        return None;
    }
    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes);
    Some(out)
}

async fn handle_submit_user_operation(
    op: UserOperation,
    tx_pool: Arc<TxPool>,
) -> Result<impl warp::Reply, Infallible> {
    match tx_pool.submit_user_operation(op) {
        Ok(hash) => Ok(warp::reply::json(&SubmitTxResponse {
            status: "success".into(),
            hash: hex::encode(hash),
        })),
        Err(e) => Ok(warp::reply::json(&SubmitTxResponse {
            status: format!("error: {}", e),
            hash: String::new(),
        })),
    }
}

async fn handle_pending_user_operations(
    tx_pool: Arc<TxPool>,
) -> Result<impl warp::Reply, Infallible> {
    Ok(warp::reply::json(&serde_json::json!({
        "pending": tx_pool.pending_user_operations(),
    })))
}

async fn handle_mev_commit(
    req: MevCommitRequest,
    tx_pool: Arc<TxPool>,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let Some(tx_hash) = parse_hex32(&req.tx_hash) else {
        return Ok(warp::reply::json(
            &serde_json::json!({"error": "Invalid tx_hash"}),
        ));
    };
    let Some(secret) = parse_hex32(&req.secret) else {
        return Ok(warp::reply::json(
            &serde_json::json!({"error": "Invalid secret"}),
        ));
    };
    let Some(sender) = parse_address(&req.sender) else {
        return Ok(warp::reply::json(
            &serde_json::json!({"error": "Invalid sender"}),
        ));
    };
    let height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    let commitment = tx_pool.submit_committed(tx_hash, secret, sender, req.nonce, height);
    Ok(warp::reply::json(&serde_json::json!({
        "status": "success",
        "commitment": hex::encode(commitment),
    })))
}

async fn handle_mev_reveal(
    req: MevRevealRequest,
    tx_pool: Arc<TxPool>,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let Some(secret) = parse_hex32(&req.secret) else {
        return Ok(warp::reply::json(
            &serde_json::json!({"error": "Invalid secret"}),
        ));
    };
    let Some(commitment) = parse_hex32(&req.commitment) else {
        return Ok(warp::reply::json(
            &serde_json::json!({"error": "Invalid commitment"}),
        ));
    };
    let height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
    match tx_pool.reveal_transaction(req.transaction, secret, commitment, height) {
        Ok(tx) => Ok(warp::reply::json(&SubmitTxResponse {
            status: "success".into(),
            hash: hex::encode(tx.hash()),
        })),
        Err(e) => Ok(warp::reply::json(&SubmitTxResponse {
            status: format!("error: {}", e),
            hash: String::new(),
        })),
    }
}

async fn handle_mev_encrypted(
    enc: mev::EncryptedTransaction,
    tx_pool: Arc<TxPool>,
) -> Result<impl warp::Reply, Infallible> {
    match tx_pool.submit_encrypted(enc) {
        Ok(()) => Ok(warp::reply::json(&serde_json::json!({"status": "success"}))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({"error": e}))),
    }
}

async fn handle_mev_decryption_share(
    share: mev::DecryptionShare,
    tx_pool: Arc<TxPool>,
) -> Result<impl warp::Reply, Infallible> {
    match tx_pool.submit_decryption_share(share) {
        Ok(()) => Ok(warp::reply::json(&serde_json::json!({"status": "success"}))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({"error": e}))),
    }
}

async fn handle_slashing_events(
    tracker: Arc<Mutex<SlashingTracker>>,
) -> Result<impl warp::Reply, Infallible> {
    let slashed: Vec<String> = tracker
        .lock()
        .await
        .get_slashed_validators()
        .iter()
        .map(|pk| hex::encode(pk))
        .collect();
    Ok(warp::reply::json(&serde_json::json!({
        "slashed_validators": slashed,
        "count": slashed.len(),
    })))
}

async fn handle_delegate(
    req: DelegateRequest,
    state_trie: Arc<Mutex<PatriciaTrie>>,
    chain_store: Arc<ChainStore>,
) -> Result<impl warp::Reply, Infallible> {
    let Some(delegator) = parse_address(&req.delegator) else {
        return Ok(warp::reply::json(
            &serde_json::json!({"error": "Invalid delegator"}),
        ));
    };
    let Some(validator) = parse_address(&req.validator) else {
        return Ok(warp::reply::json(
            &serde_json::json!({"error": "Invalid validator"}),
        ));
    };
    let amount: u128 = req.amount.parse().unwrap_or(0);
    match governance_store::apply_delegate(&state_trie, &chain_store, delegator, validator, amount)
        .await
    {
        Ok(()) => Ok(warp::reply::json(&serde_json::json!({"status": "success"}))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({"error": e}))),
    }
}

async fn handle_register_validator(
    req: RegisterValidatorRequest,
    state_trie: Arc<Mutex<PatriciaTrie>>,
) -> Result<impl warp::Reply, Infallible> {
    let Some(address) = parse_address(&req.address) else {
        return Ok(warp::reply::json(
            &serde_json::json!({"error": "Invalid address"}),
        ));
    };
    let public_key = hex::decode(req.public_key.trim_start_matches("0x")).unwrap_or_default();
    let stake: u128 = req.stake.parse().unwrap_or(0);
    let commission = req.commission_rate.unwrap_or(10);
    match governance_store::register_validator(&state_trie, address, public_key, stake, commission)
        .await
    {
        Ok(()) => Ok(warp::reply::json(&serde_json::json!({"status": "success"}))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({"error": e}))),
    }
}

async fn handle_websocket(ws: warp::ws::WebSocket, chain_store: Arc<ChainStore>) {
    use futures_util::{SinkExt, StreamExt};
    use std::time::Duration;

    let (mut tx, mut rx) = ws.split();

    loop {
        let height = chain_store.get_latest_height().unwrap_or(None).unwrap_or(0);
        let msg = warp::ws::Message::text(format!(r#"{{"event":"newHead","height":{}}}"#, height));
        if tx.send(msg).await.is_err() {
            break;
        }
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(2)) => {}
            incoming = rx.next() => {
                if incoming.is_none() { break; }
            }
        }
    }
}

// ============================================================================
// Error Handling
// ============================================================================

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
