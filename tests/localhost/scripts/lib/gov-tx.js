const nacl = require('tweetnacl');
const { API_URL, fetchJson } = require('./helpers');
const { signTx, addressFromPublicKey } = require('./tx-sign');

const fromHex = (hex) => {
  const clean = hex.replace(/^0x/, '');
  const b = Buffer.alloc(clean.length / 2);
  for (let i = 0; i < clean.length; i += 2) b[i / 2] = parseInt(clean.slice(i, i + 2), 16);
  return b;
};

function buildGovTx(wallet, payloadObj, nonceVal) {
  return {
    sender: Array.from(fromHex(wallet.address.replace('0x', ''))),
    nonce: nonceVal,
    value: 0,
    gas_limit: 21000,
    max_fee_per_gas: 1e9,
    max_priority_fee_per_gas: 1e9,
    payload: Array.from(Buffer.from(JSON.stringify(payloadObj))),
    chain_id: 1,
    to: null,
    signature: [],
  };
}

async function submitGovTx(wallet, payloadObj) {
  const bal = await fetchJson(`${API_URL}/balance/${wallet.address}`);
  const nonce = bal.nonce || 0;
  const tx = signTx(buildGovTx(wallet, payloadObj, nonce), wallet);
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
  return { publicKey: kp.publicKey, secretKey: kp.secretKey, address: addressFromPublicKey(kp.publicKey) };
}

module.exports = { submitGovTx, generateWallet, buildGovTx };
