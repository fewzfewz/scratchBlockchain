const crypto = require('crypto');
const { API_URL, fetchJson } = require('./helpers');
const { generateWallet } = require('./gov-tx');
const { signTx, txHash } = require('./tx-sign');

const fromHex = (hex) => {
  const clean = hex.replace(/^0x/, '');
  const b = Buffer.alloc(clean.length / 2);
  for (let i = 0; i < clean.length; i += 2) b[i / 2] = parseInt(clean.slice(i, i + 2), 16);
  return b;
};

function bridgeVault() {
  const hash = crypto.createHash('sha256').update('nebula-bridge-vault-v1').digest();
  return '0x' + hash.slice(0, 20).toString('hex');
}

function encodeBridgePayload(destChain, recipient) {
  return Array.from(Buffer.from(`BRIDGE:${destChain}:${recipient}`));
}

async function submitLockTx(wallet, ethRecipient, amountWei) {
  const vault = bridgeVault();
  const bal = await fetchJson(`${API_URL}/balance/${wallet.address}`);
  const nonce = bal.nonce || 0;
  const payload = encodeBridgePayload('ethereum', ethRecipient);
  const tx = {
    sender: Array.from(fromHex(wallet.address.replace('0x', ''))),
    to: Array.from(fromHex(vault.replace('0x', ''))),
    nonce,
    value: Number(amountWei),
    gas_limit: 120000,
    max_fee_per_gas: 1e9,
    max_priority_fee_per_gas: 1e9,
    payload,
    chain_id: 1,
    signature: [],
  };
  signTx(tx, wallet);
  const res = await fetch(`${API_URL}/submit_tx`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(tx),
  });
  const text = await res.text();
  let data;
  try {
    data = JSON.parse(text);
  } catch {
    data = { hash: text.replace(/^"|"$/g, '') };
  }
  if (!res.ok && !data.hash) throw new Error(text);
  return data.hash || txHash(tx).toString('hex');
}

module.exports = { bridgeVault, submitLockTx, generateWallet };
