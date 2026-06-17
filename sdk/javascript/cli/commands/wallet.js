module.exports = async function wallet(options) {
  const { Wallet } = require("@modular-blockchain/sdk");

  if (options.generate) {
    const w = Wallet.generate();
    console.log("\nGenerated Wallet");
    console.log("  Address:    ", w.address);
    console.log("  Public Key: ", w.publicKey);
    console.log("  Private Key:", w.getPrivateKey());
    console.log("\n⚠ Store the private key securely. Never share it.");
  } else if (options.fromKey) {
    const w = Wallet.fromPrivateKey(options.fromKey);
    console.log("\nImported Wallet");
    console.log("  Address:    ", w.address);
    console.log("  Public Key: ", w.publicKey);
  } else if (options.fromMnemonic) {
    const w = Wallet.fromMnemonic(options.fromMnemonic);
    console.log("\nWallet from Mnemonic");
    console.log("  Address:    ", w.address);
    console.log("  Public Key: ", w.publicKey);
  } else if (options.mnemonic) {
    const phrase = Wallet.generateMnemonic();
    const w = Wallet.fromMnemonic(phrase);
    console.log("\nGenerated Mnemonic Wallet");
    console.log("  Mnemonic:   ", phrase);
    console.log("  Address:    ", w.address);
    console.log("  Public Key: ", w.publicKey);
    console.log("\n⚠ Store the mnemonic phrase securely. Never share it.");
  } else {
    console.log("\nUsage:");
    console.log("  modular wallet --generate");
    console.log("  modular wallet --mnemonic");
    console.log("  modular wallet --from-key <privateKey>");
    console.log("  modular wallet --from-mnemonic <phrase>");
  }
};
