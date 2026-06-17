import EventEmitter from 'eventemitter3';

interface Provider {
    request(method: string, params?: any[]): Promise<any>;
    on?(event: string, callback: (...args: any[]) => void): void;
    off?(event: string, callback: (...args: any[]) => void): void;
}
interface ProviderOptions {
    timeout?: number;
    headers?: Record<string, string>;
}

type BigNumberish = string | number | bigint;
type Address = string;
type Hash = string;
type HexString = string;
interface TransactionRequest {
    to?: Address;
    value?: BigNumberish;
    data?: HexString;
    gasLimit?: BigNumberish;
    maxFeePerGas?: BigNumberish;
    maxPriorityFeePerGas?: BigNumberish;
    chainId?: number;
    nonce?: number;
}
interface Transaction {
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
interface TransactionReceipt {
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
interface BlockHeader {
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
interface Block {
    header: BlockHeader;
    transactions: Transaction[];
    hash: Hash;
}
interface Account {
    address: Address;
    balance: string;
    nonce: number;
}
interface PeerInfo {
    id: string;
    address: string;
    direction: "inbound" | "outbound";
    protocolVersion: string;
    bestHeight: number;
    latency?: number;
}
interface NodeStatus {
    height: number;
    finalized_height: number | null;
    mempool_size: number;
    peer_count: number;
}
interface GasPriceResponse {
    base_fee: string;
    suggested_priority_fee_low: string;
    suggested_priority_fee_medium: string;
    suggested_priority_fee_high: string;
    block_height: number;
}
interface EstimateGasResponse {
    estimated_gas: number;
    base_fee: string;
    total_cost_estimate: string;
    estimated_priority_fee: string;
}
interface FeeHistoryResponse {
    base_fee_per_gas: string[];
    gas_used_ratio: number[];
    oldest_block: number;
}
interface MempoolResponse {
    size: number;
    transactions: Transaction[];
}
interface Proposal {
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
interface Vote {
    proposalId: number;
    voter: Address;
    vote: boolean;
    votingPower: string;
    timestamp: number;
}
interface Validator {
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
interface Delegation {
    delegator: Address;
    validator: Address;
    amount: string;
    rewards: string;
    createdHeight: number;
}
interface NewBlockEvent {
    block: Block;
    height: number;
    timestamp: number;
}
interface NewTransactionEvent {
    transaction: Transaction;
    addedAt: number;
}
interface FinalizedBlockEvent {
    blockHash: Hash;
    blockNumber: number;
}
interface ClientOptions {
    chainId?: number;
    timeout?: number;
    headers?: Record<string, string>;
    maxRetries?: number;
    retryDelay?: number;
}
interface RPCResponse<T = any> {
    jsonrpc: "2.0";
    id: number;
    result?: T;
    error?: RPCError;
}
interface RPCError {
    code: number;
    message: string;
    data?: any;
}
interface Subscription {
    id: string;
    type: "newBlocks" | "newTransactions" | "logs";
    filter?: LogFilter;
}
interface LogFilter {
    addresses?: Address[];
    topics?: (string[] | null)[];
    fromBlock?: number | "earliest" | "latest" | "pending";
    toBlock?: number | "earliest" | "latest" | "pending";
}
type Bytes = Uint8Array;
declare const EMPTY_HASH = "0x0000000000000000000000000000000000000000000000000000000000000000";
declare const EMPTY_ADDRESS = "0x0000000000000000000000000000000000000000";
declare const NATIVE_TOKEN_SYMBOL = "NBL";
declare const NATIVE_TOKEN_DECIMALS = 18;

declare class ModularClient extends EventEmitter {
    private provider;
    private options;
    private connected;
    constructor(provider: Provider, options?: ClientOptions);
    connect(): Promise<void>;
    isConnected(): boolean;
    disconnect(): Promise<void>;
    getChainId(): Promise<number>;
    getBlockNumber(): Promise<number>;
    getBlock(blockNumber: number): Promise<Block>;
    getBlockByHash(hash: string): Promise<Block>;
    getLatestBlock(): Promise<Block>;
    getBalance(address: string): Promise<string>;
    getAccount(address: string): Promise<Account>;
    getNonce(address: string): Promise<number>;
    sendTransaction(tx: Transaction): Promise<any>;
    sendRawTransaction(signedTx: string): Promise<any>;
    getTransaction(hash: string): Promise<any>;
    getTransactionReceipt(hash: string): Promise<TransactionReceipt | null>;
    waitForTransaction(hash: string, confirmations?: number, timeout?: number): Promise<TransactionReceipt>;
    getGasPrice(): Promise<GasPriceResponse>;
    estimateGas(tx: TransactionRequest): Promise<EstimateGasResponse>;
    getFeeHistory(blockCount: number): Promise<any>;
    getMempool(_limit?: number): Promise<MempoolResponse>;
    getMempoolSize(): Promise<number>;
    getPeers(): Promise<PeerInfo[]>;
    connectPeer(multiaddr: string): Promise<void>;
    getNodeStatus(): Promise<NodeStatus>;
    getMetrics(): Promise<string>;
    healthCheck(): Promise<boolean>;
    getProvider(): Provider;
}

declare class Wallet {
    private privateKey;
    readonly address: string;
    readonly publicKey: string;
    constructor(privateKey: string | Uint8Array);
    static generate(): Wallet;
    static fromPrivateKey(privateKey: string): Wallet;
    static fromMnemonic(mnemonic: string, _index?: number): Wallet;
    static generateMnemonic(): string;
    signMessage(message: string): Promise<string>;
    signTransaction(tx: TransactionRequest): Promise<Transaction>;
    connect(provider: Provider): ConnectedWallet;
    getPrivateKey(): string;
    getPublicKey(): string;
    getAddress(): string;
    static verifySignature(message: string, signature: string, publicKey: string): boolean;
}
declare class ConnectedWallet {
    private wallet;
    private client;
    constructor(privateKey: string | Uint8Array, provider: Provider);
    sendTransaction(tx: TransactionRequest): Promise<any>;
    getBalance(): Promise<string>;
    get address(): string;
    getNonce(): Promise<number>;
    getClient(): ModularClient;
}

declare class HttpProvider implements Provider {
    private client;
    private url;
    private isConnected;
    constructor(url: string, options?: ProviderOptions);
    request(method: string, params?: any[]): Promise<any>;
    private mapMethodToEndpoint;
    private buildGetUrl;
    getUrl(): string;
    isConnectedToNode(): boolean;
}

declare class WebSocketProvider extends EventEmitter implements Provider {
    private ws;
    private url;
    private options;
    private connected;
    private requestId;
    private pending;
    private subscriptions;
    private reconnectAttempts;
    private maxReconnectAttempts;
    private reconnectDelay;
    private disconnectRequested;
    private pollTimers;
    constructor(url: string, options?: ProviderOptions);
    request(method: string, params?: any[]): Promise<any>;
    connect(): Promise<void>;
    disconnect(): Promise<void>;
    subscribe(event: string, filter?: any): Promise<string>;
    unsubscribe(subscriptionId: string): Promise<boolean>;
    on(event: string | symbol, fn: (...args: any[]) => void, context?: any): this;
    off(event: string | symbol, fn?: ((...args: any[]) => void) | undefined): this;
    isConnected(): boolean;
    getUrl(): string;
    private handleMessage;
    private startPolling;
    private attemptReconnect;
    private rejectAllPending;
}

export { Account, Address, BigNumberish, Block, BlockHeader, Bytes, ClientOptions, ConnectedWallet, Delegation, EMPTY_ADDRESS, EMPTY_HASH, EstimateGasResponse, FeeHistoryResponse, FinalizedBlockEvent, GasPriceResponse, Hash, HexString, HttpProvider, LogFilter, MempoolResponse, ModularClient, NATIVE_TOKEN_DECIMALS, NATIVE_TOKEN_SYMBOL, NewBlockEvent, NewTransactionEvent, NodeStatus, PeerInfo, Proposal, Provider, ProviderOptions, RPCError, RPCResponse, Subscription, Transaction, TransactionReceipt, TransactionRequest, Validator, Vote, Wallet, WebSocketProvider };
