import { useState, useEffect, useCallback } from 'react'
import { Link } from 'react-router-dom'
import nacl from 'tweetnacl'
import { Wallet, Key, Eye, EyeOff, Copy, RefreshCw, Trash2, Send, Settings, Fingerprint, Coins, Fuel, Sliders, ShieldAlert, Droplets, FlaskConical, Plus, Check, History } from 'lucide-react'
import PageShell from '../components/PageShell.jsx'

const toHex = (buf) => Array.from(new Uint8Array(buf)).map(b => b.toString(16).padStart(2, '0')).join('')
const fromHex = (hex) => { const b = new Uint8Array(hex.length / 2); for (let i = 0; i < hex.length; i += 2) b[i / 2] = parseInt(hex.substr(i, 2), 16); return b }
const weiToNbl = (wei) => { const n = String(wei || '0'); if (n === '0') return '0.0000'; const p = n.padStart(19, '0'); return (p.slice(0, -18) || '0') + '.' + p.slice(-18, -14) }
const weiToFull = (wei) => { const n = String(wei || '0'); if (n === '0') return '0'; const p = n.padStart(19, '0'); return (p.slice(0, -18) || '0') + '.' + p.slice(-18) }
const FEE_NBL = 0.000021

const TEST_ADDRESSES = [
  '0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18',
  '0x8fD8fB8fB8fB8fD8fB8fB8fD8fB8fB8fD8fB8fD',
  '0x5B38Da6a701c568545dCfcB03FcB875f56beddC4',
  '0xAb8483F64d9C6d1EcF9b849Ae677dD3315835cb2',
]

const addrFromPub = async (pubBytes) => {
  const hash = await crypto.subtle.digest('SHA-256', pubBytes)
  return new Uint8Array(hash.slice(0, 20))
}

const inputCls = 'w-full px-3 py-2.5 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-800 dark:text-slate-200 placeholder-slate-400 dark:placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500/40'
const iconBtnCls = 'p-2 rounded-xl bg-slate-200/50 dark:bg-slate-700/50 border border-slate-300/50 dark:border-slate-600/50 text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-white hover:bg-slate-300/50 dark:hover:bg-slate-600/50 transition-colors'

