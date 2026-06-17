const fs = require("fs");
const path = require("path");

module.exports = async function deployWizard() {
  console.log("\n📦 Modular Blockchain — Contract Deployment Wizard\n");

  // Step 1: Choose contract type
  console.log("Step 1: Select a contract template");
  const contractsDir = path.resolve(__dirname, "../../contracts");
  const contracts = fs.readdirSync(contractsDir)
    .filter(f => f.endsWith(".sol"))
    .map(f => f.replace(".sol", ""));

  contracts.forEach((c, i) => console.log(`  ${i + 1}. ${c}`));
  console.log("  4. Custom (your own bytecode)\n");

  // Step 2: RPC endpoint
  console.log("Step 2: Configure connection");
  const rpcUrl = "http://localhost:9933";
  console.log(`  RPC: ${rpcUrl}`);

  // Step 3: Wallet
  console.log("\nStep 3: Deployer wallet");
  const walletOption = "generate";
  console.log(`  Option: ${walletOption} (generate new / from key / from mnemonic)`);

  // Step 4: Constructor args
  console.log("\nStep 4: Constructor arguments");
  console.log("  (comma-separated, e.g.: MyToken,MTK,1000000)");

  // Step 5: Confirm
  console.log("\nStep 5: Confirm deployment");
  console.log("  Contract: ERC20");
  console.log("  Network:  localhost:9933");
  console.log("  Args:     MyToken, MTK, 1000000");

  console.log("\nTo deploy non-interactively, run:");
  console.log("  modular deploy --contract ERC20 --args \"MyToken,MTK,1000000\" --rpc http://localhost:9933");

  console.log("\n⚠  Interactive mode requires terminal input.");
  console.log("   Use the command above for automated deployments.");
};
