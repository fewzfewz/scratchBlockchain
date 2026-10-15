#!/usr/bin/env node
/**
 * End-to-end throughput and finality latency on the running devnet.
 *
 * Funds WALLETS fresh accounts from the faucet, then has each submit
 * TXS_PER_WALLET signed transfers (consecutive nonces) as fast as the RPC
 * accepts them, spread over all nodes. Watches finalized blocks and reports:
 *   - throughput: included transfers / (last inclusion - first submit)
 *   - latency: submit -> first seen in a finalized block (p50 / p95 / max)
 *
 * Env: WALLETS (default 20), TXS_PER_WALLET (default 25), NEBULA_NODES
 * (comma-separated RPC URLs, default the 6 devnet nodes).
 */
const { fetchJson, sleep, passBanner, failAndExit } = require('./lib/helpers');
const { fundAddress } = require('./lib/faucet');
const { generateWallet } = require('./lib/gov-tx');
const { signTx, txHash } = require('./lib/tx-sign');

const WALLETS = Number(process.env.WALLETS || 20);
const TXS_PER_WALLET = Number(process.env.TXS_PER_WALLET || 25);
const NODES = (process.env.NEBULA_NODES ||
  'http://localhost:8545,http://localhost:8546,http://localhost:8547,http://localhost:8548,http://localhost:8549,http://localhost:8550'
).split(',');
const SINK = Array.from(Buffer.alloc(20, 0x42));

const fromHex = (hex) => Array.from(Buffer.from(hex.replace(/^0x/, ''), 'hex'));
const pct = (sorted, p) => sorted[Math.min(sorted.length - 1, Math.floor((sorted.length * p) / 100))];

async function height() {
  return (await fetchJson(`${NODES[0]}/status`)).height;
}

async function test42_Throughput() {
  console.log(`🧪 Throughput: ${WALLETS} wallets x ${TXS_PER_WALLET} transfers over ${NODES.length} nodes\n`);
  try {
    const wallets = Array.from({ length: WALLETS }, generateWallet);
    for (const w of wallets) await fundAddress(w.address, 10);
    console.log(`1. Funded ${WALLETS} wallets`);

    const pending = new Map(); // tx hash hex -> submit time (ms)
    const txs = [];
    for (const w of wallets) {
      for (let nonce = 0; nonce < TXS_PER_WALLET; nonce++) {
        const tx = signTx(
          {
            sender: fromHex(w.address),
            nonce,
            value: 1,
            gas_limit: 21000,
            max_fee_per_gas: 1e9,
            max_priority_fee_per_gas: 1e9,
            payload: [],
            chain_id: 1,
            to: SINK,
            signature: [],
          },
          w
        );
        txs.push(tx);
      }
    }

    // Watch finalized blocks from here on.
    let next = (await height()) + 1;
    const included = new Map(); // hash -> inclusion time
    let watching = true;
    const watcher = (async () => {
      while (watching) {
        const tip = await height().catch(() => next - 1);
        for (; next <= tip; next++) {
          const b = await fetchJson(`${NODES[0]}/block/${next}`).catch(() => null);
          const now = Date.now();
          for (const tx of b?.block?.extrinsics || []) {
            const h = txHash(tx).toString('hex');
            if (pending.has(h) && !included.has(h)) included.set(h, now);
          }
        }
        await sleep(50);
      }
    })();

    // Submit: one wallet's txs in nonce order to one node; wallets in parallel.
    const start = Date.now();
    let rejected = 0;
    await Promise.all(
      wallets.map(async (w, wi) => {
        const node = NODES[wi % NODES.length];
        for (let n = 0; n < TXS_PER_WALLET; n++) {
          const tx = txs[wi * TXS_PER_WALLET + n];
          pending.set(txHash(tx).toString('hex'), Date.now());
          const res = await fetch(`${node}/submit_tx`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(tx),
          });
          const text = await res.text();
          if (!res.ok || text.includes('error')) {
            rejected++;
            if (rejected <= 3) console.log(`   rejected: ${text.slice(0, 120)}`);
          }
        }
      })
    );
    const submitSecs = (Date.now() - start) / 1000;
    console.log(`2. Submitted ${txs.length} txs in ${submitSecs.toFixed(1)}s (${rejected} rejected)`);

    const deadline = Date.now() + 120_000;
    while (included.size < txs.length - rejected && Date.now() < deadline) await sleep(200);
    watching = false;
    await watcher;

    const lat = [...included].map(([h, t]) => t - pending.get(h)).sort((a, b) => a - b);
    const last = Math.max(...included.values());
    const tps = included.size / ((last - start) / 1000);
    console.log(`3. Included ${included.size}/${txs.length} in finalized blocks`);
    console.log(`   throughput: ${tps.toFixed(1)} tx/s`);
    console.log(
      `   submit -> finalized latency: p50 ${pct(lat, 50)} ms, p95 ${pct(lat, 95)} ms, max ${lat[lat.length - 1]} ms`
    );
    if (included.size < (txs.length - rejected) * 0.99) throw new Error('transactions were not included');
    passBanner('Throughput');
  } catch (e) {
    failAndExit(e);
  }
}

test42_Throughput();
