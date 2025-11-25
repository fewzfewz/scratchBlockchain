import { ModularClient, HttpProvider, Wallet } from '../src';

async function main() {
  // 1. Connect to blockchain
  const provider = new HttpProvider('http://localhost:8545');
  const client = new ModularClient(provider);

  console.log('Connecting to blockchain...');
  await client.connect();

  // 2. Get chain information
  const chainId = await client.getChainId();
  const blockNumber = await client.getBlockNumber();
  console.log(`Connected to chain ${chainId}, block ${blockNumber}`);

  // 3. Create wallet
  const wallet = Wallet.generate();
  console.log('Generated wallet:', wallet.address);

  // 4. Get balance
  const balance = await client.getBalance(wallet.address);
  console.log('Balance:', balance);

  // 5. Get latest block
  const latestBlock = await client.getLatestBlock();
  console.log('Latest block:', latestBlock.number, latestBlock.hash);

  // 6. Listen for new blocks
  client.on('newBlock', (block) => {
    console.log('New block:', block.number);
  });
}

main().catch(console.error);
