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
    // /status doesn't return chain_id; default to 1 as placeholder
    return 1;
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
    const blockNumber = await this.getBlockNumber();
    return await this.getBlock(blockNumber);
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

  // Governance
  async getProposals(): Promise<GovProposal[]> {
    const response = await this.provider.request("get_proposals");
    return response.proposals || [];
  }

  async getProposal(id: number): Promise<GovProposal | null> {
    try {
      const response = await this.provider.request("get_proposal", [id]);
      return response.proposal || response;
    } catch {
      return null;
    }
  }

  async createProposal(req: CreateProposalRequest): Promise<any> {
    return await this.provider.request("create_proposal", [req]);
  }

  async vote(req: VoteRequest): Promise<any> {
    return await this.provider.request("cast_vote", [req]);
  }

  async getVotes(proposalId: number): Promise<GovVote[]> {
    const response = await this.provider.request("get_votes", [proposalId]);
    return response.votes || [];
  }

  async getTreasury(): Promise<TreasuryInfo | null> {
    try {
      return await this.provider.request("get_treasury");
    } catch {
      return null;
    }
  }

  async getGovParams(): Promise<GovParams | null> {
    try {
      return await this.provider.request("get_gov_params");
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

  async undelegate(delegator: string, validator: string, amount: string): Promise<any> {
    return await this.provider.request("undelegate_stake", [{ delegator, validator, amount }]);
  }

  async getValidators(): Promise<ValidatorInfo[]> {
    const response = await this.provider.request("get_validators");
    return response.validators || [];
  }

  async executeProposal(proposalId: number): Promise<any> {
    return await this.provider.request("execute_proposal", [proposalId]);
  }

  async getGovStats(): Promise<GovStats | null> {
    try {
      const [proposals, treasury, validators, params] = await Promise.all([
        this.getProposals(),
        this.getTreasury(),
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
