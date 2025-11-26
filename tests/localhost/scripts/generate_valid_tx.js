
const crypto = require('crypto');
const http = require('http');

// Helper to create Ed25519 keypair
const { privateKey, publicKey } = crypto.generateKeyPairSync('ed25519');

// Helper to convert buffer to array
const toArray = (buf) => Array.from(buf);

// Helper to create address from public key (first 20 bytes of hash)
const createAddress = (pubKey) => {
    const hash = crypto.createHash('sha256').update(pubKey).digest();
    return toArray(hash.slice(0, 20));
};

// Transaction parameters
const senderPub = publicKey.export({ type: 'spki', format: 'der' }).slice(-32); // Extract raw 32 bytes
const senderAddr = createAddress(senderPub);
const nonce = 0;
const gasLimit = 30000n;
const maxFeePerGas = 1000000000n;
const maxPriorityFeePerGas = 1000000000n;
const chainId = 1n; // Optional
const value = 0n;

// Build payload: [pubkey(32)]
const payload = Buffer.from(senderPub);

// Calculate transaction hash for signing
// Hash = SHA256(sender + nonce + payload + gas_limit + max_fee + max_priority + chain_id)
const hasher = crypto.createHash('sha256');
hasher.update(Buffer.from(senderAddr));
const nonceBuf = Buffer.alloc(8);
nonceBuf.writeBigUInt64LE(BigInt(nonce));
hasher.update(nonceBuf);
hasher.update(payload);
const gasLimitBuf = Buffer.alloc(8);
gasLimitBuf.writeBigUInt64LE(gasLimit);
hasher.update(gasLimitBuf);
const maxFeeBuf = Buffer.alloc(8);
maxFeeBuf.writeBigUInt64LE(maxFeePerGas);
hasher.update(maxFeeBuf);
const maxPriorityBuf = Buffer.alloc(8);
maxPriorityBuf.writeBigUInt64LE(maxPriorityFeePerGas);
hasher.update(maxPriorityBuf);
// chain_id is optional in struct but included in hash if present
// Assuming chain_id is present as per test code
const chainIdBuf = Buffer.alloc(8);
chainIdBuf.writeBigUInt64LE(chainId);
hasher.update(chainIdBuf);

// to is null, so skip

const valueBuf = Buffer.alloc(8);
valueBuf.writeBigUInt64LE(value);
hasher.update(valueBuf);

const txHash = hasher.digest();

// Sign the hash
const signature = crypto.sign(null, txHash, privateKey);

// Construct transaction object
const tx = {
    sender: senderAddr,
    nonce: nonce,
    payload: toArray(payload),
    signature: toArray(signature),
    gas_limit: Number(gasLimit),
    max_fee_per_gas: Number(maxFeePerGas),
    max_priority_fee_per_gas: Number(maxPriorityFeePerGas),
    chain_id: Number(chainId),
    value: Number(value),
    to: null // Contract creation / simple transfer
};

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
