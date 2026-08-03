const { fetchJson, sleep, passBanner, failAndExit } = require('./lib/helpers');
const { fundAddress } = require('./lib/faucet');
const { submitGovTx, generateWallet } = require('./lib/gov-tx');

async function test12_VoteProposal() {
  console.log('🧪 Test 4.2: Vote on Governance Proposal\n');
  try {
    const wallet = generateWallet();
    await fundAddress(wallet.address);

    const title = `Vote test proposal ${Date.now()}`;
    await submitGovTx(wallet, {
      action: 'propose',
      title,
      description: 'Proposal for vote test',
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

    await submitGovTx(wallet, {
      action: 'vote',
      proposal_id: proposalId,
      choice: 'yes',
    });
    console.log(`1. ✅ Vote submitted for proposal #${proposalId}`);

    for (let i = 0; i < 10; i++) {
      await sleep(3000);
      const gov = await fetchJson(`http://localhost:8545/proposal/${proposalId}`);
      const p = gov.proposal || gov;
      if (p && Number(p.yes_votes || 0) > 0) {
        console.log(`2. ✅ Yes votes recorded: ${p.yes_votes}`);
        passBanner('Vote on Proposal');
        return;
      }
    }
    throw new Error('Vote not reflected on-chain');
  } catch (e) {
    failAndExit(e);
  }
}

test12_VoteProposal();
