import chalk from "chalk";
import { HttpProvider, ModularClient } from "@modular-blockchain/sdk";

export async function networkCommand(action: string, options: { network: string; address?: string }) {
  const provider = new HttpProvider(options.network);
  const client = new ModularClient(provider);

  switch (action) {
    case "status": {
      const status = await client.getNodeStatus();
      const peers = await client.getPeers();
      console.log(chalk.cyan(`\n  Network Status — ${options.network}\n`));
      console.log(`    Block Height:    ${status.height}`);
      console.log(`    Finalized:       ${status.finalized_height ?? "N/A"}`);
      console.log(`    Mempool Size:    ${status.mempool_size}`);
      console.log(`    Peers:           ${status.peer_count}`);
      console.log(`    Connected:       ${await client.healthCheck() ? chalk.green("✓") : chalk.red("✗")}\n`);
      break;
    }

    case "peers": {
      const peers = await client.getPeers();
      console.log(chalk.cyan(`\n  Peers (${peers.length}) — ${options.network}\n`));
      for (const peer of peers) {
        console.log(`    ${peer.id} @ ${peer.address} [${peer.direction}]`);
      }
      console.log();
      break;
    }

    case "faucet": {
      if (!options.address) {
        console.error(chalk.red("faucet requires --address"));
        process.exit(1);
      }
      console.log(chalk.cyan(`\n  Requesting faucet funds for ${options.address}\n`));
      try {
        const response = await fetch(`${options.network}/faucet/send`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ address: options.address }),
        });
        const data = await response.json();
        if (data.success) {
          console.log(chalk.green(`  ✓ Sent test tokens!`));
          console.log(`    Tx: ${data.txHash}\n`);
        } else {
          console.error(chalk.red(`  ✗ ${data.error}\n`));
        }
      } catch (err) {
        console.error(chalk.red(`  ✗ Faucet error: ${err.message}\n`));
      }
      break;
    }

    default:
      console.error(chalk.red(`Unknown action "${action}". Use: status, peers, faucet`));
      process.exit(1);
  }
}
