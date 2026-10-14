import { useState, useEffect, useCallback } from 'react'
import { Link } from 'react-router-dom'
import nacl from 'tweetnacl'
import { Wallet, Key, Eye, EyeOff, Copy, RefreshCw, Trash2, Send, Settings, Sliders, Droplets, Plus, History, ArrowRight } from 'lucide-react'
import { Page, PageHeader, Card, Field, Button, Notice, LiveBadge, Badge, EmptyState, Modal, inputClass, buttonClass } from '../components/ui.jsx'
import { signTx, txHash } from '../lib/chain.js'

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
  return new Uint8Array(hash.slice(-20))
}

const normAddr = (a) => (a && a.startsWith('0x') ? a : '0x' + (a || ''))

const inputCls = inputClass
const iconBtnCls = 'shrink-0 h-11 w-11 inline-flex items-center justify-center rounded-xl border border-slate-200 dark:border-slate-800 text-slate-500 hover:text-slate-900 dark:hover:text-white hover:bg-slate-50 dark:hover:bg-slate-800 transition-colors'

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
  const [recipient, setRecipient] = useState('')
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
  const [showHistory] = useState(true)
  const [historyLoading, setHistoryLoading] = useState(false)

  // Tokens tab

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
      const r = await window.fetch(`${apiUrl}/txs/${address.replace(/^0x/, '')}?limit=25`)
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
    setKeyPair(kp); setPubKey(acc.pub); setAddress(normAddr(acc.addr)); setPrivKey(acc.priv)
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
      const addr = '0x' + toHex(addrBytes)
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
        setKeyPair(kp); setPubKey(acc.pub); setAddress(normAddr(acc.addr)); setPrivKey(acc.priv)
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
      if (!addr) { const addrBytes = await addrFromPub(kp.publicKey); addr = '0x' + toHex(addrBytes); localStorage.setItem('nebula_wallet_addr', addr) }
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
      const r = await window.fetch(`${apiUrl}/balance/${address.replace(/^0x/, '')}`)
      if (r.ok) { const d = await r.json(); setNonce(d.nonce || 0); return d.nonce || 0 }
    } catch { /* ignore */ }
    return 0
  }

  const updateBalance = useCallback(async () => {
    if (!keyPair || !address) { setBalance('0.0000'); return }
    try {
      const r = await window.fetch(`${apiUrl}/balance/${address.replace(/^0x/, '')}`)
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
      sender: Array.from(fromHex(address.replace(/^0x/, ''))),
      to: Array.from(fromHex(to.replace('0x', ''))),
      nonce: nonceVal, value: amountInWei,
      gas_limit: gasLimit, max_fee_per_gas: 1e9, max_priority_fee_per_gas: 1e9,
      payload: [], chain_id: 1, signature: [],
    }
  }

  const addTxToHistory = (hash, toAddr, amt) => {
    const entry = { hash, to: toAddr, amount: amt, timestamp: Date.now(), status: 'pending' }
    const updated = [entry, ...txHistory].slice(0, 50)
    setTxHistory(updated)
    localStorage.setItem(`nebula_tx_history_${address}`, JSON.stringify(updated))
    const pollReceipt = async (attempt = 0) => {
      if (attempt >= 12) return
      try {
        const r = await window.fetch(`${apiUrl}/tx/${hash.replace(/^0x/, '')}`)
        if (r.ok) {
          const d = await r.json()
          if (d.receipt) {
            const finalStatus = d.receipt.success === true ? 'confirmed' : 'failed'
            const hist = JSON.parse(localStorage.getItem(`nebula_tx_history_${address}`) || '[]')
            const updatedHist = hist.map(t => t.hash === hash ? { ...t, status: finalStatus } : t)
            localStorage.setItem(`nebula_tx_history_${address}`, JSON.stringify(updatedHist))
            setTxHistory(updatedHist)
            if (finalStatus === 'confirmed') updateBalance()
            return
          }
        }
      } catch {}
      setTimeout(() => pollReceipt(attempt + 1), 2500)
    }
    setTimeout(() => pollReceipt(0), 3000)
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
      const tx = await signTx(createTx(recipient, amt, n), keyPair)
      showMsg('Sending...', 'info')
      const r = await window.fetch(`${apiUrl}/submit_tx`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(tx),
      })
      const text = await r.text()
      let data
      try { data = JSON.parse(text) } catch { data = null }
      if (data?.status?.includes('already in mempool')) {
        const hash = data.hash || toHex(await txHash(tx))
        showMsg(`Transaction already pending (${hash.slice(0, 8)}...)`, 'info')
        addTxToHistory(hash, recipient, amt)
        return
      }
      if (data?.status?.startsWith('error')) {
        showMsg('Transaction failed: ' + data.status, 'error')
        return
      }
      if (!r.ok) {
        showMsg('Transaction failed: ' + text, 'error')
        return
      }
      const hash = data?.hash || text.replace(/^"|"$/g, '')
      if (!hash) {
        showMsg('Transaction failed: no hash returned', 'error')
        return
      }
      showMsg(`Transaction submitted! Hash: ${hash.slice(0, 8)}...${hash.slice(-8)}`, 'success')
      setAmount('')
      setNonce(n + 1)
      addTxToHistory(hash, recipient, amt)
      setTimeout(() => { updateBalance(); fetchNonce() }, 3000)
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

  const activeName = accounts[activeAccountIdx]?.name
  const online = nodeStatus === 'connected' ? true : nodeStatus === 'checking' ? undefined : false
  const statusKind = statusType === 'success' ? 'ok' : statusType === 'error' ? 'err' : 'info'

  return (
    <Page width="max-w-6xl">
      <PageHeader
        icon={Wallet}
        eyebrow="Wallet"
        title={keyPair ? activeName || 'Your wallet' : 'Wallet'}
        description={keyPair ? undefined : 'Ed25519 keys generated and kept in this browser. Development use only.'}
        actions={
          <>
            <LiveBadge online={online} label="Node connected" />
            <Link to="/faucet" className={buttonClass('secondary', 'sm')}><Droplets className="w-4 h-4" /> Get test NBL</Link>
            {keyPair && <Button variant="secondary" size="sm" icon={Plus} onClick={() => { setNewAccountName(''); setShowNewAccount(true) }}>New account</Button>}
            <button type="button" onClick={() => { setSettingsUrl(apiUrl); setShowSettings(true) }} className="h-9 w-9 inline-flex items-center justify-center rounded-xl border border-slate-200 dark:border-slate-800 text-slate-500 hover:text-slate-900 dark:hover:text-white" aria-label="Settings"><Settings className="w-4 h-4" /></button>
          </>
        }
      >
        {keyPair && (
          <div className="mt-2 flex flex-wrap items-center gap-2">
            <button type="button" onClick={() => copy(address, 'Address copied')} className="inline-flex items-center gap-1.5 font-mono text-sm text-slate-500 hover:text-slate-900 dark:hover:text-white" title="Copy address">
              {address} <Copy className="w-3.5 h-3.5" />
            </button>
          </div>
        )}
      </PageHeader>

      {accounts.length > 1 && (
        <div className="flex flex-wrap gap-2 mb-6" role="tablist" aria-label="Accounts">
          {accounts.map((acc, idx) => (
            <button key={idx} type="button" onClick={() => switchAccount(idx)} aria-selected={idx === activeAccountIdx}
              className={`h-8 px-3 rounded-lg text-xs font-medium border transition ${idx === activeAccountIdx ? 'border-blue-500/50 bg-blue-500/10 text-blue-700 dark:text-blue-300' : 'border-slate-200 dark:border-slate-800 text-slate-500 hover:text-slate-900 dark:hover:text-white'}`}>
              {acc.name || `Account ${idx + 1}`}
            </button>
          ))}
        </div>
      )}

      {!keyPair ? (
        <Card className="max-w-xl">
          <div className="text-center py-6">
            <div className="mx-auto w-12 h-12 rounded-2xl bg-gradient-to-br from-blue-600 to-cyan-600 text-white flex items-center justify-center"><Wallet className="w-6 h-6" /></div>
            <h2 className="mt-4 text-xl font-semibold text-slate-900 dark:text-white">Create your wallet</h2>
            <p className="mt-2 text-sm text-slate-500 max-w-sm mx-auto">A new Ed25519 key pair is generated in your browser. Then get test NBL from the faucet and start sending.</p>
            <Button icon={Plus} className="mt-6" onClick={() => { setNewAccountName(''); setShowNewAccount(true) }}>Generate a new wallet</Button>
          </div>
          <Notice type="ok" className="mt-2">{status}</Notice>
        </Card>
      ) : (
        <div className="grid lg:grid-cols-[1.3fr_1fr] gap-6">
          <div className="space-y-6">
            <section className="rounded-2xl p-6 bg-gradient-to-br from-blue-600 via-blue-700 to-cyan-700 text-white shadow-xl shadow-blue-900/20">
              <div className="flex items-center justify-between">
                <span className="text-xs uppercase tracking-wider text-white/70">Balance</span>
                <button type="button" onClick={() => { updateBalance(); fetchNonce(); showMsg('Balance refreshed', 'success') }} className="p-2 rounded-lg bg-white/10 hover:bg-white/20 transition" aria-label="Refresh balance"><RefreshCw className="w-4 h-4" /></button>
              </div>
              <div className="mt-2 flex items-baseline gap-2">
                <span className="font-display text-4xl sm:text-5xl font-semibold tabular-nums">{balance}</span>
                <span className="text-white/70">NBL</span>
              </div>
              <div className="mt-5 pt-4 border-t border-white/15 flex flex-wrap gap-x-8 gap-y-2 text-sm">
                <span><span className="text-white/60">Transactions sent</span> <span className="font-semibold tabular-nums">{nonce}</span></span>
                <span><span className="text-white/60">Network fee</span> <span className="font-semibold">~{FEE_NBL} NBL</span></span>
              </div>
            </section>

            <Card title="Send NBL" icon={Send} actions={
              <button type="button" onClick={() => setShowAdvanced(!showAdvanced)} aria-expanded={showAdvanced} className="inline-flex items-center gap-1.5 text-xs font-medium text-slate-500 hover:text-slate-900 dark:hover:text-white"><Sliders className="w-3.5 h-3.5" /> Advanced</button>
            }>
              <form onSubmit={sendTx} className="space-y-4">
                <Field label="Recipient address">
                  <input value={recipient} onChange={(e) => setRecipient(e.target.value)} placeholder="0x..." className={`${inputCls} font-mono`} />
                </Field>
                <Field label="Amount (NBL)" hint={`Available: ${balance} NBL · fee ~${FEE_NBL} NBL`}>
                  <div className="flex gap-2">
                    <input value={amount} onChange={(e) => setAmount(e.target.value)} type="number" min="0" step="any" placeholder="0.00" className={inputCls} />
                    <div className="flex gap-1">
                      {[{ p: 0.25, l: '25%' }, { p: 0.5, l: '50%' }, { p: 1, l: 'Max' }].map(({ p, l }) => (
                        <button key={l} type="button" onClick={() => setQuickAmount(p)} disabled={parseFloat(balance) <= 0}
                          className="h-11 px-3 rounded-xl border border-slate-200 dark:border-slate-800 text-xs font-medium text-slate-500 hover:text-slate-900 dark:hover:text-white disabled:opacity-40">{l}</button>
                      ))}
                    </div>
                  </div>
                </Field>
                {showAdvanced && (
                  <div className="grid sm:grid-cols-2 gap-3 p-4 rounded-xl bg-slate-50 dark:bg-slate-950/40 border border-slate-100 dark:border-slate-800">
                    <Field label="Gas limit"><input value={gasLimit} onChange={(e) => setGasLimit(Number(e.target.value))} type="number" step="1000" min="21000" className={inputCls} /></Field>
                    <Field label="Max fee per gas (gwei)"><input defaultValue="1" type="number" step="0.1" min="0.1" className={inputCls} /></Field>
                  </div>
                )}
                <Button type="submit" icon={Send} className="w-full">Send transaction</Button>
              </form>
              <Notice type={statusKind} className="mt-4">{status}</Notice>
            </Card>
          </div>

          <div className="space-y-6">
            <Card title="Recent activity" icon={History} actions={
              <Link to={`/history?address=${encodeURIComponent(address)}`} className="inline-flex items-center gap-1 text-xs font-medium text-blue-600 dark:text-blue-300 hover:underline">All <ArrowRight className="w-3 h-3" /></Link>
            }>
              {historyLoading && txHistory.length === 0 ? (
                <p className="text-sm text-slate-500 py-4 text-center">Loading…</p>
              ) : txHistory.length === 0 ? (
                <EmptyState icon={History} title="No transactions yet">Your sends and receipts appear here.</EmptyState>
              ) : (
                <ul className="divide-y divide-slate-100 dark:divide-slate-800 -my-2 max-h-80 overflow-y-auto scrollbar-thin">
                  {txHistory.slice(0, 12).map((tx, i) => (
                    <li key={i} className="flex items-center justify-between gap-3 py-2.5">
                      <div className="min-w-0">
                        <div className="font-mono text-xs text-slate-600 dark:text-slate-300 truncate">to {shorten(tx.to)}</div>
                        <div className="text-[11px] text-slate-400">{formatTime(tx.timestamp)}</div>
                      </div>
                      <div className="text-right shrink-0">
                        <div className="text-sm font-semibold tabular-nums text-slate-900 dark:text-white">{parseFloat(tx.amount) > 0 ? '−' : '+'}{Math.abs(tx.amount).toFixed(4)}</div>
                        <Badge tone={tx.status === 'confirmed' ? 'green' : tx.status === 'failed' ? 'red' : 'amber'}>{tx.status || 'pending'}</Badge>
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </Card>

            <Card title="Keys" icon={Key} actions={
              <button type="button" onClick={clearWallet} className="inline-flex items-center gap-1 text-xs font-medium text-rose-600 hover:underline"><Trash2 className="w-3.5 h-3.5" /> Remove</button>
            }>
              <div className="space-y-3">
                <Field label="Address">
                  <div className="flex gap-2">
                    <input readOnly value={address} className={`${inputCls} font-mono text-xs`} aria-label="Address" />
                    <button type="button" onClick={() => copy(address, 'Address copied')} className={iconBtnCls} aria-label="Copy address"><Copy className="w-4 h-4" /></button>
                  </div>
                </Field>
                <Field label="Public key">
                  <div className="flex gap-2">
                    <input readOnly value={pubKey} className={`${inputCls} font-mono text-xs`} aria-label="Public key" />
                    <button type="button" onClick={() => copy(pubKey, 'Public key copied')} className={iconBtnCls} aria-label="Copy public key"><Copy className="w-4 h-4" /></button>
                  </div>
                </Field>
                <Field label="Private key">
                  <div className="flex gap-2">
                    <input type={showPriv ? 'text' : 'password'} readOnly value={privKey} className={`${inputCls} font-mono text-xs`} aria-label="Private key" />
                    <button type="button" onClick={() => setShowPriv(!showPriv)} className={iconBtnCls} aria-label={showPriv ? 'Hide private key' : 'Show private key'}>{showPriv ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}</button>
                    <button type="button" onClick={() => copy(privKey, 'Private key copied')} className={iconBtnCls} aria-label="Copy private key"><Copy className="w-4 h-4" /></button>
                  </div>
                </Field>
                <Notice type="warn">Keys live in this browser's local storage — fine for a devnet, never for real funds.</Notice>
              </div>
            </Card>

            <Card title="Test recipients" description="Addresses you can send to.">
              <div className="space-y-1">
                {TEST_ADDRESSES.map((addr) => (
                  <button key={addr} type="button" onClick={() => setRecipient(addr)} className="w-full flex items-center justify-between gap-2 py-1.5 text-left group">
                    <span className="font-mono text-[11px] text-slate-500 truncate group-hover:text-slate-900 dark:group-hover:text-white">{addr}</span>
                    <span className="text-xs font-medium text-blue-600 dark:text-blue-300 shrink-0">Use</span>
                  </button>
                ))}
              </div>
            </Card>
          </div>
        </div>
      )}

      {showNewAccount && (
        <Modal title="New account" onClose={() => setShowNewAccount(false)}>
          <Field label="Account name (optional)">
            <input value={newAccountName} onChange={(e) => setNewAccountName(e.target.value)} placeholder="My Wallet" className={inputCls} autoFocus />
          </Field>
          <Button className="w-full" onClick={() => { generateKeyPair(newAccountName.trim() || undefined); setShowNewAccount(false) }}>Generate</Button>
        </Modal>
      )}

      {showSettings && (
        <Modal title="Settings" onClose={() => setShowSettings(false)}
          footer={<><Button variant="ghost" size="sm" onClick={() => setShowSettings(false)}>Cancel</Button><Button size="sm" onClick={saveSettings}>Save</Button></>}>
          <Field label="Node RPC URL" hint="Every page uses this endpoint.">
            <input value={settingsUrl} onChange={(e) => setSettingsUrl(e.target.value)} className={`${inputCls} font-mono`} />
          </Field>
        </Modal>
      )}
    </Page>
  )
}
