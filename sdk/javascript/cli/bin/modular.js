#!/usr/bin/env node

const { program } = require("commander");

program
  .name("modular")
  .description("Modular Blockchain Developer CLI")
  .version("1.0.0");

program
  .command("init")
  .description("Scaffold a new project")
  .option("-t, --template <type>", "Project type: defi, nft, dao, default", "default")
  .option("-n, --name <name>", "Project name", "my-modular-dapp")
  .action((opts) => require("../commands/init")(opts));

program
  .command("deploy")
  .description("Deploy a smart contract")
  .option("-c, --contract <name>", "Contract name (ERC20, ERC721, DAO)")
  .option("-a, --args <args>", "Constructor arguments (comma-separated)")
  .option("--rpc <url>", "RPC endpoint", "http://localhost:9933")
  .option("--key <privateKey>", "Deployer private key")
  .action((opts) => require("../commands/deploy")(opts));

program
  .command("create <type>")
  .description("Create a new contract or project file")
  .option("-n, --name <name>", "Contract name")
  .option("-o, --output <dir>", "Output directory", ".")
  .action((type, opts) => require("../commands/create")(type, opts));

program
  .command("scaffold")
  .description("Scaffold a full dApp project from a template")
  .option("-t, --template <type>", "Template (defi, nft, dao, hello-world)")
  .option("-n, --name <name>", "Project name")
  .option("-o, --out <dir>", "Output directory")
  .action((opts) => require("../commands/scaffold")(opts));

program
  .command("wallet")
  .description("Generate or inspect wallets")
  .option("-g, --generate", "Generate a new wallet")
  .option("--mnemonic", "Generate with mnemonic phrase")
  .option("--from-key <key>", "Import from private key")
  .option("--from-mnemonic <phrase>", "Import from mnemonic")
  .action((opts) => require("../commands/wallet")(opts));

program
  .command("deploy-wizard")
  .description("Interactive contract deployment wizard")
  .action(() => require("../commands/deploy-wizard")(program));

program.parse(process.argv);

if (!process.argv.slice(2).length) {
  program.outputHelp();
}
