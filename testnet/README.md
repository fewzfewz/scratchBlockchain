# Modular Blockchain Testnet

Welcome to the Modular Blockchain Testnet!

## Network Information

- **Chain ID**: `modular-testnet-1`
- **Block Time**: 3 seconds
- **RPC Endpoint**: `https://rpc.testnet.modular.io`
- **WebSocket**: `wss://ws.testnet.modular.io`
- **Explorer**: `https://explorer.testnet.modular.io`
- **Faucet**: `https://faucet.testnet.modular.io`

## Quick Links

- [Validator Guide](docs/VALIDATOR_GUIDE.md)
- [Developer Guide](docs/DEVELOPER_GUIDE.md)
- [API Documentation](docs/API.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)

## Getting Started

### For Validators

1. **Requirements**
   - Ubuntu 20.04+ or similar
   - 4 CPU cores
   - 8 GB RAM
   - 100 GB SSD
   - Public IP address

2. **Quick Setup**
   ```bash
   curl -sSL https://raw.githubusercontent.com/modular-blockchain/testnet/main/scripts/setup-validator.sh | bash
   ```

3. **Manual Setup**
   See [Validator Guide](docs/VALIDATOR_GUIDE.md)

### For Developers

1. **Install SDK**
   ```bash
   npm install @modular-blockchain/sdk
   ```

2. **Connect to Testnet**
   ```typescript
   import { ModularClient, HttpProvider } from '@modular-blockchain/sdk';
   
   const provider = new HttpProvider('https://rpc.testnet.modular.io');
   const client = new ModularClient(provider);
   ```

3. **Get Test Tokens**
   Visit [https://faucet.testnet.modular.io](https://faucet.testnet.modular.io)

## Resources

### Endpoints

- **RPC**: `https://rpc.testnet.modular.io`
- **RPC (Backup)**: `https://rpc2.testnet.modular.io`
- **WebSocket**: `wss://ws.testnet.modular.io`
- **API**: `https://api.testnet.modular.io`

### Services

- **Faucet**: `https://faucet.testnet.modular.io`
- **Explorer**: `https://explorer.testnet.modular.io`
- **Bridge**: `https://bridge.testnet.modular.io`
- **Monitoring**: `https://monitor.testnet.modular.io`

### Smart Contracts

- **Bridge (Sepolia)**: `0x...` (TBD)
- **USDC (Testnet)**: `0x...` (TBD)
- **USDT (Testnet)**: `0x...` (TBD)

## Testnet Parameters

```toml
chain_id = "modular-testnet-1"
block_time = "3s"
max_validators = 100
min_stake = "1000 tokens"
proposal_deposit = "10000 tokens"
voting_period = "24 hours"
```

## Faucet

Get test tokens for development:

- **Amount**: 1000 tokens per request
- **Cooldown**: 24 hours
- **Max Requests**: 10 per address

Visit: [https://faucet.testnet.modular.io](https://faucet.testnet.modular.io)

## Support

- **Discord**: [https://discord.gg/modular](https://discord.gg/modular)
- **Telegram**: [https://t.me/modular_blockchain](https://t.me/modular_blockchain)
- **GitHub**: [https://github.com/modular-blockchain](https://github.com/modular-blockchain)
- **Docs**: [https://docs.modular.io](https://docs.modular.io)

## Reporting Issues

Found a bug? Please report it:

1. **Critical Issues**: Discord #testnet-issues channel
2. **Non-Critical**: GitHub Issues
3. **Security**: security@modular.io

## Testnet Phases

### Phase 1: Internal Testing (Week 1-2)
- ✅ 3 genesis validators
- ✅ Basic functionality testing
- ✅ Faucet operational

### Phase 2: Public Beta (Week 3-4)
- 🔄 External validator onboarding
- 🔄 Developer testing
- 🔄 Load testing

### Phase 3: Stress Testing (Week 5-6)
- ⏳ Performance benchmarks
- ⏳ Chaos engineering
- ⏳ Security testing

### Phase 4: Mainnet Prep (Week 7-8)
- ⏳ Final audits
- ⏳ Bug fixes
- ⏳ Documentation updates

## Incentives

Testnet participants may be eligible for rewards:

- **Validators**: Uptime rewards
- **Developers**: Bug bounties
- **Community**: Contribution rewards

Details TBA.

## License

MIT
