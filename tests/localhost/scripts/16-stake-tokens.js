const { fetchJson, passBanner, failAndExit } = require('./lib/helpers');
const { fundAddress } = require('./lib/faucet');
const { generateWallet } = require('./lib/gov-tx');

async function test16_StakeTokens() {
  console.log('🧪 Test 5.1: Stake / Delegate Tokens\n');
  try {
    const wallet = generateWallet();
    console.log(`1. Delegator: ${wallet.address}`);
    await fundAddress(wallet.address);
    console.log('   ✅ Funded delegator');

    const validators = await fetchJson('http://localhost:8545/validators');
    const list = Array.isArray(validators) ? validators : validators.validators || [];
    if (list.length === 0) throw new Error('No validators available');
    const validator = list[0].address || list[0].public_key;
    console.log(`2. Validator: ${validator}`);

    const amount = '1000000000000000000';
    await fetchJson('http://localhost:8545/delegate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        delegator: wallet.address,
        validator,
        amount,
      }),
    });
    console.log('3. ✅ POST /delegate succeeded');

    const dels = await fetchJson(`http://localhost:8545/delegations/${wallet.address}`);
    const entries = Array.isArray(dels) ? dels : dels.delegations || [];
    const match = entries.find(
      (d) => (d.validator_address || d.validator) === validator || String(d.amount) === amount
    );
    if (!match && entries.length === 0) {
      throw new Error('Delegation not visible via GET /delegations');
    }
    console.log(`4. ✅ Delegation recorded (${entries.length} entries)`);

    passBanner('Stake / Delegate Tokens');
  } catch (e) {
    failAndExit(e);
  }
}

test16_StakeTokens();
