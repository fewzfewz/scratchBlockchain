/**
 * Add Liquidity
 *
 * Approves tokens and adds liquidity to an AMM pool.
 *
 * Usage: npx tsx scripts/add-liquidity.ts
 */

import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

const RPC_URL = process.env.RPC_URL || "http://localhost:9933";
const TOKEN_A = process.env.TOKEN_A || "0x0000000000000000000000000000000000000001";
const TOKEN_B = process.env.TOKEN_B || "0x0000000000000000000000000000000000000002";
const AMM_PAIR = process.env.AMM_PAIR || "0x0000000000000000000000000000000000000010";

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

  console.log("Adding Liquidity");
  console.log("Wallet:", wallet.address);

  // Approve Token A
  const approveA = "0x095ea7b3" + padAddress(AMM_PAIR) + padUint256(ethers.parseEther("1000"));
  const approveATx = await wallet.signTransaction({ to: TOKEN_A, data: approveA, gasLimit: 50000 });
  await client.sendTransaction(approveATx);

  // Approve Token B
  const approveB = "0x095ea7b3" + padAddress(AMM_PAIR) + padUint256(ethers.parseEther("500"));
  const approveBTx = await wallet.signTransaction({ to: TOKEN_B, data: approveB, gasLimit: 50000 });
  await client.sendTransaction(approveBTx);

  // Add liquidity
  const addLiqData = "0xe8e33700" + // addLiquidity(address,address,uint256,uint256,uint256,uint256,address,uint256)
    padAddress(TOKEN_A) +
    padAddress(TOKEN_B) +
    padUint256(ethers.parseEther("1000")) +
    padUint256(ethers.parseEther("500")) +
    padUint256(0) + // min LP tokens
    padUint256(0) + // min token B
    padAddress(wallet.address) +
    padUint256(Math.floor(Date.now() / 1000) + 3600);

  const addTx = await wallet.signTransaction({ to: AMM_PAIR, data: addLiqData, gasLimit: 200000 });
  const result = await client.sendTransaction(addTx);
  console.log("Liquidity added:", result.hash);
  await client.waitForTransaction(result.hash);

  console.log("\n✅ Liquidity added to pool");
  console.log("LP tokens credited to:", wallet.address);
}

main().catch(console.error);
