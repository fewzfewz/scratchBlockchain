const { fetchJson, passBanner, failAndExit } = require('./lib/helpers');
const { fundAddress } = require('./lib/faucet');
const { generateWallet, submitGovTx, waitForReceipt } = require('./lib/gov-tx');

async function test16_StakeTokens() {
  console.log('🧪 Test 5.1: Stake / Delegate Tokens\n');
  try {
    const wallet = generateWallet();
    console.log(`1. Delegator: ${wallet.address}`);
    const funded = await fundAddress(wallet.address);
    console.log('   ✅ Funded delegator');

    const validators = await fetchJson('http://localhost:8545/validators');
    const list = Array.isArray(validators) ? validators : validators.validators || [];
    if (list.length === 0) throw new Error('No validators available');
    const validator = list[0].address;
    console.log(`2. Validator: ${validator}`);

    // A signed staking transaction that bonds `amount` from the delegator.
    const amount = '1000000000000000000';
    const hash = await submitGovTx(wallet, { action: 'delegate', validator, amount });
    const receipt = await waitForReceipt(hash);
    if (!receipt.success) throw new Error(`Delegate tx failed: ${receipt.revert_reason}`);
    console.log('3. ✅ Delegate transaction included');

    const dels = await fetchJson(`http://localhost:8545/delegations/${wallet.address}`);
    const entries = Array.isArray(dels) ? dels : dels.delegations || [];
    if (!entries.some((d) => String(d.amount) === amount)) {
      throw new Error('Delegation not visible via GET /delegations');
    }
    const after = await fetchJson(`http://localhost:8545/balance/${wallet.address}`);
    if (BigInt(after.balance) > BigInt(funded.balance) - BigInt(amount)) {
      throw new Error('Delegated amount was not bonded from the balance');
    }
    console.log(`4. ✅ Delegation recorded and ${amount} wei bonded`);

    passBanner('Stake / Delegate Tokens');
  } catch (e) {
    failAndExit(e);
  }
}

test16_StakeTokens();
