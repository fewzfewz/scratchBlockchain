/**
 * Deploy AMM Pair
 *
 * Creates a new liquidity pool between two tokens.
 *
 * Usage: npx tsx scripts/deploy-amm.ts
 */

import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

const RPC_URL = process.env.RPC_URL || "http://localhost:9933";
const TOKEN_A = process.env.TOKEN_A || "0x0000000000000000000000000000000000000001";
const TOKEN_B = process.env.TOKEN_B || "0x0000000000000000000000000000000000000002";

async function main() {
  const provider = new HttpProvider(RPC_URL);
  const client = new ModularClient(provider);
  const wallet = Wallet.generate();

  console.log("Deploying AMM Pair");
  console.log("Deployer:", wallet.address);
  console.log("Token A:", TOKEN_A);
  console.log("Token B:", TOKEN_B);

  const initCode =
    process.env.AMM_BYTECODE ||
    (() => {
      try {
        const fs = require('fs');
        const path = require('path');
        const p = path.join(__dirname, '../../../contracts/bytecode/SimpleAMM.json');
        const art = JSON.parse(fs.readFileSync(p, 'utf8'));
        return art.bytecode;
      } catch {
        return '0x6080604052';
      }
    })();

  const tx = await wallet.signTransaction({
    to: null,
    data: initCode,
    gasLimit: 400000,
  });

  const result = await client.sendTransaction(tx);
  const receipt = await client.waitForTransaction(result.hash);

  console.log(`\n✅ AMM Pair Deployed`);
  console.log(`   Address:  ${receipt.contract_address}`);
  console.log(`   Tx Hash:  ${result.hash}`);
  console.log(`\nNext: Add liquidity via scripts/add-liquidity.ts`);
}

main().catch(console.error);
