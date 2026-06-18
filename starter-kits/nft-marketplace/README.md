# NFT Marketplace Starter Kit

Build a full NFT marketplace with minting, listing, buying, and selling.

## Contracts

- `NFTMarketplace.sol` — Marketplace contract with:
  - List NFTs for sale (fixed price)
  - Buy NFTs (pay in native token)
  - Cancel listings
  - Royalty support (creator fee)
  - Platform fee

- `ERC721.sol` — Standard NFT contract (included for reference)

## Quick Start

```bash
# 1. Deploy the NFT collection
npx tsx scripts/deploy-collection.ts

# 2. Deploy the marketplace
npx tsx scripts/deploy-marketplace.ts

# 3. Mint and list an NFT
npx tsx scripts/mint-and-list.ts
```

## SDK Usage

```typescript
import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

// List an NFT for sale
const listData = marketplaceInterface.encodeFunctionData("listItem", [
  nftAddress,
  tokenId,
  ethers.parseEther("10"), // price in NBL
]);

const tx = await wallet.signTransaction({
  to: marketplaceAddress,
  data: listData,
  gasLimit: 80000,
});
await client.sendTransaction(tx);

// Buy an NFT
const buyData = marketplaceInterface.encodeFunctionData("buyItem", [
  nftAddress,
  tokenId,
]);

const buyTx = await wallet.signTransaction({
  to: marketplaceAddress,
  data: buyData,
  gasLimit: 80000,
  value: ethers.parseEther("10"),
});
await client.sendTransaction(buyTx);
```
