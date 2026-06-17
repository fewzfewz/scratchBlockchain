/**
 * NFT Starter Kit — Mint and Transfer NFTs
 *
 * Prerequisites:
 *   - A running Modular Blockchain node
 *   - An ERC-721 contract deployed (use CLI: `modular deploy ERC721 --args "MyNFT,MNFT"`)
 *
 * Run: npx tsx examples/starter-kits/nft.ts
 */

import { Wallet, HttpProvider, ModularClient } from "../../src";

const RPC_URL = "http://localhost:9933";
const NFT_CONTRACT = "0x0000000000000000000000000000000000000003";

async function main() {
  const provider = new HttpProvider(RPC_URL);
  const client = new ModularClient(provider);

  // Create a creator wallet and a buyer wallet
  const creator = Wallet.generate();
  const buyer = Wallet.generate();

  console.log("NFT Starter Kit");
  console.log("Creator:", creator.address);
  console.log("Buyer:  ", buyer.address);
  console.log("NFT Contract:", NFT_CONTRACT);

  // 1. Creator mints a new NFT (token ID = 1)
  const mintData = "0x40c10f19" + // mint(address,uint256)
    "000000000000000000000000" + creator.address.slice(2) +
    "0000000000000000000000000000000000000000000000000000000000000001";

  const mintTx = await creator.signTransaction({
    to: NFT_CONTRACT,
    data: mintData,
    gasLimit: 80000,
  });
  const mintResult = await client.sendTransaction(mintTx);
  console.log("\nMinted NFT #1:", mintResult.hash);

  // 2. Get the transaction receipt to confirm
  const receipt = await client.waitForTransaction(mintResult.hash);
  console.log("Mint confirmed in block:", receipt.block_height);

  // 3. Creator approves the buyer to transfer the NFT
  const approveData = "0x095ea7b3" + // approve(address,uint256)
    "000000000000000000000000" + buyer.address.slice(2) +
    "0000000000000000000000000000000000000000000000000000000000000001";

  const approveTx = await creator.signTransaction({
    to: NFT_CONTRACT,
    data: approveData,
    gasLimit: 50000,
  });
  const approveResult = await client.sendTransaction(approveTx);
  console.log("Approval set:", approveResult.hash);
  await client.waitForTransaction(approveResult.hash);

  // 4. Creator lists the NFT for sale (approve + marketplace list)
  //    In production, this would call a marketplace contract
  console.log("\nNFT #1 ready for transfer");

  // 5. Transfer NFT from creator to buyer
  const transferData = "0x23b872dd" + // transferFrom(address,address,uint256)
    "000000000000000000000000" + creator.address.slice(2) +
    "000000000000000000000000" + buyer.address.slice(2) +
    "0000000000000000000000000000000000000000000000000000000000000001";

  const transferTx = await creator.signTransaction({
    to: NFT_CONTRACT,
    data: transferData,
    gasLimit: 60000,
  });
  const transferResult = await client.sendTransaction(transferTx);
  console.log("Transferred NFT #1 to buyer:", transferResult.hash);
  await client.waitForTransaction(transferResult.hash);

  console.log("\n✅ NFT marketplace flow complete");
}

main().catch(console.error);
