const crypto = require('crypto');
const nacl = require('tweetnacl');
const { API_URL, fetchJson } = require('./helpers');

const fromHex = (hex) => {
  const clean = hex.replace(/^0x/, '');
  const b = Buffer.alloc(clean.length / 2);
  for (let i = 0; i < clean.length; i += 2) b[i / 2] = parseInt(clean.slice(i, i + 2), 16);
  return b;
};

async function sha256(data) {
  return crypto.createHash('sha256').update(data).digest();
}

async function govTxHash(tx) {
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
  const inner = Buffer.concat(parts);
  return sha256(await sha256(inner));
}

function buildGovTx(wallet, payloadObj, nonceVal) {
  return {
    sender: Array.from(fromHex(wallet.address.replace('0x', ''))),
    nonce: nonceVal,
    value: 0,
    gas_limit: 21000,
    max_fee_per_gas: 1e9,
    max_priority_fee_per_gas: 1e8,
    payload: Array.from(Buffer.from(JSON.stringify(payloadObj))),
    chain_id: 1,
    signature: [],
  };
}

async function submitGovTx(wallet, payloadObj) {
  const bal = await fetchJson(`${API_URL}/balance/${wallet.address}`);
  const nonce = bal.nonce || 0;
  const tx = buildGovTx(wallet, payloadObj, nonce);
  const msg = await govTxHash(tx);
  tx.signature = Array.from(nacl.sign.detached(msg, wallet.secretKey));
  const res = await fetch(`${API_URL}/submit_tx`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(tx),
  });
  const text = await res.text();
  if (!res.ok) throw new Error(text);
  return text.replace(/^"|"$/g, '');
}

function generateWallet() {
  const kp = nacl.sign.keyPair();
  const hash = crypto.createHash('sha256').update(Buffer.from(kp.publicKey)).digest();
  const address = '0x' + hash.slice(0, 20).toString('hex');
  return { publicKey: kp.publicKey, secretKey: kp.secretKey, address };
}

module.exports = { submitGovTx, generateWallet, buildGovTx };
