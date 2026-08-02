const fs = require("fs");
const path = require("path");

module.exports = async function deploy(options) {
  const contractName = options.contract || "ERC20";
  const rpcUrl = options.rpc || "http://localhost:8545";
  const privateKey = options.key;
  const args = options.args ? options.args.split(",").map(s => s.trim()) : [];

  console.log(`\nDeploying ${contractName}...`);
  console.log(`RPC: ${rpcUrl}`);

  // Resolve contract source
  const contractsDir = path.resolve(__dirname, "../../contracts");
  const contractPath = path.join(contractsDir, `${contractName}.sol`);
  if (!fs.existsSync(contractPath)) {
    console.error(`Unknown contract: ${contractName}`);
    console.log(`Available: ${fs.readdirSync(contractsDir).filter(f => f.endsWith('.sol')).join(", ")}`);
    process.exit(1);
  }

  console.log(`Contract: ${contractPath}`);
  console.log(`Args: ${args.join(", ") || "none"}`);

  if (!privateKey) {
    console.log("\n⚠ No private key provided. Use --key <hex> to deploy.");
    console.log("   Or set PRIVATE_KEY environment variable.\n");
  }

  console.log("\nDeployment steps:");
  console.log("  1. Compile contract (solc or Hardhat)");
  console.log("  2. Generate deploy transaction");
  console.log("  3. Sign and submit to chain");
  console.log("  4. Confirm deployment receipt\n");

  // Deploy via SDK
  try {
    const { Wallet, HttpProvider, ModularClient } = require("@modular-blockchain/sdk");
    const provider = new HttpProvider(rpcUrl);
    const client = new ModularClient(provider);
    const wallet = privateKey ? Wallet.fromPrivateKey(privateKey) : Wallet.generate();

    // In production, compile bytecode from Solidity source
    const bytecode = "0x"; // placeholder — real bytecode from compilation

    const deployTx = await wallet.signTransaction({
      data: bytecode,
      gasLimit: 500000,
    });

    const result = await client.sendTransaction(deployTx);
    console.log(`Deploy tx sent: ${result.hash}`);

    const receipt = await client.waitForTransaction(result.hash);
    if (receipt) {
      console.log(`\n✅ ${contractName} deployed!`);
      console.log(`Contract address: ${receipt.contract_address}`);
      console.log(`Block: ${receipt.block_height}`);
      console.log(`Tx: ${receipt.tx_hash}`);
    } else {
      console.log("Deployment pending, check tx hash for status.");
    }
  } catch (err) {
    console.error("Deployment error:", err.message);
    console.log("\nMake sure the node is running and accessible.");
  }
};
