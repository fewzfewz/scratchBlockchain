//! Request and response bodies.

use super::*;

#[derive(Debug, Serialize)]
pub(super) struct StatusResponse {
    pub(super) height: u64,
    pub(super) finalized_height: Option<u64>,
    pub(super) mempool_size: usize,
    pub(super) peer_count: usize,
    pub(super) chain_id: u64,
}

#[derive(Debug, Serialize)]
pub(super) struct MempoolResponse {
    pub(super) size: usize,
    pub(super) transactions: Vec<Transaction>,
}

#[derive(Debug, Serialize)]
pub(super) struct SubmitTxResponse {
    pub(super) status: String,
    pub(super) hash: String,
}

#[derive(Debug, Serialize)]
pub(super) struct BlockResponse {
    pub(super) block: Option<serde_json::Value>,
    pub(super) error: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct BalanceResponse {
    pub(super) address: String,
    pub(super) balance: String,
    pub(super) nonce: u64,
}

#[derive(Debug, Serialize)]
pub(super) struct ReceiptResponse {
    pub(super) receipt: Option<serde_json::Value>,
    pub(super) error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TxHistoryQuery {
    pub(super) limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub(super) struct TxHistoryEntry {
    pub(super) hash: String,
    pub(super) block_height: u64,
    pub(super) sender: String,
    pub(super) to: Option<String>,
    pub(super) value: u64,
    pub(super) nonce: u64,
    pub(super) is_contract_creation: bool,
    pub(super) status: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct TxHistoryResponse {
    pub(super) address: String,
    pub(super) transactions: Vec<TxHistoryEntry>,
    pub(super) scanned_blocks: u64,
}

#[derive(Debug, Deserialize)]
pub(super) struct ConnectPeerRequest {
    pub(super) multiaddr: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ConnectPeerResponse {
    pub(super) status: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ErrorResponse {
    pub(super) error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct ValidatorInfo {
    pub(super) address: String,
    public_key: String,
    pub(super) stake: String,
    pub(super) commission_rate: u64,
    pub(super) is_active: bool,
    pub(super) blocks_produced: u64,
    pub(super) blocks_missed: u64,
    pub(super) delegator_count: u64,
    pub(super) total_delegated: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ValidatorsResponse {
    pub(super) validators: Vec<ValidatorInfo>,
    pub(super) count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct DelegationInfo {
    pub(super) validator_address: String,
    pub(super) amount: String,
}

#[derive(Debug, Serialize)]
pub(super) struct DelegationsResponse {
    pub(super) delegations: Vec<DelegationInfo>,
    pub(super) address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct GasPriceResponse {
    pub(super) base_fee: String,
    pub(super) suggested_priority_fee_low: String,
    pub(super) suggested_priority_fee_medium: String,
    pub(super) suggested_priority_fee_high: String,
    pub(super) block_height: u64,
}

#[derive(Debug, Deserialize)]
pub(super) struct EstimateGasRequest {
    pub(super) from: String,
    /// Empty, "0x" or the zero address for a contract creation.
    #[serde(default)]
    pub(super) to: String,
    #[serde(default)]
    pub(super) data: String,
    pub(super) value: Option<String>,
    pub(super) max_fee_per_gas: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct EstimateGasResponse {
    pub(super) estimated_gas: u64,
    pub(super) base_fee: String,
    pub(super) total_cost_estimate: String,
    pub(super) estimated_priority_fee: String,
    /// Set when the simulated execution reverted or halted: the transaction
    /// would fail with any gas limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct FeeHistoryResponse {
    pub(super) base_fee_per_gas: Vec<String>,
    pub(super) gas_used_ratio: Vec<f64>,
    pub(super) oldest_block: u64,
}

#[derive(Debug, Deserialize)]
pub(super) struct CallContractRequest {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) data: String,
    pub(super) value: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct CallContractResponse {
    pub(super) result: String,
    pub(super) success: bool,
    pub(super) error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct BridgeMintRequest {
    pub(super) recipient: String,
    pub(super) amount: Option<String>,
    pub(super) eth_tx_hash: Option<String>,
    /// Ignored: the node only trusts its own `ETH_RPC_URL`.
    pub(super) eth_rpc_url: Option<String>,
    pub(super) eth_bridge_address: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct BridgeUnlockRequest {
    pub(super) nebula_tx_hash: String,
    pub(super) eth_recipient: Option<String>,
    pub(super) eth_rpc_url: Option<String>,
    #[allow(dead_code)] // accepted for API compatibility; not used yet
    pub(super) eth_bridge_address: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct BridgeUnlockAckRequest {
    pub(super) nebula_tx_hash: String,
    pub(super) eth_tx_hash: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct BridgeStatusResponse {
    pub(super) vault_address: String,
    pub(super) defi_pool_address: String,
    pub(super) validators_count: usize,
    pub(super) relayers_ready: bool,
    /// Size of the genesis relayer set and the attestations a mint needs.
    pub(super) relayer_count: usize,
    pub(super) relayer_threshold: u32,
    pub(super) eth_rpc_configured: bool,
    pub(super) processed_mints: usize,
    pub(super) processed_unlocks: usize,
    pub(super) pending_unlocks: usize,
    pub(super) chain_id: u64,
}

#[derive(Debug, Deserialize)]
pub(super) struct MevCommitRequest {
    pub(super) tx_hash: String,
    pub(super) secret: String,
    pub(super) sender: String,
    pub(super) nonce: u64,
}

#[derive(Debug, Deserialize)]
pub(super) struct MevRevealRequest {
    pub(super) transaction: Transaction,
    pub(super) secret: String,
    pub(super) commitment: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct DeployWasmRequest {
    pub(super) name: String,
    /// Base64-encoded WASM module bytes
    pub(super) wasm: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct CallWasmRequest {
    pub(super) name: String,
    pub(super) func: String,
    #[serde(default)]
    pub(super) arg: i32,
}
