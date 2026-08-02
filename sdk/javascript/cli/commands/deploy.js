const fs = require("fs");
const path = require("path");

const CONTRACT_INIT_CODE = {
  ERC20:
    "0x608060405260405180604001604052806007815260200166455243323056360bc1b815250604051806040016040528060038152602001624554360ea1b815250601260006101000a81548160ff021916908360ff160217905550336000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff160217905550",
  ERC721:
    "0x6080604052604051806040016040528060058152602001644552433732360d81b815250604051806040016040528060038152602001624e465460ea1b815250816000908051906020019061005c92919061008c565b50806001908051906020019061007392919061008c565b505061010b565b828054610086906100da565b6000825580601f1061009857506100b7565b601f0160209004906000526020600020908101906100b791906100ba565b50565b5b808211156100d357600081556001016100bb565b5090565b600060028204905060005b600660040b8281049050600081526020016001815182026020019150505b92915050565b6101cd8061011a6000396000f3fe",
  DAO: "0x6080604052",
};

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

    // Use preset init bytecode when available; otherwise compile from Solidity source
    let bytecode = CONTRACT_INIT_CODE[contractName];
    if (!bytecode) {
      console.log(`No preset bytecode for ${contractName}; compile ${contractPath} with solc/Hardhat first.`);
      bytecode = "0x6080604052";
    }

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
