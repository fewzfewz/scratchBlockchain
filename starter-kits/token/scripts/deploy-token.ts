/**
 * Token Deployment Script
 *
 * Deploys an ERC-20 or ERC-721 contract to the Modular Blockchain.
 *
 * Usage:
 *   npx tsx scripts/deploy-token.ts
 *   npx tsx scripts/deploy-token.ts --type erc721 --name "MyNFT" --symbol "MNFT"
 */

import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

const RPC_URL = process.env.RPC_URL || "http://localhost:9933";
const args = parseArgs();

function parseArgs() {
  const type = process.argv.find((a) => a.startsWith("--type="))?.split("=")[1] || "erc20";
  const name = process.argv.find((a) => a.startsWith("--name="))?.split("=")[1] || (type === "erc20" ? "MyToken" : "MyNFT");
  const symbol = process.argv.find((a) => a.startsWith("--symbol="))?.split("=")[1] || (type === "erc20" ? "MTK" : "MNFT");
  const supply = process.argv.find((a) => a.startsWith("--supply="))?.split("=")[1] || "1000000";
  return { type, name, symbol, supply };
}

function padAddress(addr) {
  return "000000000000000000000000" + addr.slice(2);
}

function padUint256(val) {
  return BigInt(val).toString(16).padStart(64, "0");
}

async function deployERC20(client, wallet, name, symbol, supply) {
  const initSupply = BigInt(supply) * 10n ** 18n;
  const constructorData =
    padAddress(wallet.address) +
    padUint256(initSupply);

  const initCode =
    "0x" +
    "608060405260128060006101000a81548160ff021916908360ff160217905550" + // decimals = 18
    constructorData;

  const tx = await wallet.signTransaction({
    to: null,
    data: initCode,
    gasLimit: 200000,
  });

  const result = await client.sendTransaction(tx);
  const receipt = await client.waitForTransaction(result.hash);
  const address = receipt.contract_address;

  console.log(`\n✅ ERC-20 Deployed`);
  console.log(`   Name:     ${name}`);
  console.log(`   Symbol:   ${symbol}`);
  console.log(`   Supply:   ${supply} ${symbol}`);
  console.log(`   Address:  ${address}`);
  console.log(`   Tx Hash:  ${result.hash}`);

  return address;
}

async function deployERC721(client, wallet, name, symbol) {
  const initCode =
    "0x" +
    "60806040526040518060400160405280600581526020017f" +
    Buffer.from(name).toString("hex") +
    "0000000000000000000000000000000000000000000000000000000000";

  const tx = await wallet.signTransaction({
    to: null,
    data: initCode,
    gasLimit: 250000,
  });

  const result = await client.sendTransaction(tx);
  const receipt = await client.waitForTransaction(result.hash);
  const address = receipt.contract_address;

  console.log(`\n✅ ERC-721 Deployed`);
  console.log(`   Name:     ${name}`);
  console.log(`   Symbol:   ${symbol}`);
  console.log(`   Address:  ${address}`);
  console.log(`   Tx Hash:  ${result.hash}`);

  return address;
}

async function main() {
  const provider = new HttpProvider(RPC_URL);
  const client = new ModularClient(provider);
  const wallet = Wallet.generate();

  console.log(`\nToken Deployment`);
  console.log(`   Network:  ${RPC_URL}`);
  console.log(`   Deployer: ${wallet.address}`);
  console.log(`   Type:     ${args.type.toUpperCase()}`);

  if (args.type === "erc20") {
    await deployERC20(client, wallet, args.name, args.symbol, args.supply);
  } else if (args.type === "erc721") {
    await deployERC721(client, wallet, args.name, args.symbol);
  } else {
    console.error(`Unknown type: ${args.type}. Use "erc20" or "erc721".`);
    process.exit(1);
  }
}

main().catch(console.error);
