const { fetchJson, passBanner, failAndExit } = require('./lib/helpers');
const { fundAddress } = require('./lib/faucet');
const { generateWallet } = require('./lib/gov-tx');

async function test18_RegisterValidator() {
  console.log('🧪 Test 5.3: Validator Registration\n');
  try {
    const wallet = generateWallet();
    await fundAddress(wallet.address);

    const before = await fetchJson('http://localhost:8545/validators');
    const beforeList = Array.isArray(before) ? before : before.validators || [];
    const beforeCount = beforeList.length;

    await fetchJson('http://localhost:8545/validators/register', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        address: wallet.address,
        public_key: wallet.address,
        stake: '1000000000000000000',
        commission_rate: 5,
      }),
    });
    console.log('1. ✅ POST /validators/register succeeded');

    const after = await fetchJson('http://localhost:8545/validators');
    const afterList = Array.isArray(after) ? after : after.validators || [];
    if (afterList.length <= beforeCount) {
      throw new Error('Validator count did not increase');
    }
    console.log(`2. ✅ Validator set: ${beforeCount} → ${afterList.length}`);

    passBanner('Validator Registration');
  } catch (e) {
    failAndExit(e);
  }
}

test18_RegisterValidator();
