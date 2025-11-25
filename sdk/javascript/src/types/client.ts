export interface Block {
  number: number;
  hash: string;
  parentHash: string;
  timestamp: number;
  transactions: string[];
  stateRoot: string;
  validator: string;
}

export interface Account {
  address: string;
  balance: string;
  nonce: number;
}

export interface TransactionRequest {
  to?: string;
  from?: string;
  nonce?: number;
  value?: string;
  data?: string;
  gasLimit?: string;
  gasPrice?: string;
}

export interface Transaction extends TransactionRequest {
  hash: string;
  blockNumber?: number;
  blockHash?: string;
  timestamp?: number;
  signature?: Signature;
}

export interface TransactionReceipt {
  transactionHash: string;
  blockNumber: number;
  blockHash: string;
  from: string;
  to?: string;
  status: number;
  gasUsed: string;
  logs: Log[];
}

export interface Log {
  address: string;
  topics: string[];
  data: string;
  blockNumber: number;
  transactionHash: string;
  logIndex: number;
}

export interface Signature {
  r: string;
  s: string;
  v: number;
}

export interface ClientOptions {
  chainId?: number;
  timeout?: number;
}

export type BigNumberish = string | number | bigint;
