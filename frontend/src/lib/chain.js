import nacl from 'tweetnacl'

export const getApiUrl = () => localStorage.getItem('nebula_rpc_url') || 'http://localhost:8545'

export const toHex = (buf) =>
  Array.from(new Uint8Array(buf)).map((b) => b.toString(16).padStart(2, '0')).join('')

export const fromHex = (hex) => {
  const clean = (hex || '').replace(/^0x/, '')
  const b = new Uint8Array(clean.length / 2)
  for (let i = 0; i < clean.length; i += 2) b[i / 2] = parseInt(clean.substr(i, 2), 16)
  return b
}

export const padAddr = (addr) => (addr || '').replace(/^0x/, '').toLowerCase().padStart(64, '0')

export const weiToNbl = (wei) => {
  const n = String(wei ?? '0')
  if (n === '0') return '0.0000'
  const p = n.padStart(19, '0')
  return (p.slice(0, -18) || '0') + '.' + p.slice(-18, -14)
}

export const nblToWei = (nbl) => {
  const parts = String(nbl).split('.')
  const whole = parts[0] || '0'
  const frac = (parts[1] || '').padEnd(18, '0').slice(0, 18)
  return BigInt(whole) * 10n ** 18n + BigInt(frac || '0')
}

export const shorten = (s, a = 8, b = 6) =>
  s && s.length > a + b ? `${s.slice(0, a)}…${s.slice(-b)}` : s || '--'

export const decodeUint256 = (hex) => {
  if (!hex || hex === '0x') return 0n
  const clean = hex.replace(/^0x/, '')
  if (!clean) return 0n
  return BigInt('0x' + clean.slice(-64))
}

const sha256 = async (data) => new Uint8Array(await crypto.subtle.digest('SHA-256', data))

export async function deriveAddress(pubBytes) {
  const hash = await sha256(pubBytes)
  return toHex(hash.slice(0, 20))
}

export async function hashAddress(label) {
  const hash = await sha256(new TextEncoder().encode(label))
  return '0x' + toHex(hash.slice(0, 20))
}

export const BRIDGE_VAULT = () => hashAddress('nebula-bridge-vault-v1')
export const DEFI_POOL = () => hashAddress('nebula-defi-pool-v1')

export const BRIDGE_TOKENS = [
  { symbol: 'USDC', decimals: 6, eth: '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48' },
  { symbol: 'USDT', decimals: 6, eth: '0xdAC17F958D2ee523a2206206994597C13D831ec7' },
  { symbol: 'ETH', decimals: 18, eth: '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2' },
]

export async function mappedBridgeToken(ethAddress) {
  const hash = await sha256(new TextEncoder().encode('nebula-bridge-token-v1:' + ethAddress.toLowerCase()))
  return '0x' + toHex(hash.slice(0, 20))
}

export const CONTRACTS_KEY = 'nebula_deployed_contracts'

export function loadSavedContracts() {
  try {
    return JSON.parse(localStorage.getItem(CONTRACTS_KEY) || '[]')
  } catch {
    return []
  }
}

export function saveContract(entry) {
  const list = loadSavedContracts().filter((c) => c.address !== entry.address)
  list.unshift({ ...entry, savedAt: Date.now() })
  localStorage.setItem(CONTRACTS_KEY, JSON.stringify(list.slice(0, 50)))
}

export async function fetchJson(path, opts) {
  const r = await window.fetch(`${getApiUrl()}${path}`, opts)
  if (!r.ok) throw new Error(await r.text())
  const ct = r.headers.get('content-type') || ''
  if (ct.includes('json')) return r.json()
  return r.text()
}

export async function fetchBalance(address) {
  const d = await fetchJson(`/balance/${address.replace(/^0x/, '')}`)
  return { balance: d.balance, nonce: d.nonce ?? 0 }
}

export async function fetchStatus() {
  return fetchJson('/status')
}

export async function fetchGasPrice() {
  return fetchJson('/gas_price')
}

export async function fetchTxHistory(address, limit = 50) {
  return fetchJson(`/txs/${address.replace(/^0x/, '')}?limit=${limit}`)
}

export async function callContract(from, to, data, value = '0') {
  return fetchJson('/call_contract', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ from, to, data, value }),
  })
}

export async function estimateGas(from, to, data, value = '0') {
  return fetchJson('/estimate_gas', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ from, to, data, value }),
  })
}

export function loadWalletKeyPair() {
  const priv = localStorage.getItem('nebula_wallet_priv')
  if (!priv) return null
  try {
    const secret = fromHex(priv)
    return nacl.sign.keyPair.fromSecretKey(secret)
  } catch {
    return null
  }
}

export function walletAddress() {
  return localStorage.getItem('nebula_wallet_addr') || ''
}

export async function txHash(tx) {
  let h = new Uint8Array(0)
  const ap = (b) => {
    const a = new Uint8Array(h.length + b.length)
    a.set(h)
    a.set(b, h.length)
    h = a
  }
  ap(new Uint8Array(tx.sender.slice(0, 20)))
  const nb = new Uint8Array(8)
  new DataView(nb.buffer).setBigUint64(0, BigInt(tx.nonce), true)
  ap(nb)
  ap(new Uint8Array(tx.payload))
  const gb = new Uint8Array(8)
  new DataView(gb.buffer).setBigUint64(0, BigInt(tx.gas_limit), true)
  ap(gb)
  const fb = new Uint8Array(8)
  new DataView(fb.buffer).setBigUint64(0, BigInt(tx.max_fee_per_gas), true)
  ap(fb)
  const pb = new Uint8Array(8)
  new DataView(pb.buffer).setBigUint64(0, BigInt(tx.max_priority_fee_per_gas), true)
  ap(pb)
  if (tx.chain_id) {
    const cb = new Uint8Array(8)
    new DataView(cb.buffer).setBigUint64(0, BigInt(tx.chain_id), true)
    ap(cb)
  }
  if (tx.to && tx.to.length) ap(new Uint8Array(tx.to.slice(0, 20)))
  const vb = new Uint8Array(8)
  new DataView(vb.buffer).setBigUint64(0, BigInt(tx.value), true)
  ap(vb)
  return sha256(await sha256(h))
}

