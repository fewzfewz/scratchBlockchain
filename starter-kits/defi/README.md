# DeFi Starter Kit

Build a decentralized exchange (DEX) with automated market maker (AMM) liquidity pools.

## Contracts

- `SimpleAMM.sol` — Constant product AMM (x * y = k) with:
  - Add / remove liquidity
  - Swap tokens with price impact
  - LP token minting/burning
  - Slippage protection

## Quick Start

```bash
# 1. Deploy two ERC-20 tokens
npx tsx ../token/scripts/deploy-token.ts --name "Token A" --symbol "TKNA" --supply 1000000
npx tsx ../token/scripts/deploy-token.ts --name "Token B" --symbol "TKNB" --supply 1000000

# 2. Deploy the AMM pair
npx tsx scripts/deploy-amm.ts

# 3. Add liquidity and swap
npx tsx scripts/add-liquidity.ts
```

## SDK Usage

```typescript
import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

// Add liquidity
const addLiqData = ammInterface.encodeFunctionData("addLiquidity", [
  tokenA,
  tokenB,
  ethers.parseEther("1000"),
  ethers.parseEther("500"),
  0, // min LP tokens
  deadline,
]);

const tx = await wallet.signTransaction({
  to: ammAddress,
  data: addLiqData,
  gasLimit: 150000,
});
await client.sendTransaction(tx);

// Swap
const swapData = ammInterface.encodeFunctionData("swap", [
  tokenIn,
  tokenOut,
  amountIn,
  minAmountOut,
  deadline,
]);

const swapTx = await wallet.signTransaction({
  to: ammAddress,
  data: swapData,
  gasLimit: 100000,
});
await client.sendTransaction(swapTx);
```

## Key Concepts

- **Constant Product**: `reserveA * reserveB = k`
- **LP Tokens**: Represent pool share, earn trading fees
- **Slippage**: Set `minAmountOut` to protect against price movement
- **Fees**: 0.3% trading fee (configurable)
