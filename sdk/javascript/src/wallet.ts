import { ed25519 } from "@noble/curves/ed25519";
import { sha256 } from "@noble/hashes/sha256";
import { bytesToHex, hexToBytes } from "@noble/hashes/utils";
import { generateMnemonic, mnemonicToSeedSync } from "bip39";
import { Provider } from "./types/provider";
import { TransactionRequest, Transaction } from "./types/client";
import { ModularClient } from "./client";

function uint64LE(value: number): Uint8Array {
  const buf = new ArrayBuffer(8);
  const view = new DataView(buf);
  view.setBigUint64(0, BigInt(value), true);
  return new Uint8Array(buf);
}

export class Wallet {
  private privateKey: Uint8Array;
  public readonly address: string;
  public readonly publicKey: string;

  constructor(privateKey: string | Uint8Array) {
    if (typeof privateKey === "string") {
      const cleanKey = privateKey.startsWith("0x") ? privateKey.slice(2) : privateKey;
      this.privateKey = hexToBytes(cleanKey);
    } else {
      this.privateKey = privateKey;
    }

    const pubKey = ed25519.getPublicKey(this.privateKey);
    this.publicKey = bytesToHex(pubKey);
    // Address = last 20 bytes of SHA-256(public_key), hex with 0x prefix
    const hash = sha256(pubKey);
    this.address = "0x" + bytesToHex(hash.slice(-20));
  }

  static generate(): Wallet {
    const privateKey = ed25519.utils.randomPrivateKey();
    return new Wallet(privateKey);
  }

  static fromPrivateKey(privateKey: string): Wallet {
    return new Wallet(privateKey);
  }

  static fromMnemonic(mnemonic: string, _index: number = 0): Wallet {
    const seed = mnemonicToSeedSync(mnemonic);
    // Use first 32 bytes as private key (simplified - in production use BIP32)
    const privateKey = seed.slice(0, 32);
    return new Wallet(privateKey);
  }

  static generateMnemonic(): string {
    return generateMnemonic();
  }

  async signMessage(message: string): Promise<string> {
    const msgBytes = new TextEncoder().encode(message);
    const h = sha256(msgBytes);
    const signature = ed25519.sign(h, this.privateKey);
    return bytesToHex(signature);
  }

  async signTransaction(tx: TransactionRequest): Promise<Transaction> {
    const sender = this.address;
    const nonce = tx.nonce ?? 0;
    const rawPayload = tx.data
      ? hexToBytes(tx.data.startsWith("0x") ? tx.data.slice(2) : tx.data)
      : new Uint8Array();
    const gasLimit = Number(tx.gasLimit ?? 21000);
    const maxFeePerGas = Number(tx.maxFeePerGas ?? 1000000000);
    const maxPriorityFeePerGas = Number(tx.maxPriorityFeePerGas ?? 100000000);
    const chainId = tx.chainId ?? null;
    const to = tx.to ?? null;
    const value = Number(tx.value ?? 0);

    // Serialize matching Rust Transaction::hash() exactly:
    // Sha256(sender(20) + nonce(8LE) + payload(var) + gas_limit(8LE)
    //   + max_fee(8LE) + max_priority_fee(8LE)
    //   + [chain_id(8LE)] + [to(20)] + value(8LE))
    const parts: Uint8Array[] = [
      hexToBytes(sender.startsWith("0x") ? sender.slice(2) : sender),
      uint64LE(nonce),
      rawPayload,
      uint64LE(gasLimit),
      uint64LE(maxFeePerGas),
      uint64LE(maxPriorityFeePerGas),
    ];
    if (chainId !== null) parts.push(uint64LE(chainId));
    if (to !== null) {
      parts.push(hexToBytes(to.startsWith("0x") ? to.slice(2) : to));
    }
    parts.push(uint64LE(value));

    const total = parts.reduce((sum, p) => sum + p.length, 0);
    const message = new Uint8Array(total);
    let offset = 0;
    for (const p of parts) { message.set(p, offset); offset += p.length; }

    const hash = sha256(message);
    const signatureBytes = ed25519.sign(hash, this.privateKey);

    return {
      sender,
      nonce,
      payload: "0x" + bytesToHex(rawPayload),
      signature: "0x" + bytesToHex(signatureBytes),
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas,
      chainId,
      to: to ?? null,
      value,
      hash: "0x" + bytesToHex(hash),
    };
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

  static verifySignature(message: string, signature: string, publicKey: string): boolean {
    try {
      const msgBytes = new TextEncoder().encode(message);
      const h = sha256(msgBytes);
      const sigBytes = hexToBytes(signature);
      const pkBytes = hexToBytes(publicKey);
      return ed25519.verify(sigBytes, h, pkBytes);
    } catch {
      return false;
    }
  }
}

export class ConnectedWallet {
  private wallet: Wallet;
  private client: ModularClient;

  constructor(privateKey: string | Uint8Array, provider: Provider) {
    this.wallet = new Wallet(privateKey);
    this.client = new ModularClient(provider);
  }

  async sendTransaction(tx: TransactionRequest): Promise<any> {
    if (!tx.nonce) {
      tx.nonce = await this.getNonce();
    }
    const signedTx = await this.wallet.signTransaction(tx);
    return await this.client.sendTransaction(signedTx);
  }

  async getBalance(): Promise<string> {
    return await this.client.getBalance(this.address);
  }

  get address(): string {
    return this.wallet.address;
  }

  async getNonce(): Promise<number> {
    return await this.client.getNonce(this.address);
  }

  getClient(): ModularClient {
    return this.client;
  }
}
