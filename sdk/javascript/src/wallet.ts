import { secp256k1 } from "@noble/curves/secp256k1";
import { sha256 } from "@noble/hashes/sha256";
import { keccak_256 } from "@noble/hashes/sha3";
import { bytesToHex, hexToBytes } from "@noble/hashes/utils";
import { generateMnemonic, mnemonicToSeedSync } from "bip39";
import { Provider } from "./types/provider";
import {
  TransactionRequest,
  Transaction,
  TransactionReceipt,
  Signature,
} from "./types/client";
import { ModularClient } from "./client";

export class Wallet {
  private privateKey: Uint8Array;
  public readonly address: string;
  public readonly publicKey: string;

  constructor(privateKey: string | Uint8Array) {
    if (typeof privateKey === "string") {
      const cleanKey = privateKey.startsWith("0x")
        ? privateKey.slice(2)
        : privateKey;
      this.privateKey = hexToBytes(cleanKey);
    } else {
      this.privateKey = privateKey;
    }

    // Derive public key (uncompressed format)
    const pubKey = secp256k1.getPublicKey(this.privateKey, false);
    this.publicKey = bytesToHex(pubKey);

    // Derive address (last 20 bytes of keccak256 of public key without prefix)
    const pubKeyWithoutPrefix = pubKey.slice(1);
    const hash = keccak_256(pubKeyWithoutPrefix);
    this.address = "0x" + bytesToHex(hash.slice(-20));
  }

  static generate(): Wallet {
    const privateKey = secp256k1.utils.randomPrivateKey();
    return new Wallet(privateKey);
  }

  static fromPrivateKey(privateKey: string): Wallet {
    return new Wallet(privateKey);
  }

  static fromMnemonic(mnemonic: string, index: number = 0): Wallet {
    const seed = mnemonicToSeedSync(mnemonic);
    // Use first 32 bytes as private key (simplified - in production use BIP32)
    const privateKey = seed.slice(0, 32);
    return new Wallet(privateKey);
  }

  static generateMnemonic(): string {
    return generateMnemonic();
  }

  async signMessage(message: string): Promise<string> {
    const messageHash = keccak_256(new TextEncoder().encode(message));
    const signature = await secp256k1.sign(messageHash, this.privateKey);
    return bytesToHex(signature);
  }

  async signTransaction(tx: TransactionRequest): Promise<Transaction> {
    const txData = this.serializeTransaction(tx);
    const txHash = keccak_256(txData);
    const signature = await secp256k1.sign(txHash, this.privateKey, {
      lowS: true,
    });

    // Recover recovery ID
    const recId = signature.recovery;

    const sig: Signature = {
      r: bytesToHex(signature.r),
      s: bytesToHex(signature.s),
      v: recId !== undefined ? recId + 27 : 0,
    };

    return {
      ...tx,
      from: this.address,
      hash: "0x" + bytesToHex(txHash),
      signature: sig,
    };
  }

  async signRawTransaction(rawTx: Uint8Array): Promise<string> {
    const txHash = keccak_256(rawTx);
    const signature = await secp256k1.sign(txHash, this.privateKey);
    const signatureBytes = new Uint8Array([
      ...signature.r,
      ...signature.s,
      signature.recovery !== undefined ? signature.recovery : 0,
    ]);
    return bytesToHex(signatureBytes);
  }

  connect(provider: Provider): ConnectedWallet {
    return new ConnectedWallet(this.privateKey, provider);
  }

  getPrivateKey(): string {
    return bytesToHex(this.privateKey);
  }

  getPublicKey(): string {
    return this.publicKey;
  }

  getAddress(): string {
    return this.address;
  }

  private serializeTransaction(tx: TransactionRequest): Uint8Array {
    // EIP-1559 transaction serialization
    const data = {
      chainId: tx.chainId || 1,
      nonce: tx.nonce || 0,
      gasLimit: tx.gasLimit || "21000",
      maxFeePerGas: tx.maxFeePerGas || "1000000000",
      maxPriorityFeePerGas: tx.maxPriorityFeePerGas || "100000000",
      to: tx.to || "0x",
      value: tx.value || "0",
      data: tx.data || "0x",
    };

    return new TextEncoder().encode(JSON.stringify(data));
  }

  static verifySignature(
    message: string,
    signature: string,
    address: string,
  ): boolean {
    try {
      const messageHash = keccak_256(new TextEncoder().encode(message));
      const sigBytes = hexToBytes(signature);
      const recovered = secp256k1.recoverPublicKey(messageHash, sigBytes);
      const recoveredAddress = this.publicKeyToAddress(bytesToHex(recovered));
      return recoveredAddress.toLowerCase() === address.toLowerCase();
    } catch {
      return false;
    }
  }

  private static publicKeyToAddress(publicKey: string): string {
    const pubKeyBytes = hexToBytes(publicKey);
    const pubKeyWithoutPrefix = pubKeyBytes.slice(1);
    const hash = keccak_256(pubKeyWithoutPrefix);
    return "0x" + bytesToHex(hash.slice(-20));
  }
}

export class ConnectedWallet extends Wallet {
  private client: ModularClient;

  constructor(privateKey: string | Uint8Array, provider: Provider) {
    super(privateKey);
    this.client = new ModularClient(provider);
  }

  async sendTransaction(tx: TransactionRequest): Promise<TransactionReceipt> {
    if (!tx.nonce) {
      tx.nonce = await this.getNonce();
    }
    tx.from = this.address;

    const signedTx = await this.signTransaction(tx);
    return await this.client.sendTransaction(signedTx);
  }

  async sendRawTransaction(rawTx: Uint8Array): Promise<TransactionReceipt> {
    const signature = await this.signRawTransaction(rawTx);
    return await this.client.sendRawTransaction(signature);
  }

  async getBalance(): Promise<string> {
    return await this.client.getBalance(this.address);
  }

  async getNonce(): Promise<number> {
    return await this.client.getNonce(this.address);
  }

  async getTransactionCount(): Promise<number> {
    return await this.getNonce();
  }

  getClient(): ModularClient {
    return this.client;
  }
}
