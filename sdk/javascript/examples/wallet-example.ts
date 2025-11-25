import { Wallet, HttpProvider } from '../src';

async function main() {
  // 1. Generate new wallet
  const wallet1 = Wallet.generate();
  console.log('Generated wallet:');
  console.log('  Address:', wallet1.address);
  console.log('  Public Key:', wallet1.publicKey);
  console.log('  Private Key:', wallet1.getPrivateKey());

  // 2. Create wallet from private key
  const privateKey = wallet1.getPrivateKey();
  const wallet2 = Wallet.fromPrivateKey(privateKey);
  console.log('\nRestored wallet:', wallet2.address);

  // 3. Create wallet from mnemonic
  const mnemonic = 'test mnemonic phrase for wallet generation';
  const wallet3 = Wallet.fromMnemonic(mnemonic);
  console.log('\nWallet from mnemonic:', wallet3.address);

  // 4. Sign message
  const message = 'Hello, Modular Blockchain!';
  const signature = await wallet1.signMessage(message);
  console.log('\nMessage signature:', signature);

  // 5. Connect wallet to provider
  const provider = new HttpProvider('http://localhost:8545');
  const connectedWallet = wallet1.connect(provider);

  // 6. Get wallet balance
  try {
    const balance = await connectedWallet.getBalance();
    console.log('\nWallet balance:', balance);
  } catch (error) {
    console.log('\nCould not get balance (node not running)');
  }

  // 7. Send transaction
  try {
    const tx = await connectedWallet.sendTransaction({
      to: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb',
      value: '1000000000000000000', // 1 token
    });
    console.log('\nTransaction sent:', tx.transactionHash);
  } catch (error) {
    console.log('\nCould not send transaction (node not running)');
  }
}

main().catch(console.error);
