const fs = require("fs");
const path = require("path");

module.exports = async function scaffold(options) {
  const template = options.template || "hello-world";
  const name = options.name || `modular-${template}-dapp`;
  const outDir = path.resolve(options.out || name);

  if (fs.existsSync(outDir)) {
    console.error(`Error: "${outDir}" already exists.`);
    process.exit(1);
  }

  console.log(`\nScaffolding "${template}" dApp → ${outDir}\n`);

  // Create project structure
  const dirs = ["src", "contracts", "scripts", "test"];
  dirs.forEach(d => fs.mkdirSync(path.join(outDir, d), { recursive: true }));

  // package.json
  const pkg = {
    name,
    version: "0.1.0",
    private: true,
    scripts: {
      "compile": "npx hardhat compile",
      "deploy": "npx hardhat run scripts/deploy.ts",
      "test": "npx hardhat test",
      "start": "tsx src/index.ts",
    },
    dependencies: {
      "@modular-blockchain/sdk": "^1.0.0",
      "hardhat": "^2.19.0",
    },
    devDependencies: {
      "tsx": "^4.7.0",
      "typescript": "^5.3.0",
      "@nomicfoundation/hardhat-toolbox": "^4.0.0",
    },
  };
  fs.writeFileSync(path.join(outDir, "package.json"), JSON.stringify(pkg, null, 2));

  // hardhat.config.ts
  const hardhatConfig = `import { HardhatUserConfig } from "hardhat/config";
import "@nomicfoundation/hardhat-toolbox";

const config: HardhatUserConfig = {
  solidity: "0.8.20",
  networks: {
    modular: {
      url: process.env.RPC_URL || "http://localhost:9933",
      accounts: process.env.PRIVATE_KEY ? [process.env.PRIVATE_KEY] : [],
    },
  },
};

export default config;
`;
  fs.writeFileSync(path.join(outDir, "hardhat.config.ts"), hardhatConfig);

  // Sample contract
  const contractContent = `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract ${name.replace(/-/g, "").replace(/^\w/, c => c.toUpperCase())} {
    string public greeting;

    constructor(string memory _greeting) {
        greeting = _greeting;
    }

    function setGreeting(string memory _greeting) public {
        greeting = _greeting;
    }
}
`;
  fs.writeFileSync(path.join(outDir, "contracts", "Greeter.sol"), contractContent);

  // Deploy script
  const deployScript = `import { ethers } from "hardhat";

async function main() {
  const Greeter = await ethers.getContractFactory("Greeter");
  const greeter = await Greeter.deploy("Hello, Modular Blockchain!");
  await greeter.waitForDeployment();
  console.log("Greeter deployed:", await greeter.getAddress());
}

main().catch(console.error);
`;
  fs.writeFileSync(path.join(outDir, "scripts", "deploy.ts"), deployScript);

  // SDK client entry
  const sdkEntry = `import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";
import * as dotenv from "dotenv";

dotenv.config();

const RPC_URL = process.env.RPC_URL || "http://localhost:9933";

async function main() {
  const provider = new HttpProvider(RPC_URL);
  const client = new ModularClient(provider);

  const status = await client.getNodeStatus();
  console.log("Connected to Modular Blockchain");
  console.log("Block height:", status.height);

  // Interact with deployed contract
  // const contractAddress = "0x...";
  // const tx = await wallet.signTransaction({ to: contractAddress, data: "0x..." });
  // const result = await client.sendTransaction(tx);
}

main().catch(console.error);
`;
  fs.writeFileSync(path.join(outDir, "src", "index.ts"), sdkEntry);

  console.log("✅ Scaffolded project structure:");
  console.log(`  ${outDir}/`);
  console.log("    ├── contracts/");
  console.log("    ├── scripts/");
  console.log("    ├── src/");
  console.log("    ├── test/");
  console.log("    ├── hardhat.config.ts");
  console.log("    └── package.json");
  console.log("\nNext: cd", outDir, "&& npm install");
};
