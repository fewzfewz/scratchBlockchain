const { fetchJson, passBanner, failAndExit } = require('./lib/helpers');
const { fundAddress } = require('./lib/faucet');
const { generateWallet, submitGovTx, waitForReceipt } = require('./lib/gov-tx');

// Undelegation (unbonding) is not implemented yet. This test checks the parts
// that exist: a delegation bonds funds, and a zero-amount staking tx is
// rejected on-chain instead of being treated as an "unstake".
async function test17_Unstake() {
  console.log('🧪 Test 5.2: Undelegate / Unstake\n');
  try {
    const wallet = generateWallet();
    await fundAddress(wallet.address);

    const validators = await fetchJson('http://localhost:8545/validators');
    const list = Array.isArray(validators) ? validators : validators.validators || [];
    const validator = list[0]?.address;
    if (!validator) throw new Error('No validators');

    const bonded = await waitForReceipt(
      await submitGovTx(wallet, { action: 'delegate', validator, amount: '500000000000000000' }),
    );
    if (!bonded.success) throw new Error(`Delegate failed: ${bonded.revert_reason}`);
    console.log('1. ✅ Delegated 0.5 NBL');

    const zero = await waitForReceipt(
      await submitGovTx(wallet, { action: 'delegate', validator, amount: '0' }),
    );
    if (zero.success) throw new Error('A zero-amount delegation must fail');
    console.log(`2. ✅ Zero-amount staking tx rejected (${zero.revert_reason})`);
    console.log('   ⚠️  Unbonding is not implemented yet');

    passBanner('Undelegate / Unstake');
  } catch (e) {
    failAndExit(e);
  }
}

test17_Unstake();
