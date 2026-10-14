const nacl = require('tweetnacl');
const { API_URL, fetchJson, sleep, passBanner, failAndExit } = require('./lib/helpers');
const { fundAddress } = require('./lib/faucet');
const { generateWallet, submitGovTx, waitForReceipt } = require('./lib/gov-tx');

// Staking lifecycle on a live devnet:
//   1. register → the validator joins consensus only at the next epoch, and
//      every header commits to the set that certifies the following block
//   2. unbond → stake waits out the unbonding period, then returns
//   3. double-sign evidence → anyone can submit two conflicting signed votes;
//      the validator is slashed and removed for good
const NBL = 10n ** 18n;
const toHex = (b) => Buffer.from(b).toString('hex');

async function staking() {
  return fetchJson(`${API_URL}/staking`);
}

async function balance(address) {
  return BigInt((await fetchJson(`${API_URL}/balance/${address}`)).balance || '0');
}

async function myValidator(address) {
  const res = await fetchJson(`${API_URL}/validators`);
  const list = Array.isArray(res) ? res : res.validators || [];
  return list.find((v) => v.address.toLowerCase() === address.toLowerCase());
}

async function waitFor(what, check, timeoutMs = 120000) {
  const end = Date.now() + timeoutMs;
  while (Date.now() < end) {
    const v = await check();
    if (v) return v;
    await sleep(1000);
  }
  throw new Error(`Timed out waiting for ${what}`);
}

async function ok(wallet, payload, label) {
  const receipt = await waitForReceipt(await submitGovTx(wallet, payload), 30);
  if (!receipt.success) throw new Error(`${label} failed: ${receipt.revert_reason}`);
}

/** A precommit signed exactly as common::consensus_types::Vote::signing_bytes. */
function precommit(wallet, height, blockByte) {
  const u64 = (v) => {
    const b = Buffer.alloc(8);
    b.writeBigUInt64LE(BigInt(v));
    return b;
  };
  const hash = Buffer.alloc(32, blockByte);
  const voter = Buffer.from(wallet.publicKey);
  const bytes = Buffer.concat([
    Buffer.from('NEBULA_VOTE_V1'),
    u64(height),
    u64(0),
    Buffer.of(2), // Step::Precommit
    Buffer.of(1),
    hash,
    u64(voter.length),
    voter,
  ]);
  return {
    height,
    round: 0,
    step: 'Precommit',
    block_hash: Array.from(hash),
    signature: Array.from(nacl.sign.detached(bytes, wallet.secretKey)),
    voter: Array.from(voter),
  };
}

async function test19() {
  console.log('🧪 Staking lifecycle: epochs, unbonding, slashing\n');
  try {
    const st0 = await staking();
    const p = st0.params;
    console.log(`   epoch ${p.epoch_blocks} blocks · unbonding ${p.unbonding_blocks} · jail ${p.jail_blocks}`);

    // 0. Header commitment: block H names the set that certifies H + 1,
    //    which is the epoch set in effect after H.
    {
      const s = await staking();
      const b = await fetchJson(`${API_URL}/block/${s.height}`);
      const committed = toHex(b.block.header.next_validators_hash);
      if (committed !== s.epoch_set_hash) {
        throw new Error(`header ${s.height} commits to ${committed}, epoch set is ${s.epoch_set_hash}`);
      }
      console.log(`0. ✅ Header #${s.height} commits to the next validator set (${committed.slice(0, 12)}…)`);
    }

    // 1. Register: in the registry at once, in consensus at the next epoch.
    const v = generateWallet();
    await fundAddress(v.address);
    const pk = toHex(v.publicKey);
    const submittedAt = (await staking()).height;
    await ok(v, { action: 'register_validator', stake: (3n * NBL).toString(), commission_rate: 5 }, 'register');
    if (!(await myValidator(v.address))) throw new Error('not in the registry after registering');
    const joined = await waitFor('epoch entry', async () => {
      const s = await staking();
      return s.epoch_set.some((e) => e.public_key === pk) ? s : null;
    });
    const boundary = Math.floor(joined.height / p.epoch_blocks) * p.epoch_blocks;
    // The snapshot at block b reads the registry as of b - 1, after the tx.
    if (boundary <= submittedAt) throw new Error(`joined at epoch ${boundary}, before the tx was sent at ${submittedAt}`);
    if (boundary % p.epoch_blocks !== 0) throw new Error(`joined at ${boundary}, not an epoch boundary`);
    console.log(`1. ✅ Registered after block ${submittedAt}; joined consensus at the epoch starting block ${boundary}`);

    // 2. Unbond 1 NBL: queued, then returned after the unbonding period.
    const before = await balance(v.address);
    await ok(v, { action: 'unbond', amount: NBL.toString() }, 'unbond');
    const entry = (await staking()).unbonding.find((u) => u.owner.toLowerCase() === v.address.toLowerCase());
    if (!entry) throw new Error('unbond did not create a queue entry');
    if (BigInt((await myValidator(v.address)).stake) !== 2n * NBL) throw new Error('stake not reduced to 2 NBL');
    console.log(`2. ✅ Unbonding 1 NBL until block ${entry.release_at}`);
    await waitFor('release', async () => (await staking()).height >= entry.release_at, 180000);
    await sleep(3000);
    const gained = (await balance(v.address)) - before;
    // The unbond tx itself paid a small fee.
    if (gained < NBL - NBL / 100n) throw new Error(`balance grew by ${gained}, expected ~1 NBL`);
    console.log('   ✅ 1 NBL back in the balance after the unbonding period');

    // 3. Double signing: two precommits for different blocks at one height.
    const reporter = generateWallet();
    await fundAddress(reporter.address);
    const height = (await staking()).height;
    const evidence = { first: precommit(v, height, 0xaa), second: precommit(v, height, 0xbb) };
    // A forged pair (second vote signed by someone else) is refused.
    const other = generateWallet();
    const forged = { first: evidence.first, second: { ...precommit(other, height, 0xbb), voter: evidence.first.voter } };
    const forgedReceipt = await waitForReceipt(await submitGovTx(reporter, { action: 'evidence', evidence: forged }), 30);
    if (forgedReceipt.success) throw new Error('forged evidence got a successful receipt');
    if ((await myValidator(v.address)).tombstoned) throw new Error('forged evidence slashed the validator');
    console.log(`3. ✅ Forged evidence refused: ${forgedReceipt.revert_reason}`);

    await ok(reporter, { action: 'evidence', evidence }, 'evidence');
    const slashed = await waitFor('slash', async () => {
      const mine = await myValidator(v.address);
      return mine && mine.tombstoned ? mine : null;
    }, 30000);
    const expectedStake = 2n * NBL - (2n * NBL * 500n) / 10000n;
    if (BigInt(slashed.stake) !== expectedStake) throw new Error(`stake ${slashed.stake}, expected ${expectedStake}`);
    console.log(`   ✅ Real evidence: slashed ${Number(BigInt(slashed.slashed)) / 1e18} NBL and tombstoned`);
    await waitFor('removal at the next epoch', async () => {
      const s = await staking();
      return s.epoch_set.every((e) => e.public_key !== pk);
    });
    console.log('   ✅ Removed from consensus at the next epoch');
    await ok(v, { action: 'unjail' }, 'unjail').then(
      () => { throw new Error('a tombstoned validator was allowed to unjail'); },
      () => console.log('   ✅ Unjail refused for a slashed validator'),
    );

    passBanner('Staking lifecycle');
  } catch (e) {
    failAndExit(e);
  }
}

test19();
