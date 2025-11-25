# Developer Guide

Guide for building dApps on the Modular Blockchain Testnet.

## Getting Started

### 1. Install SDK

```bash
npm install @modular-blockchain/sdk
```

### 2. Connect to Testnet

```typescript
import { ModularClient, HttpProvider } from '@modular-blockchain/sdk';

const provider = new HttpProvider('https://rpc.testnet.modular.io');
const client = new ModularClient(provider);

// Verify connection
const chainId = await client.getChainId();
console.log('Connected to chain:', chainId);
```

### 3. Create Wallet

```typescript
import { Wallet } from '@modular-blockchain/sdk';

// Generate new wallet
const wallet = Wallet.generate();
console.log('Address:', wallet.address);

// Or import existing
const wallet = Wallet.fromPrivateKey('0x...');
```

### 4. Get Test Tokens

Visit the faucet: https://faucet.testnet.modular.io

Enter your address and request tokens (1000 tokens per 24 hours).

### 5. Send Transaction

```typescript
const connectedWallet = wallet.connect(provider);

const tx = await connectedWallet.sendTransaction({
  to: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb',
  value: '1000000000000000000', // 1 token
});

console.log('Transaction hash:', tx.transactionHash);

// Wait for confirmation
const receipt = await client.waitForTransaction(tx.transactionHash);
console.log('Confirmed in block:', receipt.blockNumber);
```

## Network Information

### Endpoints

- **RPC**: `https://rpc.testnet.modular.io`
- **WebSocket**: `wss://ws.testnet.modular.io`
- **API**: `https://api.testnet.modular.io`

### Chain Parameters

```typescript
{
  chainId: "modular-testnet-1",
  blockTime: 3000, // 3 seconds
  maxValidators: 100,
  minStake: "1000000000000000000000" // 1000 tokens
}
```

## Common Tasks

### Query Balance

```typescript
const balance = await client.getBalance(address);
console.log('Balance:', balance);
```

### Get Block Information

```typescript
const blockNumber = await client.getBlockNumber();
const block = await client.getBlock(blockNumber);
console.log('Latest block:', block);
```

### Listen for Events

```typescript
client.on('newBlock', (block) => {
  console.log('New block:', block.number);
});

client.on('newTransaction', (tx) => {
  console.log('New transaction:', tx.hash);
});
```

### Send Multiple Transactions

```typescript
const nonce = await wallet.getNonce();

for (let i = 0; i < 5; i++) {
  await wallet.sendTransaction({
    to: recipient,
    value: amount,
    nonce: nonce + i,
  });
}
```

## Bridge Usage

### Lock Tokens (Testnet → Ethereum Sepolia)

```typescript
import { BridgeClient } from '@modular-blockchain/sdk';

const bridge = new BridgeClient(wallet);

// Lock USDC
const lockTx = await bridge.lockTokens({
  token: '0x...', // USDC address
  amount: '1000000', // 1 USDC (6 decimals)
  recipient: '0x...', // Ethereum address
});

console.log('Tokens locked:', lockTx.transactionHash);
```

### Monitor Unlock Events

```typescript
bridge.on('TokensUnlocked', (event) => {
  console.log('Tokens unlocked:', {
    user: event.user,
    token: event.token,
    amount: event.amount,
  });
});
```

## Testing

### Unit Tests

```typescript
import { describe, it, expect } from '@jest/globals';
import { ModularClient, HttpProvider } from '@modular-blockchain/sdk';

describe('Testnet Integration', () => {
  it('should connect to testnet', async () => {
    const provider = new HttpProvider('https://rpc.testnet.modular.io');
    const client = new ModularClient(provider);
    
    const chainId = await client.getChainId();
    expect(chainId).toBe('modular-testnet-1');
  });
});
```

### Integration Tests

```typescript
it('should send and confirm transaction', async () => {
  const wallet = Wallet.fromPrivateKey(process.env.TEST_PRIVATE_KEY!);
  const connectedWallet = wallet.connect(provider);
  
  const tx = await connectedWallet.sendTransaction({
    to: testAddress,
    value: '1000',
  });
  
  const receipt = await client.waitForTransaction(tx.transactionHash);
  expect(receipt.status).toBe(1);
});
```

