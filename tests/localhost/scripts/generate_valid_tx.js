
const crypto = require('crypto');
const http = require('http');
const { txHash } = require('./lib/tx-sign');

// Helper to create Ed25519 keypair
const { privateKey, publicKey } = crypto.generateKeyPairSync('ed25519');

// Helper to convert buffer to array
const toArray = (buf) => Array.from(buf);

// Address = last 20 bytes of SHA-256(public key), as the node derives it
const createAddress = (pubKey) => {
    const hash = crypto.createHash('sha256').update(pubKey).digest();
    return toArray(hash.subarray(-20));
};

const senderPub = publicKey.export({ type: 'spki', format: 'der' }).slice(-32); // raw 32 bytes

// A signed native self-transfer of 0 (no payload)
const tx = {
    sender: createAddress(senderPub),
    nonce: 0,
    public_key: toArray(senderPub),
    payload: [],
    signature: [],
    gas_limit: 30000,
    max_fee_per_gas: 1000000000,
    max_priority_fee_per_gas: 1000000000,
    chain_id: 1,
    value: 0,
    to: null,
};
tx.to = tx.sender;
tx.signature = toArray(crypto.sign(null, txHash(tx), privateKey));

console.log('Sending transaction:', JSON.stringify(tx, null, 2));

// Send to RPC
const req = http.request({
    hostname: 'localhost',
    port: 26657,
    path: '/submit_tx',
    method: 'POST',
    headers: {
        'Content-Type': 'application/json'
    }
}, (res) => {
    let data = '';
    res.on('data', (chunk) => data += chunk);
    res.on('end', () => {
        console.log('Response:', data);
    });
});

req.on('error', (e) => {
    console.error('Error:', e);
});

req.write(JSON.stringify(tx));
req.end();
