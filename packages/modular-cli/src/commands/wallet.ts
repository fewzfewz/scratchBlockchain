import chalk from "chalk";
import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

export async function walletCommand(action: string, options: { privateKey?: string; network: string; address?: string; to?: string; amount?: string }) {
  switch (action) {
    case "generate": {
      const wallet = options.privateKey ? Wallet.fromPrivateKey(options.privateKey) : Wallet.generate();
      console.log(chalk.cyan(`\n  Wallet\n`));
      console.log(`    Address:    ${wallet.address}`);
      console.log(`    Public Key: ${wallet.publicKey}`);
      if (!options.privateKey) {
        console.log(chalk.yellow(`\n    ⚠  Save the private key securely!`));
      }
      console.log();
      break;
    }

    case "balance": {
      const provider = new HttpProvider(options.network);
      const client = new ModularClient(provider);
      let address = options.address;
      if (!address && options.privateKey) {
        address = Wallet.fromPrivateKey(options.privateKey).address;
      }
      if (!address) {
        console.error(chalk.red("Provide --address or --private-key"));
        process.exit(1);
      }
      const balance = await client.getBalance(address);
      console.log(chalk.cyan(`\n  Balance for ${address}\n`));
      console.log(`    ${chalk.bold(balance)} NBL\n`);
      break;
    }

    case "send": {
      const provider = new HttpProvider(options.network);
      const client = new ModularClient(provider);
      if (!options.privateKey || !options.to || !options.amount) {
        console.error(chalk.red("send requires --private-key, --to, and --amount"));
        process.exit(1);
      }
      const wallet = Wallet.fromPrivateKey(options.privateKey);
      const tx = await wallet.signTransaction({
        to: options.to,
        value: options.amount,
        gasLimit: 21000,
      });
      const result = await client.sendTransaction(tx);
      console.log(chalk.green(`\n  ✓ Sent ${options.amount} NBL to ${options.to}`));
      console.log(`    Tx: ${result.hash}\n`);
      break;
    }

    default:
      console.error(chalk.red(`Unknown action "${action}". Use: generate, balance, send`));
      process.exit(1);
  }
}
