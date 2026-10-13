import { Wallet, transactionSigningBytes } from "../wallet";
import { ed25519 } from "@noble/curves/ed25519";
import { sha256 } from "@noble/hashes/sha256";
import { hexToBytes, bytesToHex } from "@noble/hashes/utils";

describe("Wallet", () => {
  it("should generate a wallet with valid Ed25519 key", () => {
    const wallet = Wallet.generate();
    expect(wallet.address).toMatch(/^0x[a-f0-9]{40}$/);
    expect(wallet.publicKey).toMatch(/^[a-f0-9]{64}$/);
    expect(wallet.getPrivateKey()).toMatch(/^[a-f0-9]{64}$/);
  });

  it("should derive correct address from public key", () => {
    const wallet = Wallet.generate();
    const expectedHash = sha256(hexToBytes(wallet.publicKey));
    const expectedAddress = "0x" + bytesToHex(expectedHash.slice(-20));
    expect(wallet.address).toBe(expectedAddress);
  });

  it("should sign and verify messages", async () => {
    const wallet = Wallet.generate();
    const message = "Hello, Blockchain!";
    const signature = await wallet.signMessage(message);
    expect(signature).toMatch(/^[a-f0-9]{128}$/);
    const isValid = Wallet.verifySignature(message, signature, wallet.publicKey);
    expect(isValid).toBe(true);
  });

  it("should sign transactions", async () => {
    const wallet = Wallet.generate();
    const tx = await wallet.signTransaction({
      to: "0x1234567890abcdef1234567890abcdef12345678",
      value: 1000,
      nonce: 0,
      chainId: 1,
    });

    expect(tx.sender).toBe(wallet.address);
    expect(tx.signature).toMatch(/^0x[a-f0-9]{128}$/);
    expect(tx.value).toBe(1000);
    expect(tx.nonce).toBe(0);
    expect(tx.payload).toBe("0x");
    expect(tx.to).toBe("0x1234567890abcdef1234567890abcdef12345678");
  });

  it("should create wallet from private key", () => {
    const wallet1 = Wallet.generate();
    const privKey = wallet1.getPrivateKey();
    const wallet2 = Wallet.fromPrivateKey(privKey);
    expect(wallet2.address).toBe(wallet1.address);
    expect(wallet2.publicKey).toBe(wallet1.publicKey);
  });

  it("should handle 0x-prefixed private keys", () => {
    const wallet1 = Wallet.generate();
    const privKey = "0x" + wallet1.getPrivateKey();
    const wallet2 = Wallet.fromPrivateKey(privKey);
    expect(wallet2.address).toBe(wallet1.address);
  });

  it("should sign transaction with data payload", async () => {
    const wallet = Wallet.generate();
    const tx = await wallet.signTransaction({
      data: "0xdeadbeef",
      gasLimit: 50000,
      maxFeePerGas: 2000000000,
      maxPriorityFeePerGas: 500000000,
      chainId: 1,
    });

    expect(tx.payload).toBe("0xdeadbeef");
    expect(tx.gasLimit).toBe(50000);
    expect(tx.maxFeePerGas).toBe(2000000000);
    expect(tx.maxPriorityFeePerGas).toBe(500000000);
    expect(tx.signature).toMatch(/^0x[a-f0-9]{128}$/);
  });

  it("matches the node's canonical tx hash (shared vector with Rust)", () => {
    const bytes = transactionSigningBytes({
      sender: new Uint8Array(20).fill(0x11),
      nonce: 7,
      publicKey: new Uint8Array(32).fill(0x22),
      payload: Uint8Array.of(1, 2, 3),
      gasLimit: 21000,
      maxFeePerGas: 1000000000,
      maxPriorityFeePerGas: 100000000,
      chainId: 1,
      to: new Uint8Array(20).fill(0x33),
      value: 5,
    });
    expect(bytesToHex(sha256(bytes))).toBe(
      "3413644368542c163b3a62867bd5c0743479b00d5cb16a3a0629b2891d5c70b9",
    );
  });

  it("signs with its public key and the signature verifies over the hash", async () => {
    const wallet = Wallet.generate();
    const tx = await wallet.signTransaction({ to: "0x" + "ab".repeat(20), value: 1, chainId: 1 });
    expect(tx.publicKey).toBe("0x" + wallet.publicKey);
    const ok = ed25519.verify(
      hexToBytes(tx.signature.slice(2)),
      hexToBytes(tx.hash!.slice(2)),
      hexToBytes(wallet.publicKey),
    );
    expect(ok).toBe(true);
  });
});
