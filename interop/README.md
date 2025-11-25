# Bridge Infrastructure

Cross-chain bridge for transferring tokens between Ethereum and the modular blockchain.

## Overview

The bridge consists of two main components:

1. **Ethereum Smart Contracts** (Solidity) - Lock/unlock tokens on Ethereum
2. **Rust Bridge Service** - Lock/unlock tokens on the modular blockchain

## Architecture

```
Ethereum                          Modular Blockchain
┌─────────────┐                   ┌─────────────┐
│ Bridge.sol  │◄──────────────────┤ EthereumBridge│
│             │    Relayers       │             │
│ - Lock ETH  │                   │ - Lock tokens│
│ - Unlock    │                   │ - Unlock     │
│ - Multi-sig │                   │ - Verify sigs│
└─────────────┘                   └─────────────┘
```

## Smart Contracts

### Bridge.sol

Main bridge contract with the following features:

- **Lock/Unlock Mechanism**: Lock tokens on one chain, unlock on the other
- **Multi-Signature Verification**: Requires N of M relayer signatures
- **Pausable**: Emergency pause functionality
- **Reentrancy Protection**: SafeERC20 and ReentrancyGuard
- **Token Support**: Native ETH and ERC20 tokens

### Supported Tokens

- **ETH** (Native Ethereum)
- **USDC** (USD Coin)
- **USDT** (Tether USD)

## Setup

### Prerequisites

- Node.js v18+
- Hardhat
- Rust 1.70+

### Install Dependencies

```bash
cd interop
npm install
```

### Compile Contracts

```bash
npm run compile
```

### Deploy Contracts

#### Local Deployment

```bash
# Start local Ethereum node
npx hardhat node

# Deploy contracts (in another terminal)
npm run deploy:local
```

#### Testnet Deployment

```bash
# Set environment variables
export SEPOLIA_RPC_URL="https://sepolia.infura.io/v3/YOUR_KEY"
export PRIVATE_KEY="your_private_key"

# Deploy to Sepolia
npm run deploy:sepolia
```

## Usage

### Locking Tokens (Ethereum → Modular Blockchain)

```javascript
// Lock ETH
const tx = await bridge.lockTokens(
  ethers.ZeroAddress, // ETH
  ethers.parseEther("1.0"), // 1 ETH
  recipientBytes32, // Recipient on modular blockchain
  { value: ethers.parseEther("1.0") }
);

// Lock ERC20
const usdc = await ethers.getContractAt("IERC20", usdcAddress);
await usdc.approve(bridgeAddress, amount);
await bridge.lockTokens(usdcAddress, amount, recipientBytes32);
```

### Unlocking Tokens (Modular Blockchain → Ethereum)

```javascript
// Relayers sign the message
const messageHash = await bridge.hashMessage(message);
const signatures = await collectRelayerSignatures(messageHash);

// Submit to Ethereum
await bridge.unlockTokens(message, signatures);
```

## Rust Bridge Service

### Token Registry

```rust
use interop::token_registry::TokenRegistry;

let registry = TokenRegistry::default();

// Check if token is supported
if registry.is_supported("USDC") {
    // Validate amount
    registry.validate_amount("USDC", 1_000_000)?;
}
```

### Ethereum Bridge

```rust
use interop::ethereum_bridge::EthereumBridge;

let relayers = vec![[1u8; 20], [2u8; 20], [3u8; 20]];
let mut bridge = EthereumBridge::new(1, 2, relayers, 2);

// Lock tokens
let message = bridge.lock_tokens(
    user_address,
    token_address,
    amount,
    eth_recipient
)?;

// Unlock tokens
bridge.unlock_tokens(message)?;
```

## Testing

### Smart Contract Tests

```bash
npm test
```

### Rust Tests

```bash
cargo test -p interop
```

### Integration Tests

```bash
# Start local Ethereum node
npx hardhat node

# Run integration tests
cargo test -p interop --test bridge_integration
```

## Security

### Smart Contract Security

- ✅ OpenZeppelin contracts (Pausable, ReentrancyGuard, Ownable)
- ✅ Multi-signature verification
- ✅ Replay attack prevention
- ✅ Reentrancy protection
- ✅ SafeERC20 for token transfers

### Relayer Security

- ✅ Authorized relayer list
- ✅ Signature verification
- ✅ Threshold signatures (N of M)
- ✅ Message nonce tracking

### Recommended Practices

1. **Key Management**: Use hardware wallets or HSM for relayer keys
2. **Monitoring**: Set up alerts for bridge events
3. **Rate Limiting**: Implement daily/hourly limits
4. **Gradual Rollout**: Start with low limits, increase gradually
5. **Emergency Pause**: Have procedures for pausing the bridge

## Configuration

### Environment Variables

```bash
# Ethereum
SEPOLIA_RPC_URL=https://sepolia.infura.io/v3/YOUR_KEY
MAINNET_RPC_URL=https://mainnet.infura.io/v3/YOUR_KEY
PRIVATE_KEY=your_private_key

# Bridge
BRIDGE_ADDRESS=0x...
RELAYER_ADDRESSES=0x...,0x...,0x...
REQUIRED_SIGNATURES=2
```

## Deployment Addresses

### Sepolia Testnet

- Bridge: `TBD`
- Mock USDC: `TBD`
- Mock USDT: `TBD`

### Mainnet

- Bridge: `TBD`
- USDC: `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48`
- USDT: `0xdAC17F958D2ee523a2206206994597C13D831ec7`

## Monitoring

### Events to Monitor

- `TokensLocked`: User locked tokens
- `TokensUnlocked`: User unlocked tokens
- `MessageProcessed`: Message was processed
- `RelayerAdded`: New relayer added
- `RelayerRemoved`: Relayer removed

### Metrics

- Total value locked (TVL)
- Daily transaction volume
- Average transaction time
- Relayer uptime
- Failed transaction rate

## Troubleshooting

### Common Issues

**Issue**: Transaction reverts with "Insufficient signatures"
- **Solution**: Ensure enough relayers have signed the message

**Issue**: "Message already processed"
- **Solution**: Check if the message ID has been used before

**Issue**: "Invalid source chain"
- **Solution**: Verify the message source chain matches expected chain ID

## Contributing

See the main project README for contribution guidelines.

## License

MIT
