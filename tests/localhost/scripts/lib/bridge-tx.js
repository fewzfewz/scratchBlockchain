const crypto = require('crypto');
const nacl = require('tweetnacl');
const { API_URL, fetchJson } = require('./helpers');
const { generateWallet } = require('./gov-tx');

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

async function txHash(tx) {
  const parts = [];
  parts.push(Buffer.from(tx.sender.slice(0, 20)));
  const nonce = Buffer.alloc(8);
  nonce.writeBigUInt64LE(BigInt(tx.nonce));
  parts.push(nonce);
  parts.push(Buffer.from(tx.payload));
  const gas = Buffer.alloc(8);
  gas.writeBigUInt64LE(BigInt(tx.gas_limit));
  parts.push(gas);
  const fee = Buffer.alloc(8);
  fee.writeBigUInt64LE(BigInt(tx.max_fee_per_gas));
  parts.push(fee);
  const prio = Buffer.alloc(8);
  prio.writeBigUInt64LE(BigInt(tx.max_priority_fee_per_gas));
  parts.push(prio);
  if (tx.chain_id) {
    const cid = Buffer.alloc(8);
    cid.writeBigUInt64LE(BigInt(tx.chain_id));
    parts.push(cid);
  }
  if (tx.to && tx.to.length) parts.push(Buffer.from(tx.to.slice(0, 20)));
  const val = Buffer.alloc(8);
  val.writeBigUInt64LE(BigInt(tx.value));
  parts.push(val);
  return crypto.createHash('sha256').update(Buffer.concat(parts)).digest('hex');
}

async function submitLockTx(wallet, ethRecipient, amountWei) {
  const vault = bridgeVault();
  const bal = await fetchJson(`${API_URL}/balance/${wallet.address}`);
  const nonce = bal.nonce || 0;
  const payload = [...Array.from(wallet.publicKey), ...encodeBridgePayload('ethereum', ethRecipient)];
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
  const msg = Buffer.from(await txHash(tx), 'hex');
  tx.signature = Array.from(nacl.sign.detached(msg, wallet.secretKey));
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
  return data.hash || (await txHash(tx));
}

module.exports = { bridgeVault, submitLockTx, generateWallet };
