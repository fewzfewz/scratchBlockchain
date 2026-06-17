export type BigNumberish = string | number | bigint;
export type Address = string;
export type Hash = string;
export type HexString = string;

// ============================================================================
// Transaction Types (aligned with Rust common::types::Transaction)
// ============================================================================

export interface TransactionRequest {
  to?: Address;
  value?: BigNumberish;
  data?: HexString;
  gasLimit?: BigNumberish;
  maxFeePerGas?: BigNumberish;
  maxPriorityFeePerGas?: BigNumberish;
  chainId?: number;
  nonce?: number;
}

export interface Transaction {
  sender: Address;
  nonce: number;
  payload: HexString;
  signature: HexString;
  gasLimit: number;
  maxFeePerGas: number;
  maxPriorityFeePerGas: number;
  chainId: number | null;
  to: Address | null;
  value: number;
  hash?: HexString;
}

export interface TransactionReceipt {
  tx_hash: Hash;
  block_hash: Hash;
  block_height: number;
  transaction_index: number;
  gas_used: string;
  cumulative_gas_used: string;
  status: string;
  from: Address;
  to: Address | null;
  contract_address: Address | null;
}

// ============================================================================
// Block Types
// ============================================================================

export interface BlockHeader {
  parent_hash: Hash;
  state_root: Hash;
  transactions_root: Hash;
  receipts_root: Hash;
  number: number;
  timestamp: number;
  proposer: Address;
  gas_used: string;
  gas_limit: string;
  base_fee: string;
  extra_data: HexString;
}

export interface Block {
  header: BlockHeader;
  transactions: Transaction[];
  hash: Hash;
}

// ============================================================================
// Account Types
// ============================================================================

export interface Account {
  address: Address;
  balance: string;
  nonce: number;
}

// ============================================================================
// Network Types
// ============================================================================

export interface PeerInfo {
  id: string;
  address: string;
  direction: "inbound" | "outbound";
  protocolVersion: string;
  bestHeight: number;
  latency?: number;
}

export interface NodeStatus {
  height: number;
  finalized_height: number | null;
  mempool_size: number;
  peer_count: number;
}

// ============================================================================
// Gas & Fee Types
// ============================================================================

export interface GasPriceResponse {
  base_fee: string;
  suggested_priority_fee_low: string;
  suggested_priority_fee_medium: string;
  suggested_priority_fee_high: string;
  block_height: number;
}

export interface EstimateGasResponse {
  estimated_gas: number;
  base_fee: string;
  total_cost_estimate: string;
  estimated_priority_fee: string;
}

export interface FeeHistoryResponse {
  base_fee_per_gas: string[];
  gas_used_ratio: number[];
  oldest_block: number;
}

// ============================================================================
// Mempool Types
// ============================================================================

export interface MempoolResponse {
  size: number;
  transactions: Transaction[];
}

// ============================================================================
// Governance Types
// ============================================================================

export interface Proposal {
  id: number;
  proposer: Address;
  title: string;
  description: string;
  proposalType: "ParameterChange" | "SoftwareUpgrade" | "TextProposal";
  startEpoch: number;
  endEpoch: number;
  yesVotes: string;
  noVotes: string;
  status: "Active" | "Passed" | "Rejected" | "Executed";
  deposit: string;
}

export interface Vote {
  proposalId: number;
  voter: Address;
  vote: boolean;
  votingPower: string;
  timestamp: number;
}

// ============================================================================
// Validator Types
// ============================================================================

export interface Validator {
  address: Address;
  publicKey: string;
  stake: string;
  commissionRate: number;
  isActive: boolean;
  blocksProduced: number;
  blocksMissed: number;
  lastActiveHeight: number;
  description?: string;
  website?: string;
}

export interface Delegation {
  delegator: Address;
  validator: Address;
  amount: string;
  rewards: string;
  createdHeight: number;
}

// ============================================================================
// Event Types
// ============================================================================

export interface NewBlockEvent {
  block: Block;
  height: number;
  timestamp: number;
}

export interface NewTransactionEvent {
  transaction: Transaction;
  addedAt: number;
}

export interface FinalizedBlockEvent {
  blockHash: Hash;
  blockNumber: number;
}

// ============================================================================
// Client Options
// ============================================================================

export interface ClientOptions {
  chainId?: number;
  timeout?: number;
  headers?: Record<string, string>;
  maxRetries?: number;
  retryDelay?: number;
}

// ============================================================================
// Response Wrappers
// ============================================================================

export interface RPCResponse<T = any> {
  jsonrpc: "2.0";
  id: number;
  result?: T;
  error?: RPCError;
}

export interface RPCError {
  code: number;
  message: string;
  data?: any;
}

// ============================================================================
// Subscription Types
// ============================================================================

export interface Subscription {
  id: string;
  type: "newBlocks" | "newTransactions" | "logs";
  filter?: LogFilter;
}

export interface LogFilter {
  addresses?: Address[];
  topics?: (string[] | null)[];
  fromBlock?: number | "earliest" | "latest" | "pending";
  toBlock?: number | "earliest" | "latest" | "pending";
}

// ============================================================================
// Utility Types
// ============================================================================

export type Bytes = Uint8Array;

// ============================================================================
// Constants
// ============================================================================

export const EMPTY_HASH = "0x0000000000000000000000000000000000000000000000000000000000000000";
export const EMPTY_ADDRESS = "0x0000000000000000000000000000000000000000";
export const NATIVE_TOKEN_SYMBOL = "NBL";
export const NATIVE_TOKEN_DECIMALS = 18;
