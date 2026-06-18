/**
 * Mint and List NFT
 *
 * Mints a new NFT and lists it for sale on the marketplace.
 *
 * Usage: npx tsx scripts/mint-and-list.ts
 */

import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

const RPC_URL = process.env.RPC_URL || "http://localhost:9933";
const NFT_CONTRACT = process.env.NFT_CONTRACT || "0x0000000000000000000000000000000000000001";
const MARKETPLACE = process.env.MARKETPLACE || "0x0000000000000000000000000000000000000002";

function padAddress(addr) {
  return "000000000000000000000000" + addr.slice(2);
}

function padUint256(val) {
  return BigInt(val).toString(16).padStart(64, "0");
}

async function main() {
  const provider = new HttpProvider(RPC_URL);
  const client = new ModularClient(provider);
  const wallet = Wallet.generate();

  console.log("Mint & List NFT");
  console.log("Wallet:", wallet.address);
  console.log("NFT Contract:", NFT_CONTRACT);
  console.log("Marketplace:", MARKETPLACE);

  // 1. Mint NFT
  const mintData = "0x40c10f19" + padAddress(wallet.address) + padUint256(1);
  const mintTx = await wallet.signTransaction({ to: NFT_CONTRACT, data: mintData, gasLimit: 80000 });
  const mintResult = await client.sendTransaction(mintTx);
  console.log("\nMinted NFT #1:", mintResult.hash);
  await client.waitForTransaction(mintResult.hash);

  // 2. Approve marketplace
  const approveData = "0x095ea7b3" + padAddress(MARKETPLACE) + padUint256(1);
  const approveTx = await wallet.signTransaction({ to: NFT_CONTRACT, data: approveData, gasLimit: 50000 });
  const approveResult = await client.sendTransaction(approveTx);
  console.log("Approved marketplace:", approveResult.hash);
  await client.waitForTransaction(approveResult.hash);

  // 3. List for sale (10 NBL)
  const listData = "0x..." + padAddress(NFT_CONTRACT) + padUint256(1) + padUint256(ethers.parseEther("10"));
  const listTx = await wallet.signTransaction({ to: MARKETPLACE, data: listData, gasLimit: 80000 });
  const listResult = await client.sendTransaction(listTx);
  console.log("Listed for 10 NBL:", listResult.hash);
  await client.waitForTransaction(listResult.hash);

  console.log("\n✅ NFT listed on marketplace");
}

main().catch(console.error);