export async function buildSignedTx({ from, to, valueWei, payload = [], gasLimit = 21000, chainId = 1 }) {
  const kp = loadWalletKeyPair()
  if (!kp) throw new Error('No wallet — create one on /wallet')
  const { nonce } = await fetchBalance(from)
  const tx = {
    sender: Array.from(fromHex(from.replace(/^0x/, ''))),
    to: to ? Array.from(fromHex(to.replace(/^0x/, ''))) : [],
    nonce,
    value: Number(valueWei),
    gas_limit: gasLimit,
    max_fee_per_gas: 1e9,
    max_priority_fee_per_gas: 1e8,
    payload: Array.isArray(payload) ? payload : Array.from(fromHex(String(payload).replace(/^0x/, ''))),
    chain_id: chainId,
    signature: [],
  }
  const msg = await txHash(tx)
  tx.signature = Array.from(nacl.sign.detached(msg, kp.secretKey))
  return tx
}

export async function submitTx(tx) {
  const r = await window.fetch(`${getApiUrl()}/submit_tx`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(tx),
  })
  const text = await r.text()
  if (!r.ok) throw new Error(text)
  return text.replace(/^"|"$/g, '')
}

export async function signAndSubmit(opts) {
  const tx = await buildSignedTx(opts)
  const hash = await submitTx(tx)
  return { hash, tx }
}

export async function waitForReceipt(hash, attempts = 12, delayMs = 2500) {
  for (let i = 0; i < attempts; i++) {
    try {
      const d = await fetchJson(`/tx/${hash.replace(/^0x/, '')}`)
      if (d.receipt) return d.receipt
    } catch {
      /* retry */
    }
    await new Promise((r) => setTimeout(r, delayMs))
  }
  return null
}

// ERC20 / ERC721 calldata helpers
export const SELECTORS = {
  balanceOf: '0x70a08231',
  totalSupply: '0x18160ddd',
  transfer: '0xa9059cbb',
  approve: '0x095ea7b3',
  mint: '0x40c10f19',
  ownerOf: '0x6352211e',
  transferFrom: '0x23b872dd',
}

export function encodeBalanceOf(holder) {
  return SELECTORS.balanceOf + padAddr(holder)
}

export function encodeTransfer(to, amountWei) {
  return SELECTORS.transfer + padAddr(to) + amountWei.toString(16).padStart(64, '0')
}

export function encodeMint(to, tokenId) {
  return SELECTORS.mint + padAddr(to) + BigInt(tokenId).toString(16).padStart(64, '0')
}

export function encodeOwnerOf(tokenId) {
  return SELECTORS.ownerOf + BigInt(tokenId).toString(16).padStart(64, '0')
}

export function encodeBridgePayload(destChain, recipient) {
  const text = `BRIDGE:${destChain}:${recipient}`
  return Array.from(new TextEncoder().encode(text))
}

export async function scanDeployedContracts() {
  const saved = loadSavedContracts()
  const byAddr = new Map(saved.map((c) => [c.address.toLowerCase(), c]))
  const viewer = walletAddress()
  if (!viewer) return Array.from(byAddr.values())
  try {
    const hist = await fetchTxHistory(viewer, 50)
    for (const tx of hist.transactions || []) {
      if (!tx.is_contract_creation) continue
      try {
        const rec = await fetchJson(`/tx/${tx.hash.replace(/^0x/, '')}`)
        const raw = rec.receipt?.contract_address || rec.receipt?.created_address
        if (!raw) continue
        const full = String(raw).startsWith('0x') ? String(raw) : `0x${raw}`
        if (!byAddr.has(full.toLowerCase())) {
          byAddr.set(full.toLowerCase(), {
            address: full,
            type: 'unknown',
            name: `Contract block ${tx.block_height}`,
            blockHeight: tx.block_height,
            txHash: tx.hash,
          })
        }
      } catch {
        /* receipt pending */
      }
    }
  } catch {
    /* offline */
  }
  return Array.from(byAddr.values())
}

export async function erc20Balance(contract, holder) {
  const res = await callContract(holder, contract, encodeBalanceOf(holder))
  if (!res.success) return 0n
  return decodeUint256(res.result)
}

export async function nftOwner(contract, tokenId, from) {
  const res = await callContract(from || walletAddress(), contract, encodeOwnerOf(tokenId))
  if (!res.success) return null
  const raw = res.result.replace(/^0x/, '')
  if (raw.length < 40) return null
  return '0x' + raw.slice(-40)
}

export async function loadNftsFromChain(contract, viewer) {
  const addr = viewer || walletAddress()
  if (!contract || !addr) return []
  const nfts = []
  for (let id = 1; id <= 24; id++) {
    try {
      const owner = await nftOwner(contract, id, addr)
      if (owner && owner.toLowerCase() !== '0x' + '0'.repeat(40)) {
        nfts.push({ tokenId: id, owner, contract, name: `Token #${id}` })
      }
    } catch {
      break
    }
  }
  return nfts
}
