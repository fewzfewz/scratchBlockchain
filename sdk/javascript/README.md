# Modular Blockchain SDK

Official JavaScript/TypeScript SDK for interacting with the Modular Blockchain.

## Installation

```bash
npm install @modular-blockchain/sdk
```

## Quick Start

```typescript
import { ModularClient, HttpProvider, Wallet } from '@modular-blockchain/sdk';

// Connect to blockchain
const provider = new HttpProvider('http://localhost:8545');
const client = new ModularClient(provider);

// Get chain info
const chainId = await client.getChainId();
const blockNumber = await client.getBlockNumber();

// Create wallet
const wallet = Wallet.generate();
const connectedWallet = wallet.connect(provider);

// Send transaction
const tx = await connectedWallet.sendTransaction({
  to: '0x...',
  value: '1000000000000000000', // 1 token
});

console.log('Transaction hash:', tx.transactionHash);
```

## Features

- ✅ **TypeScript Support** - Full type definitions
- ✅ **Wallet Management** - Generate, import, and manage wallets
- ✅ **Transaction Building** - Easy transaction creation and signing
- ✅ **Provider System** - HTTP and WebSocket providers
- ✅ **Event Listening** - Subscribe to blockchain events
- ✅ **Comprehensive API** - Complete blockchain interaction

## API Reference

### ModularClient

Main client for blockchain interaction.

```typescript
const client = new ModularClient(provider, options);

// Chain info
await client.getChainId();
await client.getBlockNumber();
await client.getBlock(blockNumber);
await client.getLatestBlock();

// Account
await client.getBalance(address);
await client.getAccount(address);
await client.getNonce(address);

// Transactions
await client.sendTransaction(tx);
await client.getTransaction(hash);
await client.getTransactionReceipt(hash);
await client.waitForTransaction(hash, confirmations);

// Events
client.on('newBlock', callback);
client.off('newBlock', callback);
```

### Wallet

Wallet for key management and signing.

```typescript
// Create wallet
const wallet = Wallet.generate();
const wallet = Wallet.fromPrivateKey(privateKey);
const wallet = Wallet.fromMnemonic(mnemonic);

// Properties
wallet.address;
wallet.publicKey;
wallet.getPrivateKey();

// Signing
await wallet.signMessage(message);
await wallet.signTransaction(tx);

// Connect to provider
const connectedWallet = wallet.connect(provider);
await connectedWallet.sendTransaction(tx);
await connectedWallet.getBalance();
await connectedWallet.getNonce();
```

### HttpProvider

HTTP provider for JSON-RPC communication.

```typescript
const provider = new HttpProvider('http://localhost:8545', {
  timeout: 30000,
  headers: { 'Custom-Header': 'value' },
});

await provider.request('method_name', [param1, param2]);
```

## Examples

### Basic Usage

```typescript
import { ModularClient, HttpProvider } from '@modular-blockchain/sdk';

const provider = new HttpProvider('http://localhost:8545');
const client = new ModularClient(provider);

const balance = await client.getBalance('0x...');
console.log('Balance:', balance);
```

### Wallet Operations

```typescript
import { Wallet } from '@modular-blockchain/sdk';

// Generate new wallet
const wallet = Wallet.generate();
console.log('Address:', wallet.address);

// Sign message
const signature = await wallet.signMessage('Hello!');
console.log('Signature:', signature);
```

### Send Transaction

```typescript
import { Wallet, HttpProvider } from '@modular-blockchain/sdk';

const provider = new HttpProvider('http://localhost:8545');
const wallet = Wallet.fromPrivateKey('0x...').connect(provider);

const tx = await wallet.sendTransaction({
  to: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb',
  value: '1000000000000000000',
});

console.log('Transaction:', tx.transactionHash);
```

### Listen for Events

```typescript
import { ModularClient, HttpProvider } from '@modular-blockchain/sdk';

const provider = new HttpProvider('http://localhost:8545');
const client = new ModularClient(provider);

client.on('newBlock', (block) => {
  console.log('New block:', block.number);
});
```

## TypeScript

The SDK is written in TypeScript and provides full type definitions.

```typescript
import {
  ModularClient,
  Block,
  Transaction,
  TransactionReceipt,
  Account,
} from '@modular-blockchain/sdk';

const client: ModularClient = new ModularClient(provider);
const block: Block = await client.getBlock(1);
const tx: Transaction = await client.getTransaction(hash);
```

## Error Handling

```typescript
try {
  const tx = await wallet.sendTransaction({
    to: '0x...',
    value: '1000000000000000000',
  });
} catch (error) {
  if (error.message.includes('insufficient funds')) {
    console.error('Not enough balance');
  } else {
    console.error('Transaction failed:', error.message);
  }
}
```

## Advanced Usage

### Custom Provider

```typescript
import { Provider } from '@modular-blockchain/sdk';

class CustomProvider implements Provider {
  async request(method: string, params?: any[]): Promise<any> {
    // Custom implementation
  }
}

const provider = new CustomProvider();
const client = new ModularClient(provider);
```

### Transaction Options

```typescript
await wallet.sendTransaction({
  to: '0x...',
  value: '1000000000000000000',
  data: '0x...',
  gasLimit: '21000',
  gasPrice: '1000000000',
  nonce: 5,
});
```

### Wait for Confirmations

```typescript
const tx = await wallet.sendTransaction({ to: '0x...', value: '1000' });

// Wait for 3 confirmations
const receipt = await client.waitForTransaction(tx.transactionHash, 3);
console.log('Confirmed in block:', receipt.blockNumber);
```

## Development

```bash
# Install dependencies
npm install

# Build
npm run build

# Test
npm test

# Lint
npm run lint

# Generate docs
npm run docs
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## License

MIT

## Support

- Documentation: https://docs.modular-blockchain.io
- Discord: https://discord.gg/modular-blockchain
- GitHub: https://github.com/modular-blockchain/sdk

## Changelog

### 1.0.0

- Initial release
- Core client functionality
- Wallet management
- HTTP provider
- TypeScript support
