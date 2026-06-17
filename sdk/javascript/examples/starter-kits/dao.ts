/**
 * DAO Starter Kit — Governance Proposal Workflow
 *
 * Prerequisites:
 *   - A running Modular Blockchain node
 *   - An ERC-20 governance token + DAO contract deployed
 *
 * Run: npx tsx examples/starter-kits/dao.ts
 */

import { Wallet, HttpProvider, ModularClient } from "../../src";

const RPC_URL = "http://localhost:9933";
const GOVERNANCE_TOKEN = "0x0000000000000000000000000000000000000001";
const DAO_CONTRACT = "0x0000000000000000000000000000000000000004";

async function main() {
  const provider = new HttpProvider(RPC_URL);
  const client = new ModularClient(provider);

  const proposer = Wallet.generate();
  const voter1 = Wallet.generate();
  const voter2 = Wallet.generate();

  console.log("DAO Starter Kit");
  console.log("Proposer:", proposer.address);
  console.log("Voter 1: ", voter1.address);
  console.log("Voter 2: ", voter2.address);
  console.log("DAO Contract:", DAO_CONTRACT);

  // 1. Proposer creates a governance proposal
  const proposalDescription = "Increase block gas limit to 50M";
  const proposalData = "0x" +
    "0000000000000000000000000000000000000000000000000000000000000040" + // offset to description
    "00000000000000000000000000000000000000000000000000000000000000a0" + // offset to calldata
    "0000000000000000000000000000000000000000000000000000000000000020" + // description length (32)
    "496e63726561736520626c6f636b20676173206c696d697420746f2035304d" + // "Increase block gas limit to 50M"
    "0000000000000000000000000000000000000000000000000000000000000000"; // empty calldata

  const proposeTx = await proposer.signTransaction({
    to: DAO_CONTRACT,
    data: "0xda95691a" + // propose(string,bytes)
      "0000000000000000000000000000000000000000000000000000000000000040" +
      "0000000000000000000000000000000000000000000000000000000000000080" +
      "000000000000000000000000000000000000000000000000000000000000001a" +
      Buffer.from(proposalDescription).toString("hex") +
      "0000000000000000000000000000000000000000000000000000000000000000",
    gasLimit: 200000,
  });
  const proposeResult = await client.sendTransaction(proposeTx);
  console.log("\nProposal created:", proposeResult.hash);
  await client.waitForTransaction(proposeResult.hash);

  // 2. Voters cast their votes
  const voteForData1 = "0x15373e3d" + // castVote(uint256,bool)
    "0000000000000000000000000000000000000000000000000000000000000001" + // proposalId = 1
    "0000000000000000000000000000000000000000000000000000000000000001"; // support = true

  const vote1Tx = await voter1.signTransaction({
    to: DAO_CONTRACT,
    data: voteForData1,
    gasLimit: 50000,
  });
  const vote1Result = await client.sendTransaction(vote1Tx);
  console.log("Voter 1 voted:", vote1Result.hash);

  const vote2Tx = await voter2.signTransaction({
    to: DAO_CONTRACT,
    data: voteForData1,
    gasLimit: 50000,
  });
  const vote2Result = await client.sendTransaction(vote2Tx);
  console.log("Voter 2 voted:", vote2Result.hash);
  await client.waitForTransaction(vote2Result.hash);

  // 3. Anyone can execute after voting period ends
  const executeData = "0xfe0d94c1" + // execute(uint256)
    "0000000000000000000000000000000000000000000000000000000000000001";

  const executeTx = await proposer.signTransaction({
    to: DAO_CONTRACT,
    data: executeData,
    gasLimit: 100000,
  });
  const executeResult = await client.sendTransaction(executeTx);
  console.log("Proposal executed:", executeResult.hash);
  await client.waitForTransaction(executeResult.hash);

  console.log("\n✅ DAO governance flow complete");
}

main().catch(console.error);
