#!/usr/bin/env node
/**
 * Nebula → ETH bridge unlock flow: lock NBL on vault, relayer calls POST /bridge/unlock.
 */
const { fetchJson, sleep, passBanner, failAndExit } = require('./lib/helpers');
const { fundAddress } = require('./lib/faucet');
const { submitLockTx, generateWallet } = require('./lib/bridge-tx');

async function waitReceipt(hash) {
  const clean = hash.replace(/^0x/, '');
  for (let i = 0; i < 20; i++) {
    await sleep(2500);
    try {
      const rec = await fetchJson(`http://localhost:8545/tx/${clean}`);
      if (rec.receipt?.success) return rec.receipt;
    } catch {
      /* pending */
    }
  }
  throw new Error('Lock transaction receipt not found');
}

async function test22_BridgeUnlock() {
  console.log('🧪 Test 6.2: Nebula Lock → ETH Unlock Flow\n');
  try {
    const wallet = generateWallet();
    const ethRecipient = '0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18';
    await fundAddress(wallet.address);

    console.log(`1. Locking NBL from ${wallet.address.slice(0, 10)}...`);
    const lockHash = await submitLockTx(wallet, ethRecipient, 1_000_000_000_000_000_000n);
    console.log(`   Lock tx: ${lockHash.slice(0, 18)}...`);

    await waitReceipt(lockHash);
    console.log('2. ✅ Nebula lock confirmed');

    const unlock = await fetchJson('http://localhost:8545/bridge/unlock', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        nebula_tx_hash: lockHash.startsWith('0x') ? lockHash : `0x${lockHash}`,
        eth_recipient: ethRecipient,
      }),
    });

    if (unlock.error) throw new Error(unlock.error);
    if (unlock.status !== 'unlock_queued') throw new Error(`Unexpected status: ${unlock.status}`);
    console.log(`3. ✅ Unlock queued for ${unlock.eth_recipient.slice(0, 10)}...`);

    const status = await fetchJson('http://localhost:8545/bridge/status');
    if ((status.processed_unlocks || 0) < 1) {
      throw new Error('processed_unlocks not incremented');
    }
    console.log(`4. ✅ Bridge status: ${status.processed_unlocks} unlock(s) processed`);

    passBanner('Nebula Lock → ETH Unlock');
  } catch (e) {
    failAndExit(e);
  }
}

test22_BridgeUnlock();
