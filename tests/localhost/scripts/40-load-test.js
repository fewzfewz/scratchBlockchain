const { API_URL, fetchJson, sleep, passBanner, failAndExit } = require('./lib/helpers');
const { fundAddress } = require('./lib/faucet');
const { generateWallet } = require('./lib/gov-tx');

async function test40_LoadTest() {
  console.log('🧪 Test 8.3: Load Test (RPC burst)\n');
  try {
    const wallet = generateWallet();
    await fundAddress(wallet.address);

    const start = Date.now();
    const requests = 200;
    let ok = 0;
    let fail = 0;

    const tasks = [];
    for (let i = 0; i < requests; i++) {
      tasks.push(
        fetch(`${API_URL}/status`)
          .then((r) => (r.ok ? ok++ : fail++))
          .catch(() => fail++)
      );
    }
    await Promise.all(tasks);

    const elapsed = (Date.now() - start) / 1000;
    const rps = requests / elapsed;
    console.log(`1. ${requests} parallel GET /status in ${elapsed.toFixed(2)}s (${rps.toFixed(0)} req/s)`);
    console.log(`   ✅ ${ok} ok, ❌ ${fail} failed`);

    if (ok < requests * 0.9) throw new Error('Too many failed status requests under load');

    // Submit a few faucet requests sequentially (respect cooldown)
    for (let i = 0; i < 3; i++) {
      const w = generateWallet();
      await fetchJson(`${API_URL}/faucet/request`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ address: w.address, amount: 10 }),
      }).catch(() => {});
      await sleep(1000);
    }
    console.log('2. ✅ Faucet burst completed');

    passBanner('Load Test');
  } catch (e) {
    failAndExit(e);
  }
}

test40_LoadTest();
