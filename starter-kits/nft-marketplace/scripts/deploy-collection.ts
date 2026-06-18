/**
 * Deploy NFT Collection
 *
 * Deploys an ERC-721 contract for use with the NFT marketplace.
 *
 * Usage: npx tsx scripts/deploy-collection.ts
 */

import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

const RPC_URL = process.env.RPC_URL || "http://localhost:9933";

async function main() {
  const provider = new HttpProvider(RPC_URL);
  const client = new ModularClient(provider);
  const wallet = Wallet.generate();

  console.log("Deploying NFT Collection...");
  console.log("Deployer:", wallet.address);

  const initCode = "0x6080604052"; // minimal ERC-721 init (placeholder — use full bytecode in production)

  const tx = await wallet.signTransaction({
    to: null,
    data: initCode,
    gasLimit: 300000,
  });

  const result = await client.sendTransaction(tx);
  const receipt = await client.waitForTransaction(result.hash);

  console.log(`\n✅ NFT Collection Deployed`);
  console.log(`   Address:  ${receipt.contract_address}`);
  console.log(`   Tx Hash:  ${result.hash}`);

  console.log(`\nNext: npx tsx scripts/deploy-marketpoint.ts`);
}

main().catch(console.error);
