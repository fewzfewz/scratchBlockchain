# Modular Blockchain — Smart Contract Templates

Production-ready Solidity contracts for the Modular Blockchain EVM runtime.

## Available Contracts

| Contract | File | Description | Standard |
|----------|------|-------------|----------|
| **ERC-20 Token** | [`ERC20.sol`](./ERC20.sol) | Fungible token with mint/burn/approve/transferFrom | ERC-20 |
| **ERC-721 NFT** | [`ERC721.sol`](./ERC721.sol) | Non-fungible token with safe transfers | ERC-721 |
| **ERC-1155 Multi-token** | [`ERC1155.sol`](./ERC1155.sol) | Batch transfers, multi-token management | ERC-1155 |
| **DAO Governance** | [`DAO.sol`](./DAO.sol) | On-chain proposal/vote/execute | Custom |
| **Simple AMM** | [`SimpleAMM.sol`](./SimpleAMM.sol) | Constant product DEX (x * y = k) | Uniswap V2 |
| **NFT Marketplace** | [`NFTMarketplace.sol`](./NFTMarketplace.sol) | List, buy, cancel NFTs with royalties | Custom |

## Compilation

```bash
# Using solc directly
solc --bin --abi contracts/ERC20.sol -o build/

# Using Hardhat (recommended)
npx hardhat compile
```

## Deployment

Use the SDK CLI or the starter kit deployment scripts:

```bash
# Via CLI
modular deploy ERC20 --args "MyToken,MTK,1000000"

# Via TypeScript script
npx tsx starter-kits/token/scripts/deploy-token.ts
```

## Contract Details

### ERC20.sol
Standard fungible token with 18 decimals. Constructor accepts `name`, `symbol`, and `initialSupply`. Includes `mint`, `burn` (internal), `transfer`, `approve`, `transferFrom`.

### ERC721.sol
Standard NFT with `safeTransferFrom`, `approve`, `setApprovalForAll`. Includes `mint` and `burn` (public).

### DAO.sol
Token-weighted governance. Proposers need `proposalThreshold` tokens. Voting uses `votingPeriod` blocks. Executes on quorum + majority.

### SimpleAMM.sol
Constant product AMM (`reserve0 * reserve1 = k`). Functions: `addLiquidity`, `removeLiquidity`, `swap`, `getAmountOut`. 0.3% fee. LP token minted on add.

### NFTMarketplace.sol
Fixed-price NFT marketplace. `listItem`, `buyItem`, `cancelListing`, `updateListing`. Supports creator royalties and platform fee.