export default function WalletPage() {
  const [apiUrl, setApiUrl] = useState(() => localStorage.getItem('nebula_rpc_url') || 'http://localhost:8545')
  const [keyPair, setKeyPair] = useState(null)
  const [pubKey, setPubKey] = useState('')
  const [address, setAddress] = useState('')
  const [privKey, setPrivKey] = useState('')
  const [showPriv, setShowPriv] = useState(false)
  const [balance, setBalance] = useState('0.0000')
  const [nonce, setNonce] = useState(0)
  const [status, setStatus] = useState('')
  const [statusType, setStatusType] = useState('')
  const [recipient, setRecipient] = useState('0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18')
  const [amount, setAmount] = useState('')
  const [showAdvanced, setShowAdvanced] = useState(false)
  const [gasLimit, setGasLimit] = useState(21000)
  const [showSettings, setShowSettings] = useState(false)
  const [settingsUrl, setSettingsUrl] = useState(apiUrl)
  const [nodeStatus, setNodeStatus] = useState('checking')

  // Multi-account
  const [accounts, setAccounts] = useState([])
  const [activeAccountIdx, setActiveAccountIdx] = useState(0)
  const [showNewAccount, setShowNewAccount] = useState(false)
  const [newAccountName, setNewAccountName] = useState('')

  // Tx history
  const [txHistory, setTxHistory] = useState([])
  const [showHistory, setShowHistory] = useState(false)
  const [historyLoading, setHistoryLoading] = useState(false)

  // Tokens tab
  const [walletTab, setWalletTab] = useState('balance') // balance | tokens

  useEffect(() => {
    loadSavedKey()
    checkNodeStatus()
    const statusInterval = setInterval(checkNodeStatus, 15000)
    return () => clearInterval(statusInterval)
  }, [])

  useEffect(() => {
    if (keyPair) {
      updateBalance()
      fetchNonce()
      const balInterval = setInterval(updateBalance, 10000)
      return () => clearInterval(balInterval)
    }
  }, [keyPair, address])

  useEffect(() => {
    if (address) {
      const saved = JSON.parse(localStorage.getItem(`nebula_tx_history_${address}`) || '[]')
      setTxHistory(saved)
    }
  }, [address])

  const fetchChainHistory = useCallback(async () => {
    if (!address) return
    setHistoryLoading(true)
    try {
      const r = await window.fetch(`${apiUrl}/txs/${address}?limit=25`)
      if (!r.ok) return
      const d = await r.json()
      const chain = (d.transactions || []).map((t) => ({
        hash: t.hash,
        to: t.is_contract_creation ? 'Contract deploy' : (t.to || '--'),
        amount: t.value ? weiToNbl(String(t.value)) : '0',
        timestamp: Date.now() - (d.scanned_blocks - t.block_height) * 2000,
        status: t.status || 'confirmed',
        blockHeight: t.block_height,
      }))
      const local = JSON.parse(localStorage.getItem(`nebula_tx_history_${address}`) || '[]')
      const byHash = new Map()
      for (const tx of [...chain, ...local]) {
        const existing = byHash.get(tx.hash)
        byHash.set(tx.hash, existing ? { ...existing, ...tx, status: tx.status === 'pending' ? existing.status : tx.status } : tx)
      }
      const merged = Array.from(byHash.values())
        .sort((a, b) => (b.timestamp || 0) - (a.timestamp || 0))
        .slice(0, 50)
      setTxHistory(merged)
    } catch {
      /* keep local history on RPC failure */
    } finally {
      setHistoryLoading(false)
    }
  }, [address, apiUrl])

  useEffect(() => {
    if (showHistory && address) fetchChainHistory()
  }, [showHistory, address, fetchChainHistory])

  useEffect(() => {
    const saved = JSON.parse(localStorage.getItem('nebula_accounts') || '[]')
    setAccounts(saved)
    const active = parseInt(localStorage.getItem('nebula_active_account') || '0')
    if (saved.length > 0 && saved[active]) {
      setActiveAccountIdx(active)
    }
  }, [])

  const showMsg = (msg, type = 'info') => { setStatus(msg); setStatusType(type); if (type === 'success') setTimeout(() => { setStatus(s => s === msg ? '' : s) }, 5000) }

  const copy = async (val, msg) => { try { await navigator.clipboard.writeText(val); showMsg(msg, 'success') } catch { showMsg('Failed to copy', 'error') } }

  const switchAccount = async (idx) => {
    const saved = JSON.parse(localStorage.getItem('nebula_accounts') || '[]')
    if (!saved[idx]) return
    const acc = saved[idx]
    const kp = { publicKey: fromHex(acc.pub), secretKey: fromHex(acc.priv) }
    setKeyPair(kp); setPubKey(acc.pub); setAddress(acc.addr); setPrivKey(acc.priv)
    setActiveAccountIdx(idx)
    localStorage.setItem('nebula_wallet_pub', acc.pub)
    localStorage.setItem('nebula_wallet_priv', acc.priv)
    localStorage.setItem('nebula_wallet_addr', acc.addr)
    localStorage.setItem('nebula_active_account', String(idx))
    showMsg(`Switched to ${acc.name || 'Account ' + (idx + 1)}`, 'success')
  }

  const saveAccount = (name, pub, priv, addr) => {
    const saved = JSON.parse(localStorage.getItem('nebula_accounts') || '[]')
    saved.push({ name, pub, priv, addr, created: Date.now() })
    localStorage.setItem('nebula_accounts', JSON.stringify(saved))
    setAccounts(saved)
    const idx = saved.length - 1
    setActiveAccountIdx(idx)
    localStorage.setItem('nebula_active_account', String(idx))
  }

  const deleteAccount = (idx) => {
    if (accounts.length <= 1) { showMsg('Cannot delete the last account', 'error'); return }
    if (!confirm(`Delete "${accounts[idx].name || 'Account ' + (idx + 1)}"?`)) return
    const saved = JSON.parse(localStorage.getItem('nebula_accounts') || '[]')
    saved.splice(idx, 1)
    localStorage.setItem('nebula_accounts', JSON.stringify(saved))
    setAccounts(saved)
    const nextIdx = Math.min(idx, saved.length - 1)
    switchAccount(nextIdx)
  }

  const generateKeyPair = async (name) => {
    try {
      const kp = nacl.sign.keyPair()
      const pub = toHex(kp.publicKey)
      const priv = toHex(kp.secretKey)
      const addrBytes = await addrFromPub(kp.publicKey)
      const addr = toHex(addrBytes)
      setKeyPair(kp); setPubKey(pub); setAddress(addr); setPrivKey(priv)
      localStorage.setItem('nebula_wallet_priv', priv)
      localStorage.setItem('nebula_wallet_pub', pub)
      localStorage.setItem('nebula_wallet_addr', addr)
      saveAccount(name || `Account ${accounts.length + 1}`, pub, priv, addr)
      showMsg('New wallet generated!', 'success')
    } catch { showMsg('Failed to generate keypair', 'error') }
  }

  const loadSavedKey = async () => {
    const saved = JSON.parse(localStorage.getItem('nebula_accounts') || '[]')
    if (saved.length > 0) {
      const active = parseInt(localStorage.getItem('nebula_active_account') || '0')
      const acc = saved[active] || saved[0]
      try {
        const kp = { publicKey: fromHex(acc.pub), secretKey: fromHex(acc.priv) }
        setKeyPair(kp); setPubKey(acc.pub); setAddress(acc.addr); setPrivKey(acc.priv)
        setActiveAccountIdx(saved.indexOf(acc))
        return
      } catch {}
    }
    // Fallback to legacy single-account storage
    const priv = localStorage.getItem('nebula_wallet_priv')
    const pub = localStorage.getItem('nebula_wallet_pub')
    let addr = localStorage.getItem('nebula_wallet_addr')
    if (!priv || !pub) return
    try {
      const kp = { publicKey: fromHex(pub), secretKey: fromHex(priv) }
      if (!addr) { const addrBytes = await addrFromPub(kp.publicKey); addr = toHex(addrBytes); localStorage.setItem('nebula_wallet_addr', addr) }
      setKeyPair(kp); setPubKey(pub); setAddress(addr); setPrivKey(priv)
      saveAccount('Account 1', pub, priv, addr)
    } catch { localStorage.removeItem('nebula_wallet_priv'); localStorage.removeItem('nebula_wallet_pub') }
  }

  const clearWallet = () => {
    if (!confirm('Clear current wallet?')) return
    const saved = JSON.parse(localStorage.getItem('nebula_accounts') || '[]')
    if (saved.length > 1) {
      const idx = activeAccountIdx
      saved.splice(idx, 1)
      localStorage.setItem('nebula_accounts', JSON.stringify(saved))
      setAccounts(saved)
      switchAccount(Math.min(idx, saved.length - 1))
      showMsg('Account removed.', 'info')
    } else {
      localStorage.removeItem('nebula_accounts'); localStorage.removeItem('nebula_active_account')
      localStorage.removeItem('nebula_wallet_priv'); localStorage.removeItem('nebula_wallet_pub'); localStorage.removeItem('nebula_wallet_addr')
      setKeyPair(null); setPubKey(''); setAddress(''); setPrivKey(''); setBalance('0.0000')
      setAccounts([]); showMsg('Wallet cleared.', 'info')
    }
  }

  const fetchNonce = async () => {
    if (!keyPair || !address) return 0
    try {
      const r = await window.fetch(`${apiUrl}/balance/${address}`)
      if (r.ok) { const d = await r.json(); setNonce(d.nonce || 0); return d.nonce || 0 }
    } catch { /* ignore */ }
    return 0
  }

  const updateBalance = useCallback(async () => {
    if (!keyPair || !address) { setBalance('0.0000'); return }
    try {
      const r = await window.fetch(`${apiUrl}/balance/${address}`)
      if (r.ok) { const d = await r.json(); setBalance(weiToNbl(d.balance || '0')) }
      else setBalance('0.0000')
    } catch { setBalance('?') }
  }, [keyPair, apiUrl, address])

  const checkNodeStatus = async () => {
    try {
      const r = await window.fetch(`${apiUrl}/health`)
      if (r.ok) { setNodeStatus('connected') } else throw Error()
    } catch { setNodeStatus('disconnected') }
  }

  const saveSettings = () => {
    setApiUrl(settingsUrl)
    localStorage.setItem('nebula_rpc_url', settingsUrl)
    setShowSettings(false)
    checkNodeStatus()
    if (keyPair) { updateBalance(); fetchNonce() }
  }

  const createTx = (to, amt, nonceVal) => {
    const amountInWei = Math.floor(parseFloat(amt) * 1e18)
    return {
      sender: Array.from(fromHex(address)),
      to: Array.from(fromHex(to.replace('0x', ''))),
      nonce: nonceVal, value: amountInWei,
      gas_limit: gasLimit, max_fee_per_gas: 1e9, max_priority_fee_per_gas: 1e8,
      payload: [], chain_id: 1, signature: [],
    }
  }

  const sha256 = async (data) => new Uint8Array(await crypto.subtle.digest('SHA-256', data))

  const txHash = async (tx) => {
    let h = new Uint8Array(0)
    const ap = (b) => { const a = new Uint8Array(h.length + b.length); a.set(h); a.set(b, h.length); h = a }
    ap(new Uint8Array(tx.sender.slice(0, 20)))
    const nb = new Uint8Array(8); new DataView(nb.buffer).setBigUint64(0, BigInt(tx.nonce), true); ap(nb)
    ap(new Uint8Array(tx.payload))
    const gb = new Uint8Array(8); new DataView(gb.buffer).setBigUint64(0, BigInt(tx.gas_limit), true); ap(gb)
    const fb = new Uint8Array(8); new DataView(fb.buffer).setBigUint64(0, BigInt(tx.max_fee_per_gas), true); ap(fb)
    const pb = new Uint8Array(8); new DataView(pb.buffer).setBigUint64(0, BigInt(tx.max_priority_fee_per_gas), true); ap(pb)
    if (tx.chain_id) { const cb = new Uint8Array(8); new DataView(cb.buffer).setBigUint64(0, BigInt(tx.chain_id), true); ap(cb) }
    if (tx.to && tx.to.length) ap(new Uint8Array(tx.to.slice(0, 20)))
    const vb = new Uint8Array(8); new DataView(vb.buffer).setBigUint64(0, BigInt(tx.value), true); ap(vb)
    return await sha256(await sha256(h))
  }

  const addTxToHistory = (hash, toAddr, amt) => {
    const entry = { hash, to: toAddr, amount: amt, timestamp: Date.now(), status: 'pending' }
    const updated = [entry, ...txHistory].slice(0, 50)
    setTxHistory(updated)
    localStorage.setItem(`nebula_tx_history_${address}`, JSON.stringify(updated))
    // Check receipt after a delay
    setTimeout(async () => {
      try {
        const r = await window.fetch(`${apiUrl}/tx/${hash}`)
        if (r.ok) {
          const d = await r.json()
          const finalStatus = d.receipt?.status ? 'confirmed' : 'failed'
          const hist = JSON.parse(localStorage.getItem(`nebula_tx_history_${address}`) || '[]')
          const updatedHist = hist.map(t => t.hash === hash ? { ...t, status: finalStatus } : t)
          localStorage.setItem(`nebula_tx_history_${address}`, JSON.stringify(updatedHist))
          setTxHistory(updatedHist)
        }
      } catch {}
    }, 5000)
  }

  const sendTx = async (e) => {
    e.preventDefault()
    if (!keyPair) { showMsg('Generate a wallet first', 'error'); return }
    if (!recipient) { showMsg('Enter a recipient address', 'error'); return }
    const amt = parseFloat(amount)
    if (isNaN(amt) || amt <= 0) { showMsg('Enter a valid amount', 'error'); return }
    const clean = recipient.replace('0x', '')
    if (clean.length !== 64 && clean.length !== 40) { showMsg('Invalid address', 'error'); return }
    try {
      showMsg('Preparing transaction...', 'info')
      const n = await fetchNonce()
      const tx = createTx(recipient, amt, n)
      const msg = await txHash(tx)
      const sig = nacl.sign.detached(msg, keyPair.secretKey)
      tx.signature = Array.from(sig)
      showMsg('Sending...', 'info')
      const r = await window.fetch(`${apiUrl}/submit_tx`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(tx),
      })
      const text = await r.text()
      if (r.ok) {
        const hash = text.replace(/^"|"$/g, '')
        showMsg(`Transaction sent! Hash: ${hash.slice(0, 8)}...${hash.slice(-8)}`, 'success')
        setAmount('')
        setNonce(n + 1)
        addTxToHistory(hash, recipient, amt)
        setTimeout(() => { updateBalance(); fetchNonce() }, 3000)
      } else showMsg('Transaction failed: ' + text, 'error')
    } catch (e) { showMsg('Error: ' + e.message, 'error') }
  }

  const formatTime = (ts) => {
    const d = new Date(ts)
    const now = new Date()
    const diff = now - d
    if (diff < 60000) return 'Just now'
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`
    return d.toLocaleDateString()
  }

  const shorten = (s) => s ? s.slice(0, 6) + '...' + s.slice(-4) : ''

  const setQuickAmount = (pct) => {
    const bal = parseFloat(balance)
    if (isNaN(bal) || bal <= 0) { showMsg('No spendable balance', 'error'); return }
    if (pct === 1) setAmount(Math.max(0, bal - FEE_NBL).toFixed(4))
    else setAmount((bal * pct).toFixed(4))
  }

  return (
    <PageShell variant="default">
      <div className="max-w-5xl mx-auto px-4 py-8 animate-fade-in">
        {/* ── Header ─────────────────────────────────────────────── */}
        <div className="flex items-center justify-between mb-6 flex-wrap gap-3">
          <div className="flex items-center gap-3">
            <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-blue-600 to-cyan-600 flex items-center justify-center text-white shadow-lg shadow-blue-600/25">
              <Wallet className="w-6 h-6" />
            </div>
            <div>
              <p className="text-xs uppercase tracking-widest text-blue-500 dark:text-blue-400 font-medium">Nebula Network</p>
              <h1 className="text-3xl md:text-4xl font-bold tracking-tight text-slate-900 dark:text-white">
                Nebula
                <span className="bg-gradient-to-r from-blue-500 via-cyan-500 to-violet-500 bg-clip-text text-transparent"> Wallet</span>
              </h1>
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <div className={`inline-flex items-center gap-2 px-3 py-1.5 rounded-full glass text-xs font-medium text-slate-600 dark:text-slate-300`}>
              <span className={`w-2 h-2 rounded-full ${nodeStatus === 'connected' ? 'bg-emerald-400 animate-pulse-dot' : nodeStatus === 'checking' ? 'bg-amber-400' : 'bg-red-400'}`} />
              {nodeStatus === 'connected' ? 'Node connected' : nodeStatus === 'checking' ? 'Checking…' : 'Node offline'}
            </div>
            <Link to="/faucet" className="inline-flex items-center gap-1.5 px-4 py-2 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-500 text-xs font-semibold hover:bg-amber-500/20 hover:-translate-y-0.5 transition-all">
              <Droplets className="w-3.5 h-3.5" /> Get Test Tokens
            </Link>
          </div>
        </div>

        {/* Connection strip */}
        <div className="flex items-center justify-between p-3 px-5 rounded-2xl glass-strong mb-6 text-sm">
          <div className="flex items-center gap-2">
            <span className={`w-2.5 h-2.5 rounded-full ${nodeStatus === 'connected' ? 'bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.5)]' : nodeStatus === 'checking' ? 'bg-amber-400' : 'bg-red-400'}`} />
            <span className="text-slate-500 dark:text-slate-300">{nodeStatus === 'connected' ? 'Connected to local testnet' : nodeStatus === 'checking' ? 'Checking node…' : 'Disconnected — start the testnet to use the wallet'}</span>
          </div>
          <div className="flex items-center gap-2">
            <code className="text-xs bg-black/20 dark:bg-black/30 px-2 py-1 rounded-md text-slate-500 dark:text-slate-400">{apiUrl}</code>
            <button onClick={() => { setSettingsUrl(apiUrl); setShowSettings(true) }} className={iconBtnCls}>
              <Settings className="w-4 h-4" />
            </button>
          </div>
        </div>

        <div className="grid md:grid-cols-[1fr_1.2fr] gap-4">
          {/* ─── Left Column ─── */}
          <div className="p-5 rounded-2xl glass animate-fade-in">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-3">
                <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-blue-500 to-cyan-600 flex items-center justify-center text-white shadow-md">
                  <Key className="w-4 h-4" />
                </div>
                <div>
                  <p className="text-xs uppercase tracking-wider text-blue-500 dark:text-blue-300">Identity</p>
                  <h2 className="text-lg font-semibold text-slate-900 dark:text-white">Account keys</h2>
                </div>
              </div>
              <div className="flex gap-2">
                <button onClick={() => { setNewAccountName(''); setShowNewAccount(true) }} className="flex items-center gap-1.5 px-3 py-2 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 text-white text-xs font-semibold shadow-md shadow-blue-600/20 hover:opacity-90 transition-all">
                  <Plus className="w-3.5 h-3.5" /> New
                </button>
                <button onClick={clearWallet} className={iconBtnCls} title="Remove account">
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              </div>
            </div>

            {/* Account selector */}
            {accounts.length > 1 && (
              <div className="mb-4">
                <label className="text-xs uppercase text-slate-500 dark:text-slate-400 mb-2 block">Switch account</label>
                <div className="flex flex-wrap gap-2">
                  {accounts.map((acc, idx) => (
                    <button key={idx} onClick={() => switchAccount(idx)}
                      className={`flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-medium transition-all ${
                        idx === activeAccountIdx
                          ? 'bg-blue-500/20 text-blue-600 dark:text-blue-300 border border-blue-500/30'
                          : 'bg-slate-200/50 dark:bg-slate-700/50 text-slate-600 dark:text-slate-400 border border-slate-300/50 dark:border-slate-600/50 hover:bg-slate-300/50 dark:hover:bg-slate-600/50'
                      }`}>
                      {idx === activeAccountIdx && <Check className="w-3 h-3" />}
                      {acc.name || `Acc ${idx + 1}`}
                    </button>
                  ))}
                </div>
              </div>
            )}

            <div className="mb-3">
              <label className="flex items-center gap-1.5 text-xs uppercase text-slate-500 dark:text-slate-400 mb-2"><Fingerprint className="w-3 h-3" /> Address (20 bytes)</label>
              <div className="flex gap-2">
                <input readOnly value={address} placeholder="Generate a wallet to begin" className={`${inputCls} font-mono`} />
                {address && <button onClick={() => copy(address, 'Address copied')} className={iconBtnCls}><Copy className="w-3.5 h-3.5" /></button>}
              </div>
            </div>

            <div className="mb-4">
              <label className="flex items-center gap-1.5 text-xs uppercase text-slate-500 dark:text-slate-400 mb-2"><Key className="w-3 h-3" /> Public key (32 bytes)</label>
              <div className="flex gap-2">
                <input readOnly value={pubKey} placeholder="Generated from keypair" className={`${inputCls} font-mono`} />
                {pubKey && <button onClick={() => copy(pubKey, 'Public key copied')} className={iconBtnCls}><Copy className="w-3.5 h-3.5" /></button>}
              </div>
            </div>

            <div className="mb-4">
              <label className="flex items-center gap-1.5 text-xs uppercase text-slate-500 dark:text-slate-400 mb-2"><Key className="w-3 h-3" /> Private key</label>
              <div className="flex gap-2">
                <input type={showPriv ? 'text' : 'password'} readOnly value={privKey} placeholder="Stored locally" className={`${inputCls} font-mono`} />
                {privKey && <button onClick={() => setShowPriv(!showPriv)} className={iconBtnCls}>{showPriv ? <EyeOff className="w-3.5 h-3.5" /> : <Eye className="w-3.5 h-3.5" />}</button>}
                {privKey && <button onClick={() => copy(privKey, 'Private key copied')} className={iconBtnCls}><Copy className="w-3.5 h-3.5" /></button>}
              </div>
            </div>

            <div className="p-3 rounded-xl bg-amber-50 dark:bg-amber-500/10 border border-amber-200 dark:border-amber-500/20 text-xs text-slate-600 dark:text-slate-400 flex gap-2">
              <ShieldAlert className="w-4 h-4 text-amber-500 dark:text-amber-400 shrink-0 mt-0.5" />
              <div><strong className="text-amber-600 dark:text-amber-300 block mb-1">Local demo wallet</strong> Stored in browser local storage. Development-only.</div>
            </div>

            {nonce > 0 && (
              <div className="mt-4 flex items-center gap-3 p-3 rounded-xl bg-slate-100 dark:bg-slate-700/40">
                <span className="text-xs text-slate-500 dark:text-slate-400">Nonce:</span>
                <code className="text-blue-600 dark:text-blue-400 font-bold">{nonce}</code>
                <span className="text-xs text-slate-500 dark:text-slate-400">Transaction count</span>
              </div>
            )}

            <div className="mt-4 p-3 rounded-xl bg-blue-50 dark:bg-blue-500/10 border border-blue-200 dark:border-blue-500/20">
              <p className="text-xs font-semibold text-blue-600 dark:text-blue-300 mb-2 flex items-center gap-1"><FlaskConical className="w-3 h-3" /> Test Addresses (ready to use)</p>
              <div className="space-y-1">
                {TEST_ADDRESSES.map(addr => (
                  <div key={addr} className="flex items-center gap-2">
                    <code className="text-[10px] font-mono text-slate-600 dark:text-slate-400 flex-1 truncate">{addr}</code>
                    <button onClick={() => copy(addr, 'Address copied')} className="text-blue-500 dark:text-blue-400 hover:text-blue-600 dark:hover:text-blue-300 shrink-0"><Copy className="w-3 h-3" /></button>
                    <button onClick={() => setRecipient(addr)} className="text-xs text-blue-500 dark:text-blue-400 hover:underline shrink-0">Use</button>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* ─── Right Column ─── */}
          <div className="space-y-4">
            {/* Balance hero */}
            <div className="p-5 rounded-2xl bg-gradient-to-br from-blue-600 via-cyan-600 to-teal-600 text-white shadow-xl shadow-blue-600/20 animate-fade-in relative overflow-hidden">
              <div className="absolute -top-20 -right-16 w-56 h-56 rounded-full opacity-30 pointer-events-none"
                style={{ background: 'radial-gradient(circle, rgba(255,255,255,0.4), transparent 70%)' }} />
              <div className="relative">
                <div className="flex items-center justify-between mb-4">
                  <div className="flex gap-1 p-1 rounded-xl bg-white/10">
                    <button onClick={() => setWalletTab('balance')}
                      className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${walletTab === 'balance' ? 'bg-white text-blue-600 shadow' : 'text-white/70 hover:text-white'}`}>
                      <span className="inline-flex items-center gap-1"><Coins className="w-3 h-3" /> Balance</span>
                    </button>
                    <button onClick={() => setWalletTab('tokens')}
                      className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${walletTab === 'tokens' ? 'bg-white text-blue-600 shadow' : 'text-white/70 hover:text-white'}`}>
                      <span className="inline-flex items-center gap-1"><Wallet className="w-3 h-3" /> Tokens</span>
                    </button>
                  </div>
                  <button onClick={() => { updateBalance(); fetchNonce(); showMsg('Balance refreshed', 'success') }} className="p-2 rounded-xl bg-white/10 hover:bg-white/20 transition-colors">
                    <RefreshCw className="w-3.5 h-3.5" />
                  </button>
                </div>

                {walletTab === 'balance' ? (
                  keyPair ? (
                    <>
                      <div className="mb-4">
                        <p className="text-xs uppercase tracking-wider text-white/60 mb-1">Available balance</p>
                        <div className="flex items-baseline gap-2">
                          <h3 className="text-4xl md:text-5xl font-bold tabular-nums drop-shadow">{balance}</h3>
                          <span className="text-white/70">NBL</span>
                          <button onClick={() => copy(address, 'Address copied')} className="ml-1 p-1.5 rounded-lg bg-white/10 hover:bg-white/20 transition-colors" title="Copy address">
                            <Copy className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>
                      <div className="grid grid-cols-2 gap-2 pt-4 border-t border-white/10">
                        <div className="rounded-xl bg-white/10 px-3 py-2">
                          <div className="text-[10px] uppercase tracking-wider text-white/60">Available</div>
                          <div className="text-sm font-semibold tabular-nums">{balance} NBL</div>
                        </div>
                        <div className="rounded-xl bg-white/10 px-3 py-2">
                          <div className="text-[10px] uppercase tracking-wider text-white/60">Reserved (gas)</div>
                          <div className="text-sm font-semibold tabular-nums">0.0000 NBL</div>
                        </div>
                      </div>
                    </>
                  ) : (
                    <div className="text-center py-6">
                      <p className="text-white/80 text-sm mb-4">No wallet loaded on this device.</p>
                      <button onClick={() => { setNewAccountName(''); setShowNewAccount(true) }}
                        className="inline-flex items-center gap-2 px-6 py-3 rounded-xl bg-white text-blue-600 text-sm font-bold shadow-lg hover:scale-[1.02] transition-transform">
                        <Plus className="w-4 h-4" /> Generate a new wallet
                      </button>
                    </div>
                  )
                ) : (
                  <div className="space-y-3">
                    <div className="flex items-center justify-between p-3 rounded-xl bg-white/10">
                      <div className="flex items-center gap-3">
                        <div className="w-9 h-9 rounded-full bg-white/20 flex items-center justify-center text-white text-xs font-bold">N</div>
                        <div>
                          <strong className="text-sm text-white block">Nebula (NBL)</strong>
                          <span className="text-xs text-white/60">Native token</span>
                        </div>
                      </div>
                      <div className="text-right">
                        <strong className="text-sm text-white block tabular-nums">{balance}</strong>
                        <span className="text-xs text-white/60">Testnet</span>
                      </div>
                    </div>
                    <p className="text-xs text-white/60 italic text-center">More tokens will appear here as you receive them.</p>
                  </div>
                )}
              </div>
            </div>

            {/* Send form */}
            <div className="p-5 rounded-2xl glass animate-fade-in">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-3">
                  <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-emerald-500 to-teal-600 flex items-center justify-center text-white shadow-md">
                    <Send className="w-4 h-4" />
                  </div>
                  <div>
                    <p className="text-xs uppercase tracking-wider text-emerald-500 dark:text-emerald-400">Transfer</p>
                    <h2 className="text-lg font-semibold text-slate-900 dark:text-white">Send funds</h2>
                  </div>
                </div>
                <button onClick={() => setShowAdvanced(!showAdvanced)} className={iconBtnCls}>
                  <Sliders className="w-3.5 h-3.5" />
                </button>
              </div>

              <form onSubmit={sendTx} className="space-y-4">
                <div>
                  <label className="text-xs uppercase text-slate-500 dark:text-slate-400 mb-2 block">Recipient address</label>
                  <input value={recipient} onChange={e => setRecipient(e.target.value)} placeholder="0x..." className={`${inputCls} font-mono`} />
                </div>

                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="text-xs uppercase text-slate-500 dark:text-slate-400 mb-2 block">Amount (NBL)</label>
                    <input value={amount} onChange={e => setAmount(e.target.value)} type="number" min="0" step="any" placeholder="0.00" className={inputCls} />
                    <div className="flex gap-1 mt-1.5">
                      {[{ p: 0.25, l: '25%' }, { p: 0.5, l: '50%' }, { p: 0.75, l: '75%' }, { p: 1, l: 'Max' }].map(({ p, l }) => (
                        <button key={l} onClick={() => setQuickAmount(p)} disabled={!keyPair || parseFloat(balance) <= 0}
                          className="flex-1 px-1 py-1 rounded-lg bg-slate-200/50 dark:bg-slate-700/50 border border-slate-300/50 dark:border-slate-600/50 text-[10px] font-medium text-slate-500 dark:text-slate-400 hover:bg-slate-300/50 dark:hover:bg-slate-600/50 hover:text-slate-700 dark:hover:text-slate-200 disabled:opacity-40 disabled:cursor-not-allowed transition-colors">
                          {l}
                        </button>
                      ))}
                    </div>
                  </div>
                  <div>
                    <label className="text-xs uppercase text-slate-500 dark:text-slate-400 mb-2 block flex items-center gap-1"><Fuel className="w-3 h-3" /> Fee</label>
                    <div className="flex items-center justify-between px-3 py-2.5 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-600 dark:text-slate-300">
                      ~0.000021 NBL
                      <span className="text-xs text-slate-400 dark:text-slate-500" title="Standard gas: 21000 units × base fee">ⓘ</span>
                    </div>
                  </div>
                </div>

                {showAdvanced && (
                  <div className="p-4 rounded-xl bg-slate-100/70 dark:bg-slate-700/40 space-y-3">
                    <div>
                      <label className="text-xs text-slate-500 dark:text-slate-400 mb-1 block">Gas limit</label>
                      <input value={gasLimit} onChange={e => setGasLimit(Number(e.target.value))} type="number" step="1000" min="21000" className={inputCls} />
                    </div>
                    <div>
                      <label className="text-xs text-slate-500 dark:text-slate-400 mb-1 block">Max fee per gas (Gwei)</label>
                      <input defaultValue="1" type="number" step="0.1" min="0.1" className={inputCls} />
                    </div>
                  </div>
                )}

                <button type="submit" className="w-full flex items-center justify-center gap-2 px-4 py-3 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 text-white text-sm font-semibold shadow-lg shadow-blue-600/20 hover:opacity-90 hover:-translate-y-0.5 transition-all">
                  <Send className="w-4 h-4" /> Send transaction
                </button>

                <div className="flex items-center justify-between text-xs text-slate-400 dark:text-slate-500">
                  <span>Network: <code className="text-slate-500 dark:text-slate-400">{apiUrl}</code></span>
                  <span>Nonce: <code className="text-slate-600 dark:text-slate-300 font-bold tabular-nums">{nonce}</code></span>
                </div>
              </form>

              {status && (
                <div className={`mt-4 p-3 rounded-xl text-sm ${
                  statusType === 'success' ? 'bg-emerald-50 dark:bg-emerald-600/20 border border-emerald-200 dark:border-emerald-500/30 text-emerald-700 dark:text-emerald-300' :
                  statusType === 'error' ? 'bg-red-50 dark:bg-red-600/20 border border-red-200 dark:border-red-500/30 text-red-700 dark:text-red-300' :
                  'bg-blue-50 dark:bg-blue-600/20 border border-blue-200 dark:border-blue-500/30 text-blue-700 dark:text-blue-300'
                }`}>{status}</div>
              )}
            </div>

            {/* ─── Transaction History ─── */}
            <div className="p-5 rounded-2xl glass animate-fade-in">
              <button onClick={() => setShowHistory(!showHistory)} className="flex items-center justify-between w-full">
                <div className="flex items-center gap-3">
                  <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-violet-500 to-purple-600 flex items-center justify-center text-white shadow-md">
                    <History className="w-4 h-4" />
                  </div>
                  <div>
                    <h2 className="text-lg font-semibold text-slate-900 dark:text-white">Transaction History</h2>
                    <p className="text-xs text-slate-400 dark:text-slate-500 mt-0.5">
                      On-chain activity + local pending txs ·{' '}
                      <Link to={`/history?address=${encodeURIComponent(address)}`} className="text-violet-500 hover:underline">Full history</Link>
                    </p>
                  </div>
                  {txHistory.length > 0 && (
                    <span className="text-xs bg-blue-500/20 text-blue-600 dark:text-blue-300 px-2 py-0.5 rounded-full">{txHistory.length}</span>
                  )}
                </div>
                <span className={`text-slate-400 transition-transform ${showHistory ? 'rotate-90' : ''}`}>▸</span>
              </button>

              {showHistory && (
                <div className="mt-4 space-y-2 max-h-80 overflow-y-auto scrollbar-thin">
                  {historyLoading && (
                    <p className="text-xs text-slate-400 text-center py-2">Loading on-chain history...</p>
                  )}
                  {txHistory.length === 0 ? (
                    <p className="text-sm text-slate-500 dark:text-slate-400 italic text-center py-4">No transactions yet. Send funds to see history.</p>
                  ) : (
                    txHistory.map((tx, i) => (
                      <div key={i} className="flex items-center justify-between p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/40 hover:bg-slate-200/50 dark:hover:bg-slate-700/60 transition-colors">
                        <div className="flex items-center gap-3 min-w-0">
                          <div className={`w-2 h-2 rounded-full shrink-0 ${
                            tx.status === 'confirmed' ? 'bg-emerald-400' :
                            tx.status === 'failed' ? 'bg-red-400' :
                            'bg-amber-400 animate-pulse'
                          }`} />
                          <div className="min-w-0">
                            <div className="flex items-center gap-2">
                              <code className="text-xs font-mono text-slate-600 dark:text-slate-300">{shorten(tx.hash)}</code>
                              <span className={`text-[10px] px-1.5 py-0.5 rounded ${
                                tx.status === 'confirmed' ? 'bg-emerald-500/20 text-emerald-600 dark:text-emerald-400' :
                                tx.status === 'failed' ? 'bg-red-500/20 text-red-600 dark:text-red-400' :
                                'bg-amber-500/20 text-amber-600 dark:text-amber-400'
                              }`}>{tx.status || 'pending'}</span>
                            </div>
                            <p className="text-xs text-slate-500 dark:text-slate-400 truncate max-w-[200px]">To: {shorten(tx.to)}</p>
                          </div>
                        </div>
                        <div className="text-right shrink-0 ml-3">
                          <strong className={`text-sm ${
                            parseFloat(tx.amount) > 0 ? 'text-red-500 dark:text-red-400' : 'text-emerald-500'
                          }`}>{parseFloat(tx.amount) > 0 ? '-' : '+'}{Math.abs(tx.amount).toFixed(4)}</strong>
                          <p className="text-[10px] text-slate-500 dark:text-slate-500">{formatTime(tx.timestamp)}</p>
                        </div>
                      </div>
                    ))
                  )}
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* New Account Modal */}
      {showNewAccount && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50" onClick={() => setShowNewAccount(false)}>
          <div className="glass-strong rounded-2xl w-[90%] max-w-sm shadow-2xl animate-fade-in" onClick={e => e.stopPropagation()}>
            <div className="flex items-center justify-between p-5 border-b border-slate-200/50 dark:border-slate-700/50">
              <h3 className="text-lg font-semibold text-slate-900 dark:text-white">New Account</h3>
              <button onClick={() => setShowNewAccount(false)} className="text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-white text-xl">&times;</button>
            </div>
            <div className="p-5 space-y-4">
              <div>
                <label className="text-xs text-slate-500 dark:text-slate-400 mb-2 block">Account name (optional)</label>
                <input value={newAccountName} onChange={e => setNewAccountName(e.target.value)} placeholder="My Wallet" className={inputCls} autoFocus />
              </div>
              <button onClick={() => { generateKeyPair(newAccountName.trim() || undefined); setShowNewAccount(false) }}
                className="w-full px-4 py-3 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 text-white text-sm font-semibold hover:opacity-90 transition-all">
                Generate
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Settings Modal */}
      {showSettings && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50" onClick={() => setShowSettings(false)}>
          <div className="glass-strong rounded-2xl w-[90%] max-w-md shadow-2xl animate-fade-in" onClick={e => e.stopPropagation()}>
            <div className="flex items-center justify-between p-5 border-b border-slate-200/50 dark:border-slate-700/50">
              <h3 className="text-lg font-semibold text-slate-900 dark:text-white">Settings</h3>
              <button onClick={() => setShowSettings(false)} className="text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-white text-xl">&times;</button>
            </div>
            <div className="p-5 space-y-4">
              <div>
                <label className="text-xs text-slate-500 dark:text-slate-400 mb-2 block">RPC API URL</label>
                <input value={settingsUrl} onChange={e => setSettingsUrl(e.target.value)} className={`${inputCls} font-mono`} />
              </div>
            </div>
            <div className="flex justify-end gap-3 p-5 border-t border-slate-200/50 dark:border-slate-700/50">
              <button onClick={() => setShowSettings(false)} className="px-4 py-2 rounded-xl bg-slate-100 dark:bg-slate-700/50 text-slate-600 dark:text-slate-300 text-sm">Cancel</button>
              <button onClick={saveSettings} className="px-4 py-2 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 text-white text-sm font-semibold">Save</button>
            </div>
          </div>
        </div>
      )}
    </PageShell>
  )
}
