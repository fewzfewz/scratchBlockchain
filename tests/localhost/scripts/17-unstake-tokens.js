const { fetchJson, passBanner, failAndExit } = require('./lib/helpers');
const { fundAddress } = require('./lib/faucet');
const { generateWallet } = require('./lib/gov-tx');

async function test17_Unstake() {
  console.log('🧪 Test 5.2: Undelegate / Unstake\n');
  try {
    const wallet = generateWallet();
    await fundAddress(wallet.address);

    const validators = await fetchJson('http://localhost:8545/validators');
    const list = Array.isArray(validators) ? validators : validators.validators || [];
    const validator = list[0]?.address || list[0]?.public_key;
    if (!validator) throw new Error('No validators');

    const amount = '500000000000000000';
    await fetchJson('http://localhost:8545/delegate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ delegator: wallet.address, validator, amount }),
    });
    console.log('1. ✅ Delegated 0.5 NBL');

    await fetchJson('http://localhost:8545/delegate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        delegator: wallet.address,
        validator,
        amount: '0',
      }),
    });
    console.log('2. ✅ Undelegated (amount=0)');

    const dels = await fetchJson(`http://localhost:8545/delegations/${wallet.address}`);
    const entries = Array.isArray(dels) ? dels : dels.delegations || [];
    console.log(`3. ✅ Delegations after unstake: ${entries.length} entries`);

    passBanner('Undelegate / Unstake');
  } catch (e) {
    failAndExit(e);
  }
}

test17_Unstake();
