import { Provider } from "./types/provider";
import {
  Block,
  Transaction,
  TransactionReceipt,
  TransactionRequest,
  ClientOptions,
  GasPriceResponse,
  EstimateGasResponse,
  MempoolResponse,
  PeerInfo,
  NodeStatus,
  Account,
} from "./types/client";
import {
  GovProposal,
  GovVote,
  TreasuryInfo,
  GovParams,
  DelegationInfo,
  ValidatorInfo,
  CreateProposalRequest,
  VoteRequest,
  GovStats,
} from "./types/governance";
import EventEmitter from "eventemitter3";

export class ModularClient extends EventEmitter {
  private provider: Provider;
  private options: ClientOptions;
  private connected: boolean = false;

  constructor(provider: Provider, options: ClientOptions = {}) {
    super();
    this.provider = provider;
    this.options = options;
  }

  async connect(): Promise<void> {
    const chainId = await this.getChainId();
    if (this.options.chainId && chainId !== this.options.chainId) {
      throw new Error(
        `Chain ID mismatch: expected ${this.options.chainId}, got ${chainId}`,
      );
    }
    this.connected = true;
    this.emit("connected", chainId);
  }

  isConnected(): boolean {
    return this.connected;
  }

  async disconnect(): Promise<void> {
    this.connected = false;
    this.emit("disconnected");
  }

  // Chain info
  async getChainId(): Promise<number> {
    const status = await this.provider.request("status");
    return status.chain_id ?? 1;
  }

  async getBlockNumber(): Promise<number> {
    const status = await this.provider.request("block_number");
    return status.height || 0;
  }

  async getBlock(blockNumber: number): Promise<Block> {
    return await this.provider.request("get_block", [blockNumber]);
  }

  async getBlockByHash(hash: string): Promise<Block> {
    const data = await this.provider.request("get_block_by_hash", [hash]);
    // Rust wraps blocks in { block: ..., error: ... }
    if (data.error) throw new Error(data.error);
    return data.block;
  }

  async getLatestBlock(): Promise<Block> {
    const data = await this.provider.request("get_latest_block");
    if (data.error) throw new Error(data.error);
    return data.block;
  }

  async getTxHistory(address: string, limit: number = 50): Promise<any[]> {
    const response = await this.provider.request("get_tx_history", [address, limit]);
    return response.transactions || [];
  }

  // Account
  async getBalance(address: string): Promise<string> {
    const response = await this.provider.request("get_balance", [address]);
    return response.balance || "0";
  }

  async getAccount(address: string): Promise<Account> {
    const response = await this.provider.request("get_account", [address]);
    return {
      address: address,
      balance: response.balance || "0",
      nonce: response.nonce || 0,
    };
  }

  async getNonce(address: string): Promise<number> {
    const account = await this.getAccount(address);
    return account.nonce;
  }

  // Transactions
  async sendTransaction(tx: Transaction): Promise<any> {
    // POST to /submit_tx with the full Transaction JSON
    const response = await this.provider.request("submit_tx", [tx]);
    if (response.status && response.status.startsWith("error")) {
      throw new Error(response.status);
    }
    return response;
  }

  async sendRawTransaction(signedTx: string): Promise<any> {
    // signedTx is a hex-encoded Transaction JSON string
    const tx: Transaction = JSON.parse(signedTx);
    return await this.sendTransaction(tx);
  }

  async getTransaction(hash: string): Promise<any> {
    try {
      const data = await this.provider.request("get_transaction", [hash]);
      // Rust wraps receipt in { receipt: ..., error: ... }
      if (data.error) return null;
      return data.receipt;
    } catch {
      return null;
    }
  }

  async getTransactionReceipt(
    hash: string,
  ): Promise<TransactionReceipt | null> {
    try {
      const data = await this.provider.request("get_transaction_receipt", [hash]);
      if (data.error || !data.receipt) return null;
      return data.receipt;
    } catch {
      return null;
    }
  }

