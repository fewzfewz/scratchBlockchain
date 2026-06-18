import chalk from "chalk";
import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

const CONTRACT_INIT_CODE = {
  ERC20: "0x608060405260405180604001604052806007815260200166455243323056360bc1b815250604051806040016040528060038152602001624554360ea1b815250601260006101000a81548160ff021916908360ff160217905550336000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff160217905550",
  ERC721: "0x6080604052604051806040016040528060058152602001644552433732360d81b815250604051806040016040528060038152602001624e465460ea1b815250816000908051906020019061005c92919061008c565b50806001908051906020019061007392919061008c565b505061010b565b828054610086906100da565b6000825580601f1061009857506100b7565b601f0160209004906000526020600020908101906100b791906100ba565b50565b5b808211156100d357600081556001016100bb565b5090565b600060028204905060005b600660040b8281049050600081526020016001815182026020019150505b92915050565b6101cd8061011a6000396000f3fe",
};

export async function deployCommand(contractType: string, options: { args: string; network: string; wallet?: string; gas: string }) {
  console.log(chalk.cyan(`\n  Modular Blockchain — Deploy ${contractType}\n`));

  const provider = new HttpProvider(options.network);
  const client = new ModularClient(provider);

  let wallet: Wallet;
  if (options.wallet) {
    wallet = Wallet.fromPrivateKey(options.wallet);
  } else {
    wallet = Wallet.generate();
    console.log(chalk.yellow(`  ⚠  Generated new wallet: ${wallet.address}`));
    console.log(chalk.yellow(`  ⚠  Fund this address before deploying\n`));
  }

  const initCode = CONTRACT_INIT_CODE[contractType];
  if (!initCode) {
    console.error(chalk.red(`Unknown contract type "${contractType}". Available: ${Object.keys(CONTRACT_INIT_CODE).join(", ")}`));
    console.error(chalk.dim("For custom contracts, use the deployment script in starter-kits/ or contracts/"));
    process.exit(1);
  }

  const tx = await wallet.signTransaction({
    to: null,
    data: initCode,
    gasLimit: Number(options.gas),
  });

  try {
    const result = await client.sendTransaction(tx);
    const receipt = await client.waitForTransaction(result.hash);

    console.log(chalk.green(`  ✓ Contract deployed`));
    console.log(`    Address:  ${receipt.contract_address}`);
    console.log(`    Tx Hash:  ${result.hash}`);
    console.log(`    Block:    ${receipt.block_height}\n`);
    console.log(chalk.dim(`  Network: ${options.network}`));
  } catch (err) {
    console.error(chalk.red(`  ✗ Deployment failed: ${err.message}`));
    process.exit(1);
  }
}
