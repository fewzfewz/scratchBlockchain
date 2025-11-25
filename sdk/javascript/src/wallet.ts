import * as secp256k1 from '@noble/secp256k1';
import { sha256 } from '@noble/hashes/sha256';
import { keccak_256 } from '@noble/hashes/sha3';
import { Provider } from './types/provider';
import { TransactionRequest, Transaction, TransactionReceipt, Signature } from './types/client';
import { ModularClient } from './client';

export class Wallet {
  private privateKey: Uint8Array;
  public readonly address: string;
  public readonly publicKey: string;

  constructor(privateKey: string | Uint8Array) {
    if (typeof privateKey === 'string') {
      this.privateKey = hexToBytes(privateKey);
    } else {
      this.privateKey = privateKey;
    }

    // Derive public key
    const pubKey = secp256k1.getPublicKey(this.privateKey, false);
    this.publicKey = bytesToHex(pubKey);

    // Derive address (last 20 bytes of keccak256(publicKey))
    const hash = keccak_256(pubKey.slice(1)); // Remove 0x04 prefix
    this.address = '0x' + bytesToHex(hash.slice(-20));
  }

  // Static factory methods
  static generate(): Wallet {
    const privateKey = secp256k1.utils.randomPrivateKey();
    return new Wallet(privateKey);
  }

  static fromPrivateKey(privateKey: string): Wallet {
    return new Wallet(privateKey);
  }

  static fromMnemonic(mnemonic: string): Wallet {
    // Simplified: In production, use BIP39
    const hash = sha256(new TextEncoder().encode(mnemonic));
    return new Wallet(hash);
  }

  // Signing
  async signMessage(message: string): Promise<string> {
    const messageHash = keccak_256(new TextEncoder().encode(message));
    const signature = await secp256k1.sign(messageHash, this.privateKey);
    return bytesToHex(signature);
  }

  async signTransaction(tx: TransactionRequest): Promise<Transaction> {
    // Serialize transaction
    const txData = this.serializeTransaction(tx);
    const txHash = keccak_256(txData);

    // Sign
    const signature = await secp256k1.sign(txHash, this.privateKey, {
      recovered: true,
    });

    const sig: Signature = {
      r: bytesToHex(signature.slice(0, 32)),
      s: bytesToHex(signature.slice(32, 64)),
      v: signature[64],
    };

    return {
      ...tx,
      from: this.address,
      hash: '0x' + bytesToHex(txHash),
      signature: sig,
    };
  }

  // Connection
  connect(provider: Provider): ConnectedWallet {
    return new ConnectedWallet(this.privateKey, provider);
  }

  // Utilities
  getPrivateKey(): string {
    return bytesToHex(this.privateKey);
  }

  private serializeTransaction(tx: TransactionRequest): Uint8Array {
    // Simplified serialization
    const data = JSON.stringify({
      to: tx.to,
      value: tx.value,
      data: tx.data,
      nonce: tx.nonce,
      gasLimit: tx.gasLimit,
      gasPrice: tx.gasPrice,
    });
    return new TextEncoder().encode(data);
  }
}

export class ConnectedWallet extends Wallet {
  private client: ModularClient;

  constructor(privateKey: string | Uint8Array, provider: Provider) {
    super(privateKey);
    this.client = new ModularClient(provider);
  }

  async sendTransaction(tx: TransactionRequest): Promise<TransactionReceipt> {
    // Auto-fill nonce if not provided
    if (tx.nonce === undefined) {
      tx.nonce = await this.getNonce();
    }

    // Auto-fill from
    tx.from = this.address;

    // Sign transaction
    const signedTx = await this.signTransaction(tx);

    // Send to network
    return await this.client.sendTransaction(signedTx);
  }

  async getBalance(): Promise<string> {
    return await this.client.getBalance(this.address);
  }

  async getNonce(): Promise<number> {
    return await this.client.getNonce(this.address);
  }

  getClient(): ModularClient {
    return this.client;
  }
}

// Utility functions
function hexToBytes(hex: string): Uint8Array {
  if (hex.startsWith('0x')) {
    hex = hex.slice(2);
  }
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}
