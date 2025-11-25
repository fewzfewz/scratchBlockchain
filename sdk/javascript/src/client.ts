import { Provider } from '../types/provider';
import {
  Block,
  Account,
  Transaction,
  TransactionReceipt,
  TransactionRequest,
  ClientOptions,
} from '../types/client';

export class ModularClient {
  private provider: Provider;
  private options: ClientOptions;
  private eventListeners: Map<string, Set<Function>>;

  constructor(provider: Provider, options: ClientOptions = {}) {
    this.provider = provider;
    this.options = options;
    this.eventListeners = new Map();
  }

  // Connection
  async connect(): Promise<void> {
    const chainId = await this.getChainId();
    if (this.options.chainId && chainId !== this.options.chainId) {
      throw new Error(`Chain ID mismatch: expected ${this.options.chainId}, got ${chainId}`);
    }
  }

  isConnected(): boolean {
    return this.provider !== null;
  }

  // Chain info
  async getChainId(): Promise<number> {
    return await this.provider.request('chain_id');
  }

  async getBlockNumber(): Promise<number> {
    return await this.provider.request('block_number');
  }

  async getBlock(blockNumber: number): Promise<Block> {
    return await this.provider.request('get_block', [blockNumber]);
  }

  async getLatestBlock(): Promise<Block> {
    const blockNumber = await this.getBlockNumber();
    return await this.getBlock(blockNumber);
  }

  // Account
  async getBalance(address: string): Promise<string> {
    return await this.provider.request('get_balance', [address]);
  }

  async getAccount(address: string): Promise<Account> {
    return await this.provider.request('get_account', [address]);
  }

  async getNonce(address: string): Promise<number> {
    const account = await this.getAccount(address);
    return account.nonce;
  }

  // Transactions
  async sendTransaction(tx: TransactionRequest): Promise<TransactionReceipt> {
    const txHash = await this.provider.request('send_transaction', [tx]);
    
    // Wait for transaction to be mined
    return await this.waitForTransaction(txHash);
  }

  async getTransaction(hash: string): Promise<Transaction> {
    return await this.provider.request('get_transaction', [hash]);
  }

  async getTransactionReceipt(hash: string): Promise<TransactionReceipt | null> {
    return await this.provider.request('get_transaction_receipt', [hash]);
  }

  async waitForTransaction(
    hash: string,
    confirmations: number = 1,
    timeout: number = 60000
  ): Promise<TransactionReceipt> {
    const startTime = Date.now();

    while (Date.now() - startTime < timeout) {
      const receipt = await this.getTransactionReceipt(hash);
      
      if (receipt) {
        const currentBlock = await this.getBlockNumber();
        const confirms = currentBlock - receipt.blockNumber + 1;
        
        if (confirms >= confirmations) {
          return receipt;
        }
      }

      // Wait 1 second before checking again
      await new Promise(resolve => setTimeout(resolve, 1000));
    }

    throw new Error(`Transaction ${hash} not mined within ${timeout}ms`);
  }

  // Events
  on(event: string, callback: Function): void {
    if (!this.eventListeners.has(event)) {
      this.eventListeners.set(event, new Set());
    }
    this.eventListeners.get(event)!.add(callback);

    if (this.provider.on) {
      this.provider.on(event, callback as any);
    }
  }

  off(event: string, callback: Function): void {
    const listeners = this.eventListeners.get(event);
    if (listeners) {
      listeners.delete(callback);
    }

    if (this.provider.off) {
      this.provider.off(event, callback as any);
    }
  }

  // Utilities
  getProvider(): Provider {
    return this.provider;
  }
}
