#!/usr/bin/env node

import { Command } from "commander";
import chalk from "chalk";
import { initCommand } from "./commands/init.js";
import { deployCommand } from "./commands/deploy.js";
import { generateCommand } from "./commands/generate.js";
import { walletCommand } from "./commands/wallet.js";
import { networkCommand } from "./commands/network.js";
import { deployWizard } from "./commands/deploy-wizard.js";

const program = new Command();

program
  .name("modular")
  .description("Modular Blockchain CLI — develop, deploy, and manage dApps")
  .version("1.0.0");

program
  .command("init")
  .description("Scaffold a new project from a starter kit")
  .argument("[directory]", "Project directory name", "my-dapp")
  .option("-k, --kit <kit>", "Starter kit: token, nft-marketplace, defi, dao", "token")
  .action(initCommand);

program
  .command("deploy")
  .description("Deploy a smart contract to the network")
  .argument("<contract>", "Contract type: ERC20, ERC721, ERC1155, DAO, SimpleAMM, NFTMarketplace")
  .option("-a, --args <args>", "Comma-separated constructor arguments", "")
  .option("-n, --network <url>", "RPC URL", "http://localhost:9933")
  .option("-w, --wallet <key>", "Private key for deployment")
  .option("-g, --gas <limit>", "Gas limit", "300000")
  .action(deployCommand);

program
  .command("deploy-wizard")
  .description("Interactive deployment wizard")
  .option("-n, --network <url>", "RPC URL", "http://localhost:9933")
  .action(deployWizard);

program
  .command("generate")
  .description("Generate contract or script scaffolding")
  .argument("<type>", "Scaffold type: contract, script, test")
  .option("-n, --name <name>", "Name of the generated file")
  .option("-t, --template <template>", "Template: erc20, erc721, erc1155, dao, amm, marketplace")
  .action(generateCommand);

program
  .command("wallet")
  .description("Wallet operations: generate, fund, balance")
  .argument("<action>", "Action: generate, balance, send")
  .option("-p, --private-key <key>", "Private key for wallet operations")
  .option("-n, --network <url>", "RPC URL", "http://localhost:9933")
  .option("-a, --address <addr>", "Target address")
  .option("-t, --to <addr>", "Recipient address (for send)")
  .option("--amount <nbl>", "Amount in NBL (for send)")
  .action(walletCommand);

program
  .command("network")
  .description("Network operations: status, peers, faucet")
  .argument("<action>", "Action: status, peers, faucet")
  .option("-n, --network <url>", "RPC URL", "http://localhost:9933")
  .option("-a, --address <addr>", "Address for faucet")
  .action(networkCommand);

program.parse(process.argv);

if (!process.argv.slice(2).length) {
  program.outputHelp();
}
