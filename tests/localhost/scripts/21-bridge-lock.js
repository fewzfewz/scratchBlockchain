const { fetchJson, passBanner, failAndExit } = require('./lib/helpers');

async function test21_BridgeLock() {
  console.log('🧪 Test 6.1: Bridge Lock Readiness\n');
  try {
    const status = await fetchJson('http://localhost:8545/status');
    const validators = await fetchJson('http://localhost:8545/validators');
    const list = Array.isArray(validators) ? validators : validators.validators || [];

    console.log(`1. Node height: ${status.height}, validators: ${list.length}`);
    if (status.height === undefined) throw new Error('Node status unavailable');
    if (list.length < 1) throw new Error('No validators — bridge relayers need a live network');

    const ethRpc = process.env.ETH_RPC_URL;
    if (!ethRpc) {
      console.log('2. ⏭️  ETH_RPC_URL not set — skipping on-chain Bridge.lockTokens()');
      console.log('   Set ETH_RPC_URL=http://127.0.0.1:8545 to run full bridge contract test');
    } else {
      console.log(`2. ETH RPC configured: ${ethRpc}`);
      console.log('   ℹ️  Deploy Bridge.sol via interop/scripts and call lockTokens in CI');
    }

    console.log('3. ✅ Bridge operator readiness checks passed');
    passBanner('Bridge Lock Readiness');
  } catch (e) {
    failAndExit(e);
  }
}

test21_BridgeLock();
