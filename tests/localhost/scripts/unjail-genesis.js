// Devnet helper: unjail a genesis validator with its key from deployment/local/configs.
const fs = require('fs');
const path = require('path');
const nacl = require('tweetnacl');
const { submitGovTx, waitForReceipt } = require('./lib/gov-tx');
const { addressFromPublicKey } = require('./lib/tx-sign');

(async () => {
  const name = process.argv[2] || 'validator1';
  const file = path.resolve(__dirname, `../../../deployment/local/configs/${name}_key.json`);
  const kp = nacl.sign.keyPair.fromSeed(Buffer.from(JSON.parse(fs.readFileSync(file, 'utf8')).secret_key, 'hex'));
  const wallet = { publicKey: kp.publicKey, secretKey: kp.secretKey, address: addressFromPublicKey(kp.publicKey) };
  const r = await waitForReceipt(await submitGovTx(wallet, { action: 'unjail' }), 30);
  console.log(`${name} unjail: ${r.success ? 'ok' : 'refused: ' + r.revert_reason}`);
})().catch((e) => { console.error(e.message); process.exit(1); });
