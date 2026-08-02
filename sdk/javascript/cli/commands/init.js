const fs = require("fs");
const path = require("path");

module.exports = async function init(options) {
  const projectName = options.name || "my-modular-dapp";
  const projectDir = path.resolve(projectName);

  if (fs.existsSync(projectDir)) {
    console.error(`Error: Directory "${projectName}" already exists.`);
    process.exit(1);
  }

  console.log(`\nScaffolding ${options.template} project: ${projectName}\n`);

  fs.mkdirSync(projectDir, { recursive: true });

  // Package.json
  const pkg = {
    name: projectName,
    version: "0.1.0",
    private: true,
    scripts: {
      start: "tsx src/index.ts",
      build: "tsup src/index.ts --format cjs,esm",
      test: "jest",
    },
    dependencies: {
      "@modular-blockchain/sdk": "^1.0.0",
      "dotenv": "^16.3.1",
    },
    devDependencies: {
      "tsx": "^4.7.0",
      "typescript": "^5.3.0",
    },
  };
  fs.writeFileSync(path.join(projectDir, "package.json"), JSON.stringify(pkg, null, 2));

  // tsconfig.json
  const tsconfig = {
    compilerOptions: {
      target: "ES2020",
      module: "commonjs",
      strict: true,
      esModuleInterop: true,
      outDir: "./dist",
      rootDir: "./src",
    },
    include: ["src/**/*"],
  };
  fs.writeFileSync(path.join(projectDir, "tsconfig.json"), JSON.stringify(tsconfig, null, 2));

  // .env.example
  fs.writeFileSync(path.join(projectDir, ".env.example"),
    "# Modular Blockchain Connection\n" +
    "RPC_URL=http://localhost:8545\n" +
    "PRIVATE_KEY=0x\n"
  );

  // Create src directory
  fs.mkdirSync(path.join(projectDir, "src"), { recursive: true });

  // Main entry file
  let mainContent;
  if (options.template === "defi") {
    mainContent = getDefiTemplate(projectName);
  } else if (options.template === "nft") {
    mainContent = getNFTTemplate(projectName);
  } else if (options.template === "dao") {
    mainContent = getDAOTemplate(projectName);
  } else {
    mainContent = getDefaultTemplate(projectName);
  }
  fs.writeFileSync(path.join(projectDir, "src", "index.ts"), mainContent);

  // .gitignore
  fs.writeFileSync(path.join(projectDir, ".gitignore"), "node_modules/\ndist/\n.env\n");

  console.log(`Created project at: ${projectDir}`);
  console.log("\nNext steps:");
  console.log(`  cd ${projectName}`);
  console.log("  npm install");
  console.log("  npm start");
};

function getDefaultTemplate(name) {
  return `import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

const RPC_URL = process.env.RPC_URL || "http://localhost:8545";

async function main() {
  const provider = new HttpProvider(RPC_URL);
  const client = new ModularClient(provider);

  const status = await client.getNodeStatus();
  console.log("Connected to Modular Blockchain");
  console.log("Height:", status.height);
  console.log("Peers:", status.peer_count);

  const wallet = Wallet.generate();
  console.log("\\nWallet:", wallet.address);
  console.log("Public Key:", wallet.publicKey);
}

main().catch(console.error);
`;
}

function getDefiTemplate(name) {
  return `import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

const RPC_URL = process.env.RPC_URL || "http://localhost:8545";

async function main() {
  const provider = new HttpProvider(RPC_URL);
  const client = new ModularClient(provider);
  const wallet = Wallet.fromPrivateKey(process.env.PRIVATE_KEY || Wallet.generate().getPrivateKey());

  console.log("DeFi App: ${name}");
  console.log("Wallet:", wallet.address);

  const balance = await client.getBalance(wallet.address);
  console.log("Balance:", balance);
}

main().catch(console.error);
`;
}

function getNFTTemplate(name) {
  return `import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

const RPC_URL = process.env.RPC_URL || "http://localhost:8545";

async function main() {
  const provider = new HttpProvider(RPC_URL);
  const client = new ModularClient(provider);
  const wallet = Wallet.fromPrivateKey(process.env.PRIVATE_KEY || Wallet.generate().getPrivateKey());

  console.log("NFT App: ${name}");
  console.log("Wallet:", wallet.address);
}

main().catch(console.error);
`;
}

function getDAOTemplate(name) {
  return `import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

const RPC_URL = process.env.RPC_URL || "http://localhost:8545";

async function main() {
  const provider = new HttpProvider(RPC_URL);
  const client = new ModularClient(provider);
  const wallet = Wallet.fromPrivateKey(process.env.PRIVATE_KEY || Wallet.generate().getPrivateKey());

  console.log("DAO App: ${name}");
  console.log("Wallet:", wallet.address);
}

main().catch(console.error);
`;
}
