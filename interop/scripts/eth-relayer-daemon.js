#!/usr/bin/env node
/**
 * Poll Nebula for pending bridge unlocks and submit Bridge.unlockTokens on Ethereum.
 *
 * Env:
 *   NEBULA_RPC_URL   — default http://localhost:8545
 *   ETH_RPC_URL      — Hardhat / Sepolia JSON-RPC
 *   ETH_BRIDGE_ADDRESS — deployed Bridge.sol
 *   RELAYER_PRIVATE_KEYS — comma-separated hex keys (need >= requiredSignatures)
 *   BRIDGE_DEPLOY_OUT — optional path to bridge-deployment.json
 */
const fs = require('fs');
const path = require('path');
const { ethers } = require('ethers');
const {
  buildMessageFromPending,
  signBridgeMessage,
} = require('./lib/bridge-message');

const NEBULA_RPC = process.env.NEBULA_RPC_URL || 'http://localhost:8545';
const POLL_MS = Number(process.env.RELAYER_POLL_MS || 5000);

function loadConfig() {
  const deployPath =
    process.env.BRIDGE_DEPLOY_OUT ||
    path.join(__dirname, '../../deployment/local/configs/bridge-deployment.json');
  if (fs.existsSync(deployPath)) {
    return JSON.parse(fs.readFileSync(deployPath, 'utf8'));
  }
  return {};
}

async function fetchJson(url, opts) {
  const res = await fetch(url, opts);
  const text = await res.text();
  try {
    return JSON.parse(text);
  } catch {
    throw new Error(text);
  }
}

async function processPending(cfg, ethProvider, bridge, relayerWallets) {
  const data = await fetchJson(`${NEBULA_RPC}/bridge/pending_unlocks`);
  const pending = data.pending || [];
  if (pending.length === 0) return;

  for (const entry of pending) {
    try {
      const msg = buildMessageFromPending(entry);
      const sigs = [];
      for (const w of relayerWallets.slice(0, cfg.requiredSignatures || 2)) {
        sigs.push(await signBridgeMessage(msg, w));
      }

      const tx = await bridge.unlockTokens(
        {
          id: msg.id,
          sourceChain: msg.sourceChain,
          destChain: msg.destChain,
          sender: msg.sender,
          recipient: msg.recipient,
          token: msg.token,
          amount: msg.amount,
          nonce: msg.nonce,
        },
        sigs,
      );
      const receipt = await tx.wait();
      console.log(
        `[relayer] unlock ${entry.nebula_tx_hash.slice(0, 12)}… → ETH tx ${receipt.hash}`,
      );

      await fetchJson(`${NEBULA_RPC}/bridge/unlock/ack`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          nebula_tx_hash: entry.nebula_tx_hash.startsWith('0x')
            ? entry.nebula_tx_hash
            : `0x${entry.nebula_tx_hash}`,
          eth_tx_hash: receipt.hash,
        }),
      });
    } catch (e) {
      console.error(`[relayer] failed ${entry.nebula_tx_hash}:`, e.message || e);
    }
  }
}

async function main() {
  const cfg = loadConfig();
  const ethRpc = process.env.ETH_RPC_URL || cfg.ethRpcUrl;
  const bridgeAddr = process.env.ETH_BRIDGE_ADDRESS || cfg.bridge;
  if (!ethRpc || !bridgeAddr) {
    console.error('Set ETH_RPC_URL and ETH_BRIDGE_ADDRESS (or bridge-deployment.json)');
    process.exit(1);
  }

  const keys = (process.env.RELAYER_PRIVATE_KEYS || '')
    .split(',')
    .map((k) => k.trim())
    .filter(Boolean);
  if (keys.length === 0) {
    console.error('Set RELAYER_PRIVATE_KEYS (comma-separated)');
    process.exit(1);
  }

  const ethProvider = new ethers.JsonRpcProvider(ethRpc);
  const relayerWallets = keys.map((k) => new ethers.Wallet(k, ethProvider));
  const artifact = JSON.parse(
    fs.readFileSync(
      path.join(__dirname, '../artifacts/contracts/Bridge.sol/Bridge.json'),
      'utf8',
    ),
  );
  const bridge = new ethers.Contract(bridgeAddr, artifact.abi, relayerWallets[0]);

  console.log(`[relayer] Nebula ${NEBULA_RPC} → ETH ${bridgeAddr} (${keys.length} keys)`);

  for (;;) {
    try {
      await processPending(cfg, ethProvider, bridge, relayerWallets);
    } catch (e) {
      console.error('[relayer] poll error:', e.message || e);
    }
    await new Promise((r) => setTimeout(r, POLL_MS));
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
