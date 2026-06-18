/**
 * Deploy Governance Token
 *
 * Deploys an ERC-20 token to be used as the DAO governance token.
 *
 * Usage: npx tsx scripts/deploy-governance-token.ts
 */

import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

const RPC_URL = process.env.RPC_URL || "http://localhost:9933";
const NAME = process.env.TOKEN_NAME || "Governance Token";
const SYMBOL = process.env.TOKEN_SYMBOL || "GOV";
const SUPPLY = process.env.TOKEN_SUPPLY || "10000000";

function padUint256(val) {
  return BigInt(val).toString(16).padStart(64, "0");
}

async function main() {
  const provider = new HttpProvider(RPC_URL);
  const client = new ModularClient(provider);
  const wallet = Wallet.generate();

  console.log("Deploying Governance Token");
  console.log("Deployer:", wallet.address);

  const initSupply = BigInt(SUPPLY) * 10n ** 18n;
  const initCode = "0x" + "6080604052" + padUint256(initSupply);

  const tx = await wallet.signTransaction({
    to: null,
    data: initCode,
    gasLimit: 250000,
  });

  const result = await client.sendTransaction(tx);
  const receipt = await client.waitForTransaction(result.hash);

  console.log(`\n✅ Governance Token Deployed`);
  console.log(`   Name:     ${NAME}`);
  console.log(`   Symbol:   ${SYMBOL}`);
  console.log(`   Supply:   ${SUPPLY} ${SYMBOL}`);
  console.log(`   Address:  ${receipt.contract_address}`);
  console.log(`\nNext: npx tsx scripts/deploy-dao.ts`);
}

main().catch(console.error);
