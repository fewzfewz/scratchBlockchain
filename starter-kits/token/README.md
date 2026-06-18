# Token Starter Kit

Deploy ERC-20 fungible tokens and ERC-721 non-fungible tokens (NFTs).

## Contracts

- `ERC20.sol` — Standard fungible token with mint, burn, approve, transferFrom
- `ERC721.sol` — Standard NFT with safe transfers, approvals, mint, burn

## Quick Start

```bash
# Deploy an ERC-20 token
npx tsx scripts/deploy-token.ts

# Deploy an ERC-721 NFT collection
npx tsx scripts/deploy-token.ts --type erc721
```

## SDK Usage

```typescript
import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

const client = new ModularClient(new HttpProvider("http://localhost:9933"));
const wallet = Wallet.generate();

// Mint tokens
const mintData = "0x40c10f19" + // mint(address,uint256)
  padAddress(wallet.address) +
  padUint256(ethers.parseUnits("1000", 18));
const tx = await wallet.signTransaction({ to: tokenAddress, data: mintData, gasLimit: 80000 });
await client.sendTransaction(tx);

// Transfer tokens
const transferData = "0xa9059cbb" + // transfer(address,uint256)
  padAddress(recipient) +
  padUint256(amount);
const tx2 = await wallet.signTransaction({ to: tokenAddress, data: transferData, gasLimit: 50000 });
await client.sendTransaction(tx2);
```

## Helper Functions

```typescript
function padAddress(addr: string): string {
  return "000000000000000000000000" + addr.slice(2);
}
function padUint256(val: bigint | number): string {
  return val.toString(16).padStart(64, "0");
}
```
