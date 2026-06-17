import { Wallet } from "../wallet";
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
});
