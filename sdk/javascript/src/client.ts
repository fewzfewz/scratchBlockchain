import { Provider } from "./types/provider";
import {
  Block,
  Account,
  Transaction,
  TransactionReceipt,
  TransactionRequest,
  ClientOptions,
  GasPriceResponse,
  EstimateGasResponse,
  MempoolResponse,
  PeerInfo,
} from "./types/client";
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

  async disconnect(): void {
    this.connected = false;
    this.emit("disconnected");
  }

  // Chain info
  async getChainId(): Promise<number> {
    const status = await this.provider.request("chain_id");
    // Extract chain ID from status response
    return status.chain_id || 1;
  }

  async getBlockNumber(): Promise<number> {
    const status = await this.provider.request("block_number");
    return status.height || 0;
  }

  async getBlock(blockNumber: number): Promise<Block> {
    const response = await this.provider.request("get_block", [blockNumber]);
    return this.formatBlock(response);
  }

  async getBlockByHash(hash: string): Promise<Block> {
    const response = await this.provider.request("get_block_by_hash", [hash]);
    return this.formatBlock(response);
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
  async sendTransaction(tx: TransactionRequest): Promise<TransactionReceipt> {
    const response = await this.provider.request("send_transaction", [tx]);
    const txHash = response.hash || response;

    // Wait for transaction to be mined
    const receipt = await this.waitForTransaction(txHash);
    this.emit("transactionSent", receipt);
    return receipt;
  }

  async sendRawTransaction(signedTx: string): Promise<TransactionReceipt> {
    const response = await this.provider.request("send_raw_transaction", [
      signedTx,
    ]);
    const txHash = response.hash || response;
    return await this.waitForTransaction(txHash);
  }

  async getTransaction(hash: string): Promise<Transaction | null> {
    try {
      const response = await this.provider.request("get_transaction", [hash]);
      if (!response) return null;
      return this.formatTransaction(response, hash);
    } catch (error) {
      return null;
    }
  }

  async getTransactionReceipt(
    hash: string,
  ): Promise<TransactionReceipt | null> {
    try {
      const response = await this.provider.request("get_transaction_receipt", [
        hash,
      ]);
      if (!response || Object.keys(response).length === 0) return null;
      return this.formatReceipt(response);
    } catch (error) {
      return null;
    }
  }

  async waitForTransaction(
    hash: string,
    confirmations: number = 1,
    timeout: number = 60000,
  ): Promise<TransactionReceipt> {
    const startTime = Date.now();
    let lastReceipt: TransactionReceipt | null = null;

    while (Date.now() - startTime < timeout) {
      const receipt = await this.getTransactionReceipt(hash);

      if (receipt) {
        lastReceipt = receipt;
        const currentBlock = await this.getBlockNumber();
        const confirms = currentBlock - receipt.blockNumber + 1;

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

      // Exponential backoff
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    throw new Error(`Transaction ${hash} not confirmed within ${timeout}ms`);
  }

  // Gas & Fees
  async getGasPrice(): Promise<GasPriceResponse> {
    return await this.provider.request("gas_price", []);
  }

  async estimateGas(tx: TransactionRequest): Promise<EstimateGasResponse> {
    return await this.provider.request("estimate_gas", [tx]);
  }

  async getFeeHistory(blockCount: number): Promise<any> {
    return await this.provider.request("fee_history", [blockCount]);
  }

  // Mempool
  async getMempool(limit: number = 100): Promise<MempoolResponse> {
    const response = await this.provider.request("get_mempool", [limit]);
    return response;
  }

  async getMempoolSize(): Promise<number> {
    const mempool = await this.getMempool(1);
    return mempool.size || 0;
  }

  // Network
  async getPeers(): Promise<PeerInfo[]> {
    return await this.provider.request("get_peers", []);
  }

  async connectPeer(multiaddr: string): Promise<void> {
    await this.provider.request("connect_peer", [{ multiaddr }]);
  }

  // Node Status
  async getNodeStatus(): Promise<any> {
    return await this.provider.request("status", []);
  }

  async getMetrics(): Promise<string> {
    return await this.provider.request("get_metrics", []);
  }

  async healthCheck(): Promise<boolean> {
    try {
      await this.provider.request("health", []);
      return true;
    } catch {
      return false;
    }
  }

  // Utilities
  getProvider(): Provider {
    return this.provider;
  }

  // Private formatting methods
  private formatBlock(data: any): Block {
    return {
      number: data.number || data.height || 0,
      hash: data.hash || "0x",
      parentHash: data.parent_hash || "0x",
      timestamp: data.timestamp || 0,
      transactions: data.transactions || [],
      stateRoot: data.state_root || "0x",
      validator: data.validator || "0x",
      gasUsed: data.gas_used || "0",
      gasLimit: data.gas_limit || "30000000",
    };
  }

  private formatTransaction(data: any, hash: string): Transaction {
    return {
      hash: hash,
      from: data.from || "0x",
      to: data.to,
      nonce: data.nonce || 0,
      value: data.value || "0",
      gasLimit: data.gas_limit || "21000",
      gasPrice: data.gas_price,
      maxFeePerGas: data.max_fee_per_gas,
      maxPriorityFeePerGas: data.max_priority_fee_per_gas,
      data: data.data || "0x",
      blockNumber: data.block_number,
      blockHash: data.block_hash,
      timestamp: data.timestamp,
      signature: data.signature,
    };
  }

  private formatReceipt(data: any): TransactionReceipt {
    return {
      transactionHash: data.transaction_hash || data.tx_hash,
      blockNumber: data.block_number,
      blockHash: data.block_hash,
      from: data.from,
      to: data.to,
      status: data.status === "Success" || data.status === "success" ? 1 : 0,
      gasUsed: data.gas_used || "0",
      cumulativeGasUsed: data.cumulative_gas_used || "0",
      logs: data.logs || [],
      contractAddress: data.contract_address,
    };
  }
}
