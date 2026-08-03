const hre = require("hardhat");
const fs = require("fs");
const path = require("path");

async function main() {
  console.log("Deploying Bridge contract...");

  // Get signers
  const [deployer, relayer1, relayer2, relayer3] = await hre.ethers.getSigners();
  
  console.log("Deploying with account:", deployer.address);
  console.log("Account balance:", (await hre.ethers.provider.getBalance(deployer.address)).toString());

  // Deploy parameters
  const chainId = 1; // Ethereum mainnet ID
  const destChainId = 100; // Modular blockchain ID
  const relayers = [relayer1.address, relayer2.address, relayer3.address];
  const requiredSignatures = 2;

  // Deploy Bridge
  const Bridge = await hre.ethers.getContractFactory("Bridge");
  const bridge = await Bridge.deploy(chainId, destChainId, relayers, requiredSignatures);

  await bridge.waitForDeployment();

  const bridgeAddress = await bridge.getAddress();
  console.log("Bridge deployed to:", bridgeAddress);

  // Deploy MockERC20 for testing
  const MockERC20 = await hre.ethers.getContractFactory("MockERC20");
  const usdc = await MockERC20.deploy("USD Coin", "USDC", 6);
  await usdc.waitForDeployment();
  
  const usdcAddress = await usdc.getAddress();
  console.log("Mock USDC deployed to:", usdcAddress);

  const usdt = await MockERC20.deploy("Tether USD", "USDT", 6);
  await usdt.waitForDeployment();
  
  const usdtAddress = await usdt.getAddress();
  console.log("Mock USDT deployed to:", usdtAddress);

  const deployment = {
    bridge: bridgeAddress,
    usdc: usdcAddress,
    usdt: usdtAddress,
    relayers: relayers,
    requiredSignatures: requiredSignatures,
    chainId: chainId,
    destChainId: destChainId,
    deployer: deployer.address,
    ethRpcUrl: process.env.ETH_RPC_URL || "http://127.0.0.1:9545",
    deployedAt: new Date().toISOString(),
  };

  const deployOut =
    process.env.BRIDGE_DEPLOY_OUT ||
    path.join(__dirname, "../../deployment/local/configs/bridge-deployment.json");
  const frontendOut =
    process.env.FRONTEND_BRIDGE_CONFIG ||
    path.join(__dirname, "../../frontend/public/bridge-config.json");

  fs.mkdirSync(path.dirname(deployOut), { recursive: true });
  fs.mkdirSync(path.dirname(frontendOut), { recursive: true });
  fs.writeFileSync(deployOut, JSON.stringify(deployment, null, 2));

  const frontendConfig = {
    bridge: bridgeAddress,
    ethRpcUrl: deployment.ethRpcUrl,
    usdc: usdcAddress,
    usdt: usdtAddress,
    deployed: true,
  };
  fs.writeFileSync(frontendOut, JSON.stringify(frontendConfig, null, 2));

  console.log("\nDeployment complete!");
  console.log(JSON.stringify(deployment, null, 2));
  console.log(`\nSaved: ${deployOut}`);
  console.log(`Saved: ${frontendOut}`);

  // Verify contracts on Etherscan (if not localhost)
  if (hre.network.name !== "localhost" && hre.network.name !== "hardhat") {
    console.log("\nWaiting for block confirmations...");
    await bridge.deploymentTransaction().wait(6);
    
    console.log("Verifying contracts on Etherscan...");
    await hre.run("verify:verify", {
      address: bridgeAddress,
      constructorArguments: [chainId, destChainId, relayers, requiredSignatures],
    });
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