  async waitForTransaction(
    hash: string,
    confirmations: number = 1,
    timeout: number = 60000,
  ): Promise<TransactionReceipt> {
    const startTime = Date.now();

    while (Date.now() - startTime < timeout) {
      const receipt = await this.getTransactionReceipt(hash);

      if (receipt) {
        const currentBlock = await this.getBlockNumber();
        const confirms = currentBlock - receipt.block_height + 1;

        if (confirms >= confirmations) {
          this.emit("transactionConfirmed", receipt);
          return receipt;
        }

        this.emit("transactionPending", {
          hash,
          confirmations: confirms,
          required: confirmations,
        });
      }

      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    throw new Error(`Transaction ${hash} not confirmed within ${timeout}ms`);
  }

  // Gas & Fees
  async getGasPrice(): Promise<GasPriceResponse> {
    return await this.provider.request("gas_price");
  }

  async estimateGas(tx: TransactionRequest): Promise<EstimateGasResponse> {
    return await this.provider.request("estimate_gas", [tx]);
  }

  async getFeeHistory(blockCount: number): Promise<any> {
    return await this.provider.request("fee_history", [blockCount]);
  }

  // Mempool
  async getMempool(_limit: number = 100): Promise<MempoolResponse> {
    const response = await this.provider.request("get_mempool");
    return response;
  }

  async getMempoolSize(): Promise<number> {
    const mempool = await this.getMempool(1);
    return mempool.size || 0;
  }

  // Network
  async getPeers(): Promise<PeerInfo[]> {
    const response = await this.provider.request("get_peers");
    return response.peers || [];
  }

  async connectPeer(multiaddr: string): Promise<void> {
    await this.provider.request("connect_peer", [{ multiaddr }]);
  }

  // Node Status
  async getNodeStatus(): Promise<NodeStatus> {
    return await this.provider.request("status");
  }

  async getMetrics(): Promise<string> {
    return await this.provider.request("get_metrics");
  }

  async healthCheck(): Promise<boolean> {
    try {
      const response = await this.provider.request("health");
      return response.status === "healthy";
    } catch {
      return false;
    }
  }

  // Governance (read from on-chain state)
  async getProposals(): Promise<GovProposal[]> {
    const response = await this.provider.request("get_governance");
    return response.proposals || [];
  }

  async getProposal(id: number): Promise<GovProposal | null> {
    try {
      const response = await this.provider.request("get_proposal", [id]);
      return response.proposal || null;
    } catch {
      return null;
    }
  }

  /** Governance writes use signed POST /submit_tx — use Wallet.signTransaction with governance payload. */
  async createProposal(_req: CreateProposalRequest): Promise<any> {
    throw new Error("Use Wallet.signTransaction with governance payload and client.sendTransaction()");
  }

  async vote(_req: VoteRequest): Promise<any> {
    throw new Error("Use Wallet.signTransaction with governance vote payload and client.sendTransaction()");
  }

  async getVotes(_proposalId: number): Promise<GovVote[]> {
    const proposal = await this.getProposal(_proposalId);
    if (!proposal || !proposal.voters) return [];
    return Object.entries(proposal.voters).map(([voter, choice]) => ({
      proposalId: _proposalId,
      voter,
      support: (String(choice) as GovVote["support"]) || "Abstain",
      weight: "0",
      timestamp: 0,
    }));
  }

  async getTreasury(): Promise<TreasuryInfo | null> {
    try {
      const response = await this.provider.request("get_governance");
      return response.treasury || null;
    } catch {
      return null;
    }
  }

  async getGovParams(): Promise<GovParams | null> {
    try {
      const response = await this.provider.request("get_governance");
      return response.params || null;
    } catch {
      return null;
    }
  }

  async getDelegations(address: string): Promise<DelegationInfo[]> {
    const response = await this.provider.request("get_delegations", [address]);
    return response.delegations || [];
  }

  async delegate(delegator: string, validator: string, amount: string): Promise<any> {
    return await this.provider.request("delegate_stake", [{ delegator, validator, amount }]);
  }

  async undelegate(_delegator: string, _validator: string, _amount: string): Promise<any> {
    throw new Error("Undelegate via signed governance/staking transaction (POST /submit_tx)");
  }

  async getValidators(): Promise<ValidatorInfo[]> {
    const response = await this.provider.request("get_validators");
    return response.validators || response || [];
  }

  async registerValidator(req: {
    address: string;
    public_key: string;
    stake: string;
    commission_rate?: number;
  }): Promise<any> {
    return await this.provider.request("register_validator", [req]);
  }

  async executeProposal(_proposalId: number): Promise<any> {
    throw new Error("Execute via signed governance transaction (POST /submit_tx)");
  }

  async requestFaucet(address: string, amount?: string): Promise<any> {
    return await this.provider.request("faucet_request", [{ address, amount }]);
  }

  async getSlashingEvents(): Promise<any[]> {
    const response = await this.provider.request("get_slashing_events");
    return response.events || [];
  }

  // WASM contracts
  async deployWasm(name: string, wasmBase64: string): Promise<any> {
    return await this.provider.request("deploy_wasm", [{ name, wasm: wasmBase64 }]);
  }

  async callWasm(name: string, func: string, arg: number = 0): Promise<any> {
    return await this.provider.request("call_wasm", [{ name, func, arg }]);
  }

  async listWasmContracts(): Promise<string[]> {
    const response = await this.provider.request("list_wasm_contracts");
    return response.contracts || [];
  }

  // Account abstraction & MEV
  async submitUserOperation(op: Record<string, unknown>): Promise<any> {
    return await this.provider.request("submit_user_operation", [op]);
  }

  async getPendingUserOperations(): Promise<number> {
    const response = await this.provider.request("pending_user_ops");
    return response.pending ?? response.count ?? 0;
  }

  async getGovStats(): Promise<GovStats | null> {
    try {
      const [proposals, validators, params] = await Promise.all([
        this.getProposals(),
        this.getValidators(),
        this.getGovParams(),
      ]);
      const activeProposals = proposals.filter((p) => p.status === "Active");
      const activeValidators = validators.filter((v) => v.isActive);
      const totalStaked = validators.reduce((sum, v) => sum + BigInt(v.stake || "0"), 0n).toString();
      const totalVotes = proposals.reduce((sum, p) => sum + BigInt(p.yesVotes || "0") + BigInt(p.noVotes || "0"), 0n).toString();
      return {
        totalProposals: proposals.length,
        activeProposals: activeProposals.length,
        totalVotes: Number(totalVotes),
        totalDelegators: 0,
        totalStaked,
        votingPower: "0",
        inflationRate: params ? 100 / Number(params.votingPeriod) * 365 * 24 * 60 * 60 / 6 : 0,
        activeValidators: activeValidators.length,
      };
    } catch {
      return null;
    }
  }

  // Utilities
  getProvider(): Provider {
    return this.provider;
  }
}
