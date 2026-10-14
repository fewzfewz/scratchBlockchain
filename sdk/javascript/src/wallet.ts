import { ed25519 } from "@noble/curves/ed25519";
import { sha256 } from "@noble/hashes/sha256";
import { bytesToHex, hexToBytes } from "@noble/hashes/utils";
import { generateMnemonic, mnemonicToSeedSync } from "bip39";
import { Provider } from "./types/provider";
import { TransactionRequest, Transaction } from "./types/client";
import { ModularClient } from "./client";

function uint64LE(value: number | bigint): Uint8Array {
  const buf = new ArrayBuffer(8);
  const view = new DataView(buf);
  view.setBigUint64(0, BigInt(value), true);
  return new Uint8Array(buf);
}

function concatBytes(parts: Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let offset = 0;
  for (const p of parts) {
    out.set(p, offset);
    offset += p.length;
  }
  return out;
}

const TX_DOMAIN = new TextEncoder().encode("NEBULA_TX_V1");

/**
 * Bytes whose SHA-256 is signed — must match Rust `Transaction::hash`
 * (common/src/types.rs): a domain tag, a length-prefixed payload, and
 * presence tags on `chainId` / `to` so no two transactions share an encoding.
 */
export function transactionSigningBytes(tx: {
  sender: Uint8Array;
  nonce: number;
  publicKey: Uint8Array;
  payload: Uint8Array;
  gasLimit: number;
  maxFeePerGas: number;
  maxPriorityFeePerGas: number;
  chainId: number | null;
  to: Uint8Array | null;
  value: number;
}): Uint8Array {
  return concatBytes([
    TX_DOMAIN,
    tx.sender,
    uint64LE(tx.nonce),
    tx.publicKey,
    uint64LE(tx.payload.length),
    tx.payload,
    uint64LE(tx.gasLimit),
    uint64LE(tx.maxFeePerGas),
    uint64LE(tx.maxPriorityFeePerGas),
    tx.chainId !== null ? concatBytes([Uint8Array.of(1), uint64LE(tx.chainId)]) : Uint8Array.of(0),
    tx.to !== null ? concatBytes([Uint8Array.of(1), tx.to]) : Uint8Array.of(0),
    uint64LE(tx.value),
  ]);
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
    // Node mempools reject priority fees below 1 gwei.
    const maxPriorityFeePerGas = Number(tx.maxPriorityFeePerGas ?? 1000000000);
    // The node requires a chain ID (replay protection); default to the devnet's.
    const chainId = tx.chainId ?? 1;
    const to = tx.to ?? null;
    const value = Number(tx.value ?? 0);

    const senderBytes = hexToBytes(sender.startsWith("0x") ? sender.slice(2) : sender);
    const pubKeyBytes = hexToBytes(this.publicKey);
    const toBytes =
      to !== null ? hexToBytes(to.startsWith("0x") ? to.slice(2) : to) : null;
    const message = transactionSigningBytes({
      sender: senderBytes,
      nonce,
      publicKey: pubKeyBytes,
      payload: rawPayload,
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas,
      chainId,
      to: toBytes,
      value,
    });

    const hash = sha256(message);
    const signatureBytes = ed25519.sign(hash, this.privateKey);

    return {
      sender,
      nonce,
      publicKey: "0x" + this.publicKey,
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

  /** Submit a signed governance action (propose / vote) via POST /submit_tx. */
  async submitGovernanceAction(action: Record<string, unknown>): Promise<any> {
    const nonce = await this.getNonce();
    const body = new TextEncoder().encode(JSON.stringify(action));
    return this.sendTransaction({
      data: "0x" + bytesToHex(body),
      gasLimit: 21000,
      chainId: 1,
      nonce,
    });
  }

  async createProposal(req: {
    title: string;
    description?: string;
    proposal_type?: string;
    version?: string;
    hash?: string;
  }): Promise<any> {
    return this.submitGovernanceAction({
      action: "propose",
      title: req.title,
      description: req.description ?? "",
      proposal_type: req.proposal_type,
      version: req.version,
      hash: req.hash,
    });
  }

  /** Register this account as a validator, bonding `stake` wei from its balance. */
  async registerValidator(stake: string, commissionRate: number = 10): Promise<any> {
    return this.submitGovernanceAction({
      action: "register_validator",
      stake,
      commission_rate: commissionRate,
    });
  }

  /** Bond `amount` wei from this account to a registered validator. */
  async delegate(validator: string, amount: string): Promise<any> {
    return this.submitGovernanceAction({ action: "delegate", validator, amount });
  }

  async vote(proposalId: number, choice: "yes" | "no" | "abstain"): Promise<any> {
    return this.submitGovernanceAction({
      action: "vote",
      proposal_id: proposalId,
      choice,
    });
  }

  async executeProposal(proposalId: number): Promise<any> {
    return this.submitGovernanceAction({
      action: "execute",
      proposal_id: proposalId,
    });
  }

  getClient(): ModularClient {
    return this.client;
  }
}
