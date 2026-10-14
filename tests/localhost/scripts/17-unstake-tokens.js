const { fetchJson, passBanner, failAndExit } = require('./lib/helpers');
const { fundAddress } = require('./lib/faucet');
const { generateWallet, submitGovTx, waitForReceipt } = require('./lib/gov-tx');

// Delegate, then undelegate: the stake goes through the unbonding queue and
// returns to the balance after the unbonding period. A zero-amount staking
// tx is rejected on-chain.
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

    const before = BigInt((await fetchJson(`http://localhost:8545/balance/${wallet.address}`)).balance);
    const undone = await waitForReceipt(
      await submitGovTx(wallet, { action: 'undelegate', validator, amount: '500000000000000000' }),
    );
    if (!undone.success) throw new Error(`Undelegate failed: ${undone.revert_reason}`);
    const st = await fetchJson('http://localhost:8545/staking');
    const queued = st.unbonding.find((u) => u.owner.toLowerCase() === wallet.address.toLowerCase());
    if (!queued) throw new Error('Undelegation is not in the unbonding queue');
    console.log(`3. ✅ Undelegated 0.5 NBL; returns at block ${queued.release_at}`);
    for (let i = 0; i < 120; i++) {
      const s = await fetchJson('http://localhost:8545/status');
      if (s.height >= queued.release_at + 2) break;
      await new Promise((r) => setTimeout(r, 1000));
    }
    const after = BigInt((await fetchJson(`http://localhost:8545/balance/${wallet.address}`)).balance);
    if (after - before < 490000000000000000n) throw new Error(`Balance grew by ${after - before}, expected ~0.5 NBL`);
    console.log('4. ✅ 0.5 NBL returned after the unbonding period');

    passBanner('Undelegate / Unstake');
  } catch (e) {
    failAndExit(e);
  }
}

test17_Unstake();
