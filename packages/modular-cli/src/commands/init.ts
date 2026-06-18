import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import chalk from "chalk";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TEMPLATES_DIR = path.resolve(__dirname, "../../templates");

const KIT_DESCRIPTIONS = {
  token: "ERC-20 token and ERC-721 NFT deployment",
  "nft-marketplace": "NFT minting, listing, buying, selling",
  defi: "AMM DEX with liquidity pools and swapping",
  dao: "DAO governance with proposal/vote/execute",
};

const KIT_FILES = {
  token: ["contracts/ERC20.sol", "scripts/deploy.ts", "scripts/interact.ts", "README.md"],
  "nft-marketplace": ["contracts/NFTMarketplace.sol", "contracts/ERC721.sol", "scripts/deploy.ts", "scripts/mint-and-list.ts", "README.md"],
  defi: ["contracts/SimpleAMM.sol", "contracts/ERC20.sol", "scripts/deploy.ts", "scripts/add-liquidity.ts", "scripts/swap.ts", "README.md"],
  dao: ["contracts/DAO.sol", "contracts/ERC20.sol", "scripts/deploy.ts", "scripts/propose-and-vote.ts", "README.md"],
};

export async function initCommand(dir: string, options: { kit: string }) {
  const kit = options.kit;
  if (!KIT_DESCRIPTIONS[kit]) {
    console.error(chalk.red(`Unknown kit "${kit}". Available: ${Object.keys(KIT_DESCRIPTIONS).join(", ")}`));
    process.exit(1);
  }

  const targetDir = path.resolve(process.cwd(), dir);
  if (fs.existsSync(targetDir)) {
    console.error(chalk.red(`Directory "${dir}" already exists`));
    process.exit(1);
  }

  console.log(chalk.cyan(`\n  Modular Blockchain — Init "${kit}" Kit\n`));
  console.log(`  ${KIT_DESCRIPTIONS[kit]}\n`);

  fs.mkdirSync(targetDir, { recursive: true });
  fs.mkdirSync(path.join(targetDir, "contracts"), { recursive: true });
  fs.mkdirSync(path.join(targetDir, "scripts"), { recursive: true });
  fs.mkdirSync(path.join(targetDir, "test"), { recursive: true });

  const packageJson = {
    name: path.basename(targetDir),
    version: "1.0.0",
    type: "module",
    scripts: {
      deploy: "npx tsx scripts/deploy.ts",
      test: "echo 'No tests configured yet'",
    },
    dependencies: {
      "@modular-blockchain/sdk": "^1.0.0",
    },
    devDependencies: {
      tsx: "^4.7.0",
      typescript: "^5.3.0",
    },
  };

  fs.writeFileSync(path.join(targetDir, "package.json"), JSON.stringify(packageJson, null, 2));
  fs.writeFileSync(path.join(targetDir, "README.md"), `# ${dir}\n\n${KIT_DESCRIPTIONS[kit]} starter kit.\n\n## Quick Start\n\n\`\`\`bash\nnpm install\nnpx tsx scripts/deploy.ts\n\`\`\`\n`);

  console.log(chalk.green(`  ✓ Created ${targetDir}`));
  console.log(chalk.green(`  ✓ package.json`));
  console.log(chalk.green(`  ✓ README.md`));
  console.log(chalk.green(`  ✓ contracts/`));
  console.log(chalk.green(`  ✓ scripts/`));
  console.log(chalk.green(`  ✓ test/\n`));
  console.log(chalk.dim(`  Next steps:\n    cd ${dir}\n    npm install\n    npx tsx scripts/deploy.ts\n`));
}
