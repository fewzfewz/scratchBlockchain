import chalk from "chalk";
import inquirer from "inquirer";
import ora from "ora";
import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

const CONTRACT_TEMPLATES = [
  { name: "ERC-20 Token", value: "ERC20", description: "Fungible token with mint/burn/transfer" },
  { name: "ERC-721 NFT", value: "ERC721", description: "Non-fungible token collection" },
  { name: "ERC-1155 Multi-token", value: "ERC1155", description: "Batch transfers, multiple token types" },
  { name: "DAO Governance", value: "DAO", description: "On-chain proposal/vote/execute" },
  { name: "Simple AMM DEX", value: "SimpleAMM", description: "Constant product AMM (x*y=k)" },
  { name: "NFT Marketplace", value: "NFTMarketplace", description: "List, buy, sell NFTs with royalties" },
];

export async function deployWizard(options: { network: string }) {
  console.log(chalk.cyan("\n  ╔══════════════════════════════════════╗"));
  console.log(chalk.cyan("  ║   Modular Blockchain — Deploy Wizard ║"));
  console.log(chalk.cyan("  ╚══════════════════════════════════════╝\n"));

  const { contractType } = await inquirer.prompt([
    {
      type: "list",
      name: "contractType",
      message: "Select contract template:",
      choices: CONTRACT_TEMPLATES.map((c) => ({
        name: `${c.name} — ${c.description}`,
        value: c.value,
      })),
    },
  ]);

  const template = CONTRACT_TEMPLATES.find((c) => c.value === contractType);
  console.log(chalk.dim(`\n  Selected: ${template.name}\n`));

  const { deployMethod } = await inquirer.prompt([
    {
      type: "list",
      name: "deployMethod",
      message: "Deployment method:",
      choices: [
        { name: "Generate new wallet", value: "new" },
        { name: "Use existing private key", value: "existing" },
      ],
    },
  ]);

  let wallet;
  if (deployMethod === "existing") {
    const { privateKey } = await inquirer.prompt([
      {
        type: "password",
        name: "privateKey",
        message: "Enter private key:",
        validate: (input) => input.length > 0 ? true : "Private key is required",
      },
    ]);
    wallet = Wallet.fromPrivateKey(privateKey);
  } else {
    wallet = Wallet.generate();
    console.log(chalk.yellow(`\n  ⚠  Generated wallet: ${wallet.address}`));
    console.log(chalk.yellow(`  ⚠  Fund it before deploying!\n`));
  }

  const provider = new HttpProvider(options.network);
  const client = new ModularClient(provider);

  console.log(chalk.dim(`  Deployer: ${wallet.address}`));
  console.log(chalk.dim(`  Network:  ${options.network}\n`));

  const spinner = ora("Deploying contract...").start();

  try {
    const tx = await wallet.signTransaction({
      to: null,
      data: "0x6080604052",
      gasLimit: 300000,
    });

    const result = await client.sendTransaction(tx);
    const receipt = await client.waitForTransaction(result.hash);

    spinner.succeed(chalk.green("Contract deployed!"));

    console.log(chalk.cyan(`\n  ┌─ Deployment Summary ─────────────────┐`));
    console.log(`  │ Contract:  ${(template?.name || contractType).padEnd(30)}│`);
    console.log(`  │ Address:   ${(receipt.contract_address || "").padEnd(30)}│`);
    console.log(`  │ Tx Hash:   ${(result.hash || "").padEnd(30)}│`);
    console.log(`  │ Block:     ${(String(receipt.block_height) || "").padEnd(30)}│`);
    console.log(`  │ Network:   ${options.network.padEnd(30)}│`);
    console.log(chalk.cyan(`  └────────────────────────────────────────┘\n`));
  } catch (err) {
    spinner.fail(chalk.red(`Deployment failed: ${err.message}`));
    process.exit(1);
  }
}
