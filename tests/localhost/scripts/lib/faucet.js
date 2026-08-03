const { API_URL, fetchJson, sleep } = require('./helpers');

async function fundAddress(address, amount = 100) {
  await fetchJson(`${API_URL}/faucet/request`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ address, amount }),
  });
  for (let i = 0; i < 15; i++) {
    await sleep(2000);
    const bal = await fetchJson(`${API_URL}/balance/${address}`);
    if (BigInt(bal.balance || '0') > 0n) return bal;
  }
  throw new Error(`Faucet did not credit ${address} in time`);
}

module.exports = { fundAddress };
