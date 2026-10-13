// Canonical Nebula tx hashing/signing for the localhost test scripts.
// Must match Rust `Transaction::hash` (common/src/types.rs).
const crypto = require('crypto');

const u64le = (v) => {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(BigInt(v));
  return b;
};

function txHash(tx) {
  const payload = Buffer.from(tx.payload || []);
  const hasTo = Array.isArray(tx.to) ? tx.to.length > 0 : !!tx.to;
  const parts = [
    Buffer.from('NEBULA_TX_V1'),
    Buffer.from(tx.sender.slice(0, 20)),
    u64le(tx.nonce),
    Buffer.from(tx.public_key),
    u64le(payload.length),
    payload,
    u64le(tx.gas_limit),
    u64le(tx.max_fee_per_gas),
    u64le(tx.max_priority_fee_per_gas),
    tx.chain_id != null ? Buffer.concat([Buffer.of(1), u64le(tx.chain_id)]) : Buffer.of(0),
    hasTo ? Buffer.concat([Buffer.of(1), Buffer.from(tx.to.slice(0, 20))]) : Buffer.of(0),
    u64le(tx.value),
  ];
  return crypto.createHash('sha256').update(Buffer.concat(parts)).digest();
}

/** Set `public_key` and `signature` (Ed25519 over `txHash(tx)`). */
function signTx(tx, wallet) {
  const nacl = require('tweetnacl'); // lazy: txHash alone needs no deps
  tx.public_key = Array.from(wallet.publicKey);
  tx.signature = Array.from(nacl.sign.detached(txHash(tx), wallet.secretKey));
  return tx;
}

/** Address = last 20 bytes of SHA-256(public key), as the node derives it. */
function addressFromPublicKey(publicKey) {
  return '0x' + crypto.createHash('sha256').update(Buffer.from(publicKey)).digest().subarray(-20).toString('hex');
}

/** Headers for dev-mode admin routes; set NEBULA_ADMIN_TOKEN if the node requires one. */
function adminHeaders() {
  const token = process.env.NEBULA_ADMIN_TOKEN;
  return {
    'Content-Type': 'application/json',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
  };
}

module.exports = { txHash, signTx, addressFromPublicKey, adminHeaders };