## Best Practices

### 1. Error Handling

```typescript
try {
  const tx = await wallet.sendTransaction({ to, value });
  await client.waitForTransaction(tx.transactionHash);
} catch (error) {
  if (error.message.includes('insufficient funds')) {
    console.error('Not enough balance');
  } else if (error.message.includes('nonce')) {
    console.error('Nonce mismatch');
  } else {
    console.error('Transaction failed:', error);
  }
}
```

### 2. Nonce Management

```typescript
// Get current nonce
const nonce = await wallet.getNonce();

// Send with explicit nonce
await wallet.sendTransaction({
  to,
  value,
  nonce,
});
```

### 3. Gas Estimation

```typescript
// Estimate gas for transaction
const gasEstimate = await client.estimateGas({
  to,
  value,
  data,
});

// Send with gas limit
await wallet.sendTransaction({
  to,
  value,
  gasLimit: gasEstimate,
});
```

### 4. Transaction Confirmation

```typescript
// Wait for multiple confirmations
const receipt = await client.waitForTransaction(
  txHash,
  3 // confirmations
);
```

## Rate Limits

### Faucet
- **Amount**: 1000 tokens per request
- **Cooldown**: 24 hours
- **Max Requests**: 10 per address

### RPC
- **Rate Limit**: 100 requests per second
- **Burst**: 200 requests
- **WebSocket**: 10 connections per IP

## Troubleshooting

### Connection Issues

```typescript
// Add timeout
const provider = new HttpProvider('https://rpc.testnet.modular.io', {
  timeout: 30000, // 30 seconds
});

// Retry logic
async function withRetry(fn, retries = 3) {
  for (let i = 0; i < retries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (i === retries - 1) throw error;
      await new Promise(r => setTimeout(r, 1000 * (i + 1)));
    }
  }
}
```

### Transaction Failures

Common reasons:
- Insufficient balance
- Nonce mismatch
- Gas limit too low
- Invalid recipient address

### Debugging

```typescript
// Enable debug logging
const client = new ModularClient(provider, {
  debug: true,
});

// Check transaction status
const tx = await client.getTransaction(txHash);
console.log('Transaction:', tx);

const receipt = await client.getTransactionReceipt(txHash);
console.log('Receipt:', receipt);
```

## Resources

### Documentation
- **SDK Docs**: https://docs.modular.io/sdk
- **API Reference**: https://docs.modular.io/api
- **Examples**: https://github.com/modular-blockchain/examples

### Tools
- **Explorer**: https://explorer.testnet.modular.io
- **Faucet**: https://faucet.testnet.modular.io
- **Bridge**: https://bridge.testnet.modular.io

### Support
- **Discord**: #testnet-developers
- **GitHub**: https://github.com/modular-blockchain
- **Stack Overflow**: Tag `modular-blockchain`

## Example Projects

### Simple Transfer dApp

```typescript
import { ModularClient, HttpProvider, Wallet } from '@modular-blockchain/sdk';

async function main() {
  const provider = new HttpProvider('https://rpc.testnet.modular.io');
  const wallet = Wallet.fromPrivateKey(process.env.PRIVATE_KEY!).connect(provider);
  
  const balance = await wallet.getBalance();
  console.log('Balance:', balance);
  
  const tx = await wallet.sendTransaction({
    to: process.env.RECIPIENT!,
    value: '1000000000000000000',
  });
  
  console.log('Transaction sent:', tx.transactionHash);
  
  const receipt = await provider.waitForTransaction(tx.transactionHash);
  console.log('Confirmed in block:', receipt.blockNumber);
}

main().catch(console.error);
```

### Token Faucet dApp

See: https://github.com/modular-blockchain/examples/tree/main/faucet-dapp

### NFT Marketplace

See: https://github.com/modular-blockchain/examples/tree/main/nft-marketplace

## Contributing

Found a bug or want to contribute?

1. **Report Issues**: GitHub Issues
2. **Submit PRs**: Follow contribution guidelines
3. **Join Community**: Discord #developers

## License

MIT
