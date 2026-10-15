const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const nacl = require('tweetnacl');
const { API_URL, fetchJson, sleep, passBanner, failAndExit } = require('./lib/helpers');
const { submitGovTx, waitForReceipt } = require('./lib/gov-tx');
const { addressFromPublicKey } = require('./lib/tx-sign');

// Jailing on a live devnet: stop validator4, watch the chain jail it for
// missed proposer turns and drop it at the next epoch (so its turns stop
// costing timeouts), then restart it and have it unjail itself.
const CONFIGS = path.resolve(__dirname, '../../../deployment/local/configs');
const COMPOSE = path.resolve(__dirname, '../../../deployment/local');

function validatorWallet(name) {
  const { secret_key: seed } = JSON.parse(fs.readFileSync(path.join(CONFIGS, `${name}_key.json`), 'utf8'));
  const kp = nacl.sign.keyPair.fromSeed(Buffer.from(seed, 'hex'));
  return { publicKey: kp.publicKey, secretKey: kp.secretKey, address: addressFromPublicKey(kp.publicKey) };
}

const compose = (args) => execSync(`docker-compose ${args}`, { cwd: COMPOSE, stdio: 'pipe' });

/** Registry entry by public key: genesis validators carry placeholder
 *  addresses that are not derived from their keys. */
async function entry(publicKeyHex) {
  const res = await fetchJson(`${API_URL}/validators`);
  return (res.validators || res).find((v) => v.public_key.toLowerCase() === publicKeyHex);
}

async function waitFor(what, check, timeoutMs) {
  const end = Date.now() + timeoutMs;
  while (Date.now() < end) {
    const v = await check();
    if (v) return v;
    await sleep(1000);
  }
  throw new Error(`Timed out waiting for ${what}`);
}

/** Blocks per second over `ms`. */
async function blockRate(ms) {
  const a = (await fetchJson(`${API_URL}/status`)).height;
  await sleep(ms);
  const b = (await fetchJson(`${API_URL}/status`)).height;
  return (b - a) / (ms / 1000);
}

async function test20() {
  console.log('🧪 Jailing an offline validator\n');
  const v4 = validatorWallet('validator4');
  const pk = Buffer.from(v4.publicKey).toString('hex');
  try {
    // Stopping validator4 must leave more than 2/3 of the epoch set's stake
    // online, or the chain halts (correctly) instead of jailing it.
    const set = (await fetchJson(`${API_URL}/staking`)).epoch_set;
    const total = set.reduce((a, e) => a + BigInt(e.voting_power), 0n);
    const mine = set.filter((e) => e.public_key === pk).reduce((a, e) => a + BigInt(e.voting_power), 0n);
    if (mine === 0n) throw new Error('validator4 is not in the epoch set');
    if ((total - mine) * 3n <= total * 2n) throw new Error('the rest of the set holds 2/3 or less without validator4; unjail the others first');

    const healthy = await blockRate(10000);
    console.log(`0. Block rate with all validators up: ${healthy.toFixed(2)}/s`);

    compose('stop validator4');
    console.log('1. Stopped validator4');
    const degraded = await blockRate(10000);
    console.log(`   Block rate while its turns time out: ${degraded.toFixed(2)}/s`);

    const jailed = await waitFor('jail', async () => {
      const e = await entry(pk);
      return e && e.jailed ? e : null;
    }, 600000);
    console.log(`2. ✅ Jailed after ${jailed.missed_in_row} missed turns (until block ${jailed.jailed_until})`);
    await waitFor('removal from the epoch set', async () => {
      const s = await fetchJson(`${API_URL}/staking`);
      return s.epoch_set.every((e) => e.public_key !== pk);
    }, 60000);
    const recovered = await blockRate(10000);
    console.log(`   ✅ Out of the epoch set; block rate back to ${recovered.toFixed(2)}/s`);
    if (recovered < degraded) console.log('   ⚠️  block rate did not improve after removal');

    compose('start validator4');
    console.log('3. Restarted validator4');
    await waitFor('validator4 to sync', async () => {
      try {
        const [mine, ref] = await Promise.all([
          fetchJson('http://localhost:8550/status'),
          fetchJson(`${API_URL}/status`),
        ]);
        return ref.height - mine.height < 5;
      } catch {
        return false;
      }
    }, 180000);
    await waitFor('jail period to pass', async () => (await fetchJson(`${API_URL}/status`)).height >= jailed.jailed_until, 120000);
    const receipt = await waitForReceipt(await submitGovTx(v4, { action: 'unjail' }), 30);
    if (!receipt.success) throw new Error(`unjail failed: ${receipt.revert_reason}`);
    await waitFor('unjail to apply', async () => !(await entry(pk)).jailed, 30000);
    await waitFor('return to the epoch set', async () => {
      const s = await fetchJson(`${API_URL}/staking`);
      return s.epoch_set.some((e) => e.public_key === pk);
    }, 60000);
    console.log('4. ✅ Unjailed and back in the validator set at the next epoch');

    passBanner('Jail offline validator');
  } catch (e) {
    try { compose('start validator4'); } catch { /* already running */ }
    failAndExit(e);
  }
}

test20();
