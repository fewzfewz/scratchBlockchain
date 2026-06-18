# DAO Starter Kit

Build a decentralized autonomous organization with token-based governance.

## Contracts

- `DAO.sol` — On-chain governance with:
  - Proposal creation (token holder gated)
  - Vote casting (weighted by token balance)
  - Automatic execution on success
  - Quorum and threshold checks

- `ERC20.sol` — Governance token (included)

## Quick Start

```bash
# 1. Deploy governance token
npx tsx scripts/deploy-governance-token.ts

# 2. Deploy DAO contract
npx tsx scripts/deploy-dao.ts

# 3. Create and vote on a proposal
npx tsx scripts/propose-and-vote.ts
```

## SDK Usage

```typescript
import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

// Create proposal
const proposeData = daoInterface.encodeFunctionData("propose", [
  "Increase block gas limit to 50M",
  "0x", // optional execution calldata
]);

const tx = await wallet.signTransaction({
  to: daoAddress,
  data: proposeData,
  gasLimit: 200000,
});
await client.sendTransaction(tx);

// Cast vote
const voteData = daoInterface.encodeFunctionData("castVote", [
  proposalId,
  true, // support
]);

const voteTx = await wallet.signTransaction({
  to: daoAddress,
  data: voteData,
  gasLimit: 50000,
});
await client.sendTransaction(voteTx);

// Execute after voting period
const executeData = daoInterface.encodeFunctionData("execute", [proposalId]);
const execTx = await wallet.signTransaction({
  to: daoAddress,
  data: executeData,
  gasLimit: 100000,
});
await client.sendTransaction(execTx);
```

## Governance Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| Voting Period | 100 blocks | Duration proposals stay open |
| Quorum | 1000 tokens | Minimum votes required |
| Proposal Threshold | 100 tokens | Minimum tokens to create proposal |
