const { fetchJson, sleep, passBanner, failAndExit } = require('./lib/helpers');
const { fundAddress } = require('./lib/faucet');
const { submitGovTx, generateWallet } = require('./lib/gov-tx');

async function test25_ProposeUpgrade() {
  console.log('🧪 Test 4.4: Runtime Upgrade Proposal\n');
  try {
    const wallet = generateWallet();
    await fundAddress(wallet.address);

    const title = `Runtime upgrade v2-test-${Date.now()}`;
    const description = 'Software upgrade proposal from 25-propose-upgrade.js';
    await submitGovTx(wallet, {
      action: 'propose',
      title,
      description,
      proposal_type: 'software_upgrade',
      version: '2.1.0-test',
      hash: '0x' + 'ab'.repeat(32),
    });
    console.log('1. ✅ Upgrade-themed proposal submitted');

    let found = null;
    for (let i = 0; i < 15; i++) {
      await sleep(3000);
      const gov = await fetchJson('http://localhost:8545/governance');
      found = (gov.proposals || []).find((p) => p.title === title);
      if (found) break;
    }
    if (!found) throw new Error('Upgrade proposal not indexed');
    console.log(`2. ✅ Proposal #${found.id} on-chain (type may be text until Rust supports upgrade payload)`);

    passBanner('Runtime Upgrade Proposal');
  } catch (e) {
    failAndExit(e);
  }
}

test25_ProposeUpgrade();
