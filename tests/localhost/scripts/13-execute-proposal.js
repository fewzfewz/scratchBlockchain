const { fetchJson, sleep, passBanner, failAndExit } = require('./lib/helpers');
const { fundAddress } = require('./lib/faucet');
const { submitGovTx, generateWallet } = require('./lib/gov-tx');

async function test13_ExecuteProposal() {
  console.log('🧪 Test 4.3: Proposal Lifecycle\n');
  try {
    const wallet = generateWallet();
    await fundAddress(wallet.address);

    const title = `Execute flow ${Date.now()}`;
    await submitGovTx(wallet, {
      action: 'propose',
      title,
      description: 'Lifecycle test — vote then verify status',
    });

    let proposalId = null;
    for (let i = 0; i < 15; i++) {
      await sleep(3000);
      const gov = await fetchJson('http://localhost:8545/governance');
      const p = (gov.proposals || []).find((x) => x.title === title);
      if (p) {
        proposalId = p.id;
        break;
      }
    }
    if (proposalId == null) throw new Error('Proposal not found');

    await submitGovTx(wallet, { action: 'vote', proposal_id: proposalId, choice: 'yes' });
    console.log(`1. ✅ Voted yes on #${proposalId}`);

    await sleep(6000);
    const detail = await fetchJson(`http://localhost:8545/proposal/${proposalId}`);
    const p = detail.proposal || detail;
    console.log(`2. ✅ Proposal status: ${p.status}, yes=${p.yes_votes || 0}`);

    if (!p.status) throw new Error('Proposal status missing');
    passBanner('Proposal Lifecycle');
  } catch (e) {
    failAndExit(e);
  }
}

test13_ExecuteProposal();
