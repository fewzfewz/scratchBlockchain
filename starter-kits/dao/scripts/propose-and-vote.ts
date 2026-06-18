/**
 * Create and Vote on a DAO Proposal
 *
 * Creates a proposal, casts votes, and executes it.
 *
 * Usage: npx tsx scripts/propose-and-vote.ts
 */

import { Wallet, HttpProvider, ModularClient } from "@modular-blockchain/sdk";

const RPC_URL = process.env.RPC_URL || "http://localhost:9933";
const DAO_ADDRESS = process.env.DAO_ADDRESS || "0x0000000000000000000000000000000000000004";
const GOV_TOKEN = process.env.GOV_TOKEN || "0x0000000000000000000000000000000000000001";

function padAddress(addr) {
  return "000000000000000000000000" + addr.slice(2);
}

function padUint256(val) {
  return BigInt(val).toString(16).padStart(64, "0");
}

async function main() {
  const provider = new HttpProvider(RPC_URL);
  const client = new ModularClient(provider);
  const proposer = Wallet.generate();
  const voter = Wallet.generate();

  console.log("DAO Proposal Workflow");
  console.log("Proposer:", proposer.address);
  console.log("Voter:   ", voter.address);

  // 1. Create proposal
  const desc = "Increase block gas limit to 50M";
  const descHex = Buffer.from(desc).toString("hex");
  const proposeData = "0xda95691a" + // propose(string,bytes)
    padUint256(64) + // offset to description
    padUint256(64 + 32 + descHex.length / 2) + // offset to calldata
    padUint256(descHex.length / 2) + // description length
    descHex.padEnd(64, "0") +
    padUint256(0); // empty calldata

  const proposeTx = await proposer.signTransaction({ to: DAO_ADDRESS, data: proposeData, gasLimit: 200000 });
  const proposeResult = await client.sendTransaction(proposeTx);
  console.log("\nProposal created:", proposeResult.hash);
  await client.waitForTransaction(proposeResult.hash);

  // 2. Vote for
  const voteData = "0x15373e3d" + padUint256(1) + padUint256(1); // castVote(1, yes)
  const voteTx = await voter.signTransaction({ to: DAO_ADDRESS, data: voteData, gasLimit: 50000 });
  const voteResult = await client.sendTransaction(voteTx);
  console.log("Vote cast:", voteResult.hash);
  await client.waitForTransaction(voteResult.hash);

  // 3. Execute (after voting period ends)
  const executeData = "0xfe0d94c1" + padUint256(1); // execute(1)
  const execTx = await proposer.signTransaction({ to: DAO_ADDRESS, data: executeData, gasLimit: 100000 });
  const execResult = await client.sendTransaction(execTx);
  console.log("Proposal executed:", execResult.hash);
  await client.waitForTransaction(execResult.hash);

  console.log("\n✅ DAO workflow complete");
}

main().catch(console.error);
