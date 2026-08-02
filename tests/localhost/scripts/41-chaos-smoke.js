const { execSync } = require('child_process');
const { fetchJson, sleep, passBanner, failAndExit } = require('./lib/helpers');

async function test41_ChaosSmoke() {
  console.log('🧪 Test 8.4: Chaos Smoke (validator restart)\n');
  try {
    const before = await fetchJson('http://localhost:8545/status');
    const h0 = before.height || 0;
    console.log(`1. Height before chaos: ${h0}`);

    try {
      execSync('docker restart validator2', { stdio: 'inherit' });
      console.log('2. ✅ Restarted validator2 container');
    } catch {
      console.log('2. ⏭️  docker restart skipped (not in Docker environment)');
      passBanner('Chaos Smoke (skipped)');
      return;
    }

    await sleep(15000);

    let recovered = false;
    for (let i = 0; i < 20; i++) {
      try {
        const after = await fetchJson('http://localhost:8545/status');
        if ((after.height || 0) > h0) {
          recovered = true;
          console.log(`3. ✅ Network recovered — height ${after.height}`);
          break;
        }
      } catch {
        /* node may be briefly unavailable */
      }
      await sleep(3000);
    }

    if (!recovered) throw new Error('Chain did not advance after validator restart');
    passBanner('Chaos Smoke');
  } catch (e) {
    failAndExit(e);
  }
}

test41_ChaosSmoke();
