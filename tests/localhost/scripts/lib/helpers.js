const API_URL = process.env.NEBULA_API_URL || 'http://localhost:8545';
const RPC_URL = process.env.NEBULA_RPC_URL || 'http://localhost/rpc';

async function fetchJson(url, options) {
  const res = await fetch(url, options);
  const text = await res.text();
  let data;
  try {
    data = JSON.parse(text);
  } catch {
    data = text;
  }
  if (!res.ok) {
    throw new Error(typeof data === 'string' ? data : JSON.stringify(data));
  }
  return data;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function passBanner(name) {
  console.log('\n' + '='.repeat(50));
  console.log(`✅ TEST PASSED: ${name}`);
  console.log('='.repeat(50));
}

function failAndExit(err) {
  console.error('\n❌ TEST FAILED:', err.message || err);
  process.exit(1);
}

module.exports = { API_URL, RPC_URL, fetchJson, sleep, passBanner, failAndExit };
