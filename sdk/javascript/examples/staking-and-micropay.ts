import { ModularClient, HttpProvider, Wallet } from 'nebula-chain-sdk';

const RPC_URL = process.env.NEBULA_RPC_URL || 'https://scratchblockchain.onrender.com';

async function main() {
  const provider = new HttpProvider(RPC_URL);
  const client = new ModularClient(provider);

  const status = await client.getNodeStatus();
  console.log(`Connected to Nebula @ ${RPC_URL} (height=${status.height})`);

  const wallet = Wallet.generate();
  console.log('Generated wallet:', wallet.address);

  const balance = await client.getBalance(wallet.address);
  console.log('Current balance:', balance);

  // Sign a 0.01 NBL micro-payment (fund via /faucet before broadcasting)
  const signedTx = await wallet.signTransaction({
    to: '0x000000000000000000000000000000000000dEaD',
    value: '10000000000000000',
  });
  console.log('Signed micro-payment tx:', signedTx);
}

main().catch(console.error);
