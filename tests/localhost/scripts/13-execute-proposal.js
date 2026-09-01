const { fetchJson, sleep, passBanner, failAndExit } = require('./lib/helpers');
const { fundAddress } = require('./lib/faucet');
const { submitGovTx, generateWallet } = require('./lib/gov-tx');

async function waitForPassed(proposalId, endBlock) {
  for (let i = 0; i < 40; i++) {
    await sleep(3000);
    const status = await fetchJson('http://localhost:8545/status');
    const height = Number(status.height || 0);
    if (height < endBlock) continue;
    const detail = await fetchJson(`http://localhost:8545/proposal/${proposalId}`);
    const p = detail.proposal || detail;
    if ((p.status || '').toLowerCase() === 'passed') return p;
    if ((p.status || '').toLowerCase() === 'rejected') {
      throw new Error(`Proposal rejected after voting period (status=${p.status})`);
    }
  }
  throw new Error('Proposal did not reach Passed status in time');
}

async function test13_ExecuteProposal() {
  console.log('🧪 Test 4.3: Governance Execute (full lifecycle)\n');
  try {
    const wallet = generateWallet();
    await fundAddress(wallet.address, 1000);

    const title = `Execute E2E ${Date.now()}`;
    await submitGovTx(wallet, {
      action: 'propose',
      title,
      description: 'Propose → vote → wait → execute',
    });

    let proposal = null;
    for (let i = 0; i < 15; i++) {
      await sleep(3000);
      const gov = await fetchJson('http://localhost:8545/governance');
      proposal = (gov.proposals || []).find((x) => x.title === title);
      if (proposal) break;
    }
    if (!proposal) throw new Error('Proposal not found on-chain');

    await submitGovTx(wallet, {
      action: 'vote',
      proposal_id: proposal.id,
      choice: 'yes',
    });
    console.log(`1. ✅ Voted yes on #${proposal.id}`);

    const endBlock = proposal.end_block;
    const passed = await waitForPassed(proposal.id, endBlock);
    console.log(`2. ✅ Voting ended — status ${passed.status}`);

    await submitGovTx(wallet, { action: 'execute', proposal_id: proposal.id });
    console.log('3. ✅ Execute transaction submitted');

    for (let i = 0; i < 15; i++) {
      await sleep(3000);
      const detail = await fetchJson(`http://localhost:8545/proposal/${proposal.id}`);
      const p = detail.proposal || detail;
      if ((p.status || '').toLowerCase() === 'executed') {
        console.log(`4. ✅ Proposal #${proposal.id} executed on-chain`);
        passBanner('Governance Execute');
        return;
      }
    }
    throw new Error('Proposal not marked Executed after execute tx');
  } catch (e) {
    failAndExit(e);
  }
}

test13_ExecuteProposal();
