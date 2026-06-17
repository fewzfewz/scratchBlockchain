/**
 * DeFi Starter Kit — Simple AMM Liquidity Pool
 *
 * Prerequisites:
 *   - A running Modular Blockchain node (http://localhost:9933)
 *   - A funded wallet with test tokens
 *
 * Run: npx tsx examples/starter-kits/defi.ts
 */

import { Wallet, HttpProvider, ModularClient } from "../../src";

const RPC_URL = "http://localhost:9933";
const TOKEN_A = "0x0000000000000000000000000000000000000001";
const TOKEN_B = "0x0000000000000000000000000000000000000002";

async function main() {
  const provider = new HttpProvider(RPC_URL);
  const client = new ModularClient(provider);
  const wallet = Wallet.generate();

  console.log("DeFi Starter Kit");
  console.log("Wallet:", wallet.address);
  console.log("Token A:", TOKEN_A);
  console.log("Token B:", TOKEN_B);

  // 1. Check wallet balance
  const balance = await client.getBalance(wallet.address);
  console.log("\nNative Balance:", balance);

  // 2. Approve token spending
  const approveTx = await wallet.signTransaction({
    to: TOKEN_A,
    data: "0x095ea7b3000000000000000000000000" + wallet.address.slice(2) + "0000000000000000000000000000000000000000000000000000000000000000",
    gasLimit: 50000,
  });
  const approveResult = await client.sendTransaction(approveTx);
  console.log("Approval sent:", approveResult.hash);

  // 3. Add liquidity (simplified — real AMM would use a pair contract)
  const addLiqTx = await wallet.signTransaction({
    to: "0x0000000000000000000000000000000000000010", // Router contract
    data: "0xe8e33700", // addLiquidity selector
    gasLimit: 100000,
    value: "1000000000000000000", // 1 NBL
  });
  const liqResult = await client.sendTransaction(addLiqTx);
  console.log("Liquidity added:", liqResult.hash);

  // 4. Swap tokens (simplified)
  const swapTx = await wallet.signTransaction({
    to: "0x0000000000000000000000000000000000000010",
    data: "0x38ed1739", // swapExactTokensForTokens selector
    gasLimit: 80000,
  });
  const swapResult = await client.sendTransaction(swapTx);
  console.log("Swap completed:", swapResult.hash);

  // 5. Check LP token balance
  const lpBalance = await client.getBalance(wallet.address);
  console.log("LP Token Balance:", lpBalance);

  console.log("\n✅ DeFi flow complete");
}

main().catch(console.error);
