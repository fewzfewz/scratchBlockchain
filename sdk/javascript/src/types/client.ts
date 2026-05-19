// ============================================================================
// Core Types
// ============================================================================

export type BigNumberish = string | number | bigint;
export type Address = string;
export type Hash = string;
export type HexString = string;

// ============================================================================
// Block Types
// ============================================================================

export interface Block {
  number: number;
  hash: Hash;
  parentHash: Hash;
  timestamp: number;
  transactions: Hash[];
  stateRoot: Hash;
  validator: Address;
  gasUsed?: string;
  gasLimit?: string;
  baseFeePerGas?: string;
  size?: number;
  transactionCount?: number;
}

export interface BlockHeader {
  parentHash: Hash;
  stateRoot: Hash;
  extrinsicsRoot: Hash;
  slot: number;
  epoch: number;
  validatorSetId: number;
  gasUsed: string;
  baseFee: string;
  hash: Hash;
}

export interface BlockWithTransactions extends Block {
  transactions: Transaction[];
}

// ============================================================================
// Transaction Types
// ============================================================================

export interface TransactionRequest {
  to?: Address;
  from?: Address;
  nonce?: number;
  value?: BigNumberish;
  data?: HexString;
  gasLimit?: BigNumberish;
  gasPrice?: BigNumberish;
  maxFeePerGas?: BigNumberish;
  maxPriorityFeePerGas?: BigNumberish;
  chainId?: number;
  signature?: Signature;
}

export interface Transaction extends TransactionRequest {
  hash: Hash;
  blockNumber?: number;
  blockHash?: Hash;
  timestamp?: number;
  transactionIndex?: number;
  signature?: Signature;
  from: Address;
}

export interface RawTransaction {
  rlp: HexString;
  hash: Hash;
  from: Address;
}

export interface TransactionReceipt {
  transactionHash: Hash;
  blockNumber: number;
  blockHash: Hash;
  from: Address;
  to?: Address;
  status: number; // 1 = success, 0 = failure
  gasUsed: string;
  cumulativeGasUsed: string;
  logs: Log[];
  contractAddress?: Address;
  transactionIndex: number;
  effectiveGasPrice?: string;
}

export interface Log {
  address: Address;
  topics: string[];
  data: HexString;
  blockNumber: number;
  transactionHash: Hash;
  logIndex: number;
  removed?: boolean;
}

export interface Signature {
  r: string; // 32 bytes hex
  s: string; // 32 bytes hex
  v: number; // recovery ID
}

// ============================================================================
// Account Types
// ============================================================================

export interface Account {
  address: Address;
  balance: string;
  nonce: number;
  codeHash?: Hash;
  code?: HexString;
  storageRoot?: Hash;
}

export interface AccountInfo {
  nonce: number;
  balance: string;
  storageRoot: Hash;
  codeHash: Hash;
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
  userAgent?: string;
}

export interface NodeStatus {
  chainId: number;
  height: number;
  finalizedHeight: number;
  currentRound: number;
  currentStep: string;
  mempoolSize: number;
  peerCount: number;
  isValidator: boolean;
  version: string;
  uptime: number;
}

// ============================================================================
// Gas & Fee Types
// ============================================================================

export interface GasPriceResponse {
  baseFee: string;
  suggestedPriorityFeeLow: string;
  suggestedPriorityFeeMedium: string;
  suggestedPriorityFeeHigh: string;
  blockHeight: number;
}

export interface EstimateGasResponse {
  estimatedGas: number;
  baseFee: string;
  totalCostEstimate: string;
  estimatedPriorityFee: string;
}

export interface FeeHistoryResponse {
  baseFeePerGas: string[];
  gasUsedRatio: number[];
  oldestBlock: number;
  reward?: string[][];
}

// ============================================================================
// Mempool Types
// ============================================================================

export interface MempoolTransaction extends Transaction {
  addedAt: number;
  feePerGas: string;
  priority: number;
}

export interface MempoolResponse {
  size: number;
  transactions: MempoolTransaction[];
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

export interface SlashingEvent {
  validator: Address;
  reason: "DoubleSign" | "Downtime" | "InvalidStateTransition";
  amountSlashed: string;
  height: number;
}

// ============================================================================
// MEV Types
// ============================================================================

export interface BuilderBid {
  builderAddress: Address;
  blockNumber: number;
  bidAmount: string;
  mevValue: string;
  txCount: number;
  timestamp: number;
}

export interface MEVAuctionResult {
  blockNumber: number;
  winner: Address;
  winningBid: string;
  totalBids: number;
  mevExtracted: string;
}

// ============================================================================
// Rollup Types
// ============================================================================

export interface RollupBatch {
  batchId: number;
  transactions: Transaction[];
  prevStateRoot: Hash;
  newStateRoot: Hash;
  zkProof?: HexString;
  daCommitment?: HexString;
  timestamp: number;
  submitter: Address;
}

export interface CrossRollupMessage {
  fromRollup: string;
  toRollup: string;
  sender: Address;
  recipient: Address;
  data: HexString;
  value: string;
  nonce: number;
  proof?: HexString;
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

export interface LogEvent {
  log: Log;
  transactionHash: Hash;
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

export interface CallRequest extends TransactionRequest {
  blockOverride?: number | "latest" | "pending";
}

export interface FeeMarketInfo {
  baseFee: string;
  nextBaseFee: string;
  priorityFeeRange: {
    min: string;
    median: string;
    max: string;
  };
  suggestedGasPrice: string;
}

// ============================================================================
// Contract Types (for EVM compatibility)
// ============================================================================

export interface ContractCall {
  contract: Address;
  method: string;
  args: any[];
  value?: BigNumberish;
  gasLimit?: BigNumberish;
}

export interface ContractDeploy {
  bytecode: HexString;
  args: any[];
  value?: BigNumberish;
  gasLimit?: BigNumberish;
}

export interface ContractEvent {
  name: string;
  signature: string;
  topics: string[];
  data: HexString;
}

// ============================================================================
// Type Guards
// ============================================================================

export function isTransaction(x: any): x is Transaction {
  return x && typeof x === "object" && "hash" in x && "from" in x;
}

export function isBlock(x: any): x is Block {
  return x && typeof x === "object" && "number" in x && "hash" in x;
}

export function isReceipt(x: any): x is TransactionReceipt {
  return x && typeof x === "object" && "transactionHash" in x && "status" in x;
}

// ============================================================================
// Type Utilities
// ============================================================================

export type Optional<T, K extends keyof T> = Omit<T, K> & Partial<Pick<T, K>>;

export type WithRequired<T, K extends keyof T> = T & { [P in K]-?: T[P] };

export type Nullable<T> = T | null;

export type AsyncOrSync<T> = T | Promise<T>;

// ============================================================================
// Constants
// ============================================================================

export const EMPTY_HASH =
  "0x0000000000000000000000000000000000000000000000000000000000000000";
export const EMPTY_ADDRESS = "0x0000000000000000000000000000000000000000";
export const NATIVE_TOKEN_SYMBOL = "NBL";
export const NATIVE_TOKEN_DECIMALS = 18;

