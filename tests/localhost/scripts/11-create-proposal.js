const { fetchJson, sleep, passBanner, failAndExit } = require('./lib/helpers');
const { fundAddress } = require('./lib/faucet');
const { submitGovTx, generateWallet } = require('./lib/gov-tx');

async function test11_CreateProposal() {
  console.log('🧪 Test 4.1: Create Governance Proposal\n');
  try {
    const wallet = generateWallet();
    console.log(`1. Wallet: ${wallet.address}`);
    await fundAddress(wallet.address);
    console.log('   ✅ Funded via faucet');

    const before = await fetchJson('http://localhost:8545/governance');
    const nextId = (before.next_proposal_id || before.proposals?.length + 1 || 1);
    console.log(`2. Next proposal id (expected): ${nextId}`);

    const title = `Integration proposal ${Date.now()}`;
    const txHash = await submitGovTx(wallet, {
      action: 'propose',
      title,
      description: 'Created by 11-create-proposal.js',
    });
    console.log(`3. Submitted propose tx: ${txHash.slice(0, 16)}...`);

    let found = null;
    for (let i = 0; i < 20; i++) {
      await sleep(3000);
      const gov = await fetchJson('http://localhost:8545/governance');
      found = (gov.proposals || []).find((p) => p.title === title);
      if (found) break;
    }

    if (!found) throw new Error('Proposal not found on-chain after waiting');
    console.log(`4. ✅ Proposal #${found.id} status=${found.status}`);

    passBanner('Create Governance Proposal');
  } catch (e) {
    failAndExit(e);
  }
}

test11_CreateProposal();
