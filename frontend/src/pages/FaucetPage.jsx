import { useState, useEffect } from 'react'
import { Droplets, CheckCircle, AlertCircle, Clock, Copy, WifiOff, Wallet, History, RefreshCw, Activity, Zap, ShieldCheck, Gauge } from 'lucide-react'

const RPC_URL = 'http://localhost:8545'
const DRIP_AMOUNT = 100
const COOLDOWN_MS = 24 * 60 * 60 * 1000

const DEMO_ADDR = '0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18'

const TEST_ADDRESSES = [
  '0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18',
  '0x5B38Da6a701c568545dCfcB03FcB875f56beddC4',
  '0xAb8483F64d9C6d1EcF9b849Ae677dD3315835cb2',
]

const weiToNbl = (wei) => {
  const n = String(wei || '0')
  if (n === '0') return '0.0000'
  const p = n.padStart(19, '0')
  return (p.slice(0, -18) || '0') + '.' + p.slice(-18, -14)
}

const fmt = (v) => v == null || isNaN(Number(v)) ? '--' : Number(v).toLocaleString()

const formatTime = (ts) => {
  const d = new Date(ts)
  const now = Date.now()
  const diff = now - d.getTime()
  if (diff < 60000) return 'Just now'
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`
  return d.toLocaleDateString() + ' ' + d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

const shorten = (s) => s ? `${s.slice(0, 6)}...${s.slice(-4)}` : ''

export default function FaucetPage() {
  const [address, setAddress] = useState(DEMO_ADDR)
  const [status, setStatus] = useState('')
  const [statusType, setStatusType] = useState('')
  const [remaining, setRemaining] = useState(10)
  const [totalDistributed, setTotalDistributed] = useState(0)
  const [cooldownEnd, setCooldownEnd] = useState(null)
  const [lastRequest, setLastRequest] = useState(null)
  const [submitting, setSubmitting] = useState(false)
  const [backendOnline, setBackendOnline] = useState(true)
  const [balance, setBalance] = useState(null)
  const [balanceLoading, setBalanceLoading] = useState(false)
  const [drips, setDrips] = useState([])
  const [network, setNetwork] = useState(null)

  // Node health + network info polling
  useEffect(() => {
    const poll = async () => {
      try {
        const [hr, sr, gr] = await Promise.all([
          window.fetch(`${RPC_URL}/health`),
          window.fetch(`${RPC_URL}/status`),
          window.fetch(`${RPC_URL}/gas_price`),
        ])
        if (!hr.ok) throw new Error('offline')
        const sd = await sr.json()
        const gd = await gr.json()
        setBackendOnline(true)
        setNetwork({ height: sd.height, finalized: sd.finalized_height, baseFee: gd.base_fee })
      } catch { setBackendOnline(false) }
    }
    poll()
    const t = setInterval(poll, 5000)
    return () => clearInterval(t)
  }, [])

  useEffect(() => {
    const saved = JSON.parse(localStorage.getItem(`faucet_${address}`) || '{}')
    if (saved.remaining !== undefined) setRemaining(saved.remaining)
    if (saved.total !== undefined) setTotalDistributed(saved.total)
    if (saved.lastRequest) updateCooldown(saved.lastRequest)
    fetchBalance(address)
  }, [address])

  useEffect(() => {
    const history = JSON.parse(localStorage.getItem('faucet_history') || '[]')
    setDrips(history.slice(0, 10))
  }, [])

  useEffect(() => {
    if (!cooldownEnd) return
    const interval = setInterval(() => {
      if (Date.now() >= cooldownEnd) { setCooldownEnd(null); return }
    }, 60000)
    return () => clearInterval(interval)
  }, [cooldownEnd])

  const updateCooldown = (lastRequest) => {
    const remainingTime = COOLDOWN_MS - (Date.now() - lastRequest)
    setCooldownEnd(remainingTime > 0 ? lastRequest + COOLDOWN_MS : null)
    if (remainingTime <= 0) setLastRequest(null)
    else setLastRequest(lastRequest)
  }

  const saveData = (addr, data) => localStorage.setItem(`faucet_${addr}`, JSON.stringify(data))

  const fetchBalance = async (addr) => {
    if (!addr || !addr.match(/^0x[a-fA-F0-9]{40}$/)) { setBalance(null); return }
    setBalanceLoading(true)
    try {
      const r = await window.fetch(`${RPC_URL}/balance/${addr}`)
      if (r.ok) {
        const d = await r.json()
        setBalance({ nbl: weiToNbl(d.balance), nonce: d.nonce || 0 })
      } else setBalance(null)
    } catch { setBalance(null) }
    finally { setBalanceLoading(false) }
  }

  const requestTokens = async () => {
    if (!address.match(/^0x[a-fA-F0-9]{40}$/)) { setStatus('Enter a valid address (0x + 40 hex chars)'); setStatusType('error'); return }
    setSubmitting(true)
    setStatus('Requesting tokens...')
    setStatusType('info')
    try {
      if (backendOnline) {
        const res = await window.fetch(`${RPC_URL}/faucet/request`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ address }),
        })
        const data = await res.json()
        if (data.error) {
          throw new Error(data.error)
        }
      } else {
        throw new Error('Node not reachable')
      }
      const newRemaining = remaining - 1
      const newTotal = totalDistributed + DRIP_AMOUNT
      setRemaining(newRemaining)
      setTotalDistributed(newTotal)
      const now = Date.now()
      saveData(address, { remaining: newRemaining, total: newTotal, lastRequest: now })
      setCooldownEnd(now + COOLDOWN_MS)
      setLastRequest(now)
      setStatus(`${DRIP_AMOUNT} tokens sent to ${address}`)
      setStatusType('success')

      const drip = { address, amount: DRIP_AMOUNT, timestamp: now }
      const history = JSON.parse(localStorage.getItem('faucet_history') || '[]')
      const updated = [drip, ...history].slice(0, 20)
      localStorage.setItem('faucet_history', JSON.stringify(updated))
      setDrips(updated.slice(0, 10))

      setTimeout(() => fetchBalance(address), 1500)
    } catch (e) {
      setStatus(`Error: ${e.message}`)
      setStatusType('error')
    } finally {
      setSubmitting(false)
    }
  }

  const cooldownRemaining = cooldownEnd ? Math.max(0, cooldownEnd - Date.now()) : 0
  const cooldownHours = Math.floor(cooldownRemaining / (60 * 60 * 1000))
  const cooldownMinutes = Math.floor((cooldownRemaining % (60 * 60 * 1000)) / (60 * 1000))
  const usedRequests = Math.max(0, 10 - remaining)

  const heroStats = [
    { icon: Droplets, label: 'Drip size', value: `${DRIP_AMOUNT} NBL`, sub: 'Per faucet request', chip: 'from-blue-500 to-cyan-600' },
    { icon: Clock, label: 'Cooldown', value: '24 hours', sub: 'Between requests', chip: 'from-amber-500 to-orange-600' },
    { icon: ShieldCheck, label: 'Requests left', value: `${remaining} / 10`, sub: 'Per address lifetime', chip: 'from-emerald-500 to-teal-600' },
    { icon: Zap, label: 'Total distributed', value: `${fmt(totalDistributed)} NBL`, sub: 'From this browser', chip: 'from-violet-500 to-purple-600' },
  ]

  return (
    <div className="relative min-h-screen overflow-hidden">
      {/* Aurora blobs + grid backdrop */}
      <div className="absolute -top-40 -left-40 w-[38rem] h-[38rem] rounded-full opacity-25 animate-float pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(14,165,233,0.45), transparent 70%)' }} />
      <div className="absolute top-40 -right-40 w-[34rem] h-[34rem] rounded-full opacity-20 animate-float-alt pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(251,146,60,0.35), transparent 70%)' }} />
      <div className="absolute inset-0 bg-grid pointer-events-none" />

      <div className="relative z-10 max-w-5xl mx-auto px-4 py-8 animate-fade-in">
        {/* ── Header ─────────────────────────────────────────────── */}
        <div className="text-center mb-8">
          <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full glass text-xs font-medium text-slate-600 dark:text-slate-300 mb-5">
            <span className={`w-2 h-2 rounded-full ${backendOnline ? 'bg-emerald-400 animate-pulse-dot' : 'bg-red-400'}`} />
            {backendOnline ? 'Faucet online' : 'Node offline'}
            <span className="text-slate-400 dark:text-slate-500">· {RPC_URL}</span>
          </div>

          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-blue-600 to-cyan-600 flex items-center justify-center text-white shadow-lg shadow-blue-600/25">
              <Droplets className="w-6 h-6" />
            </div>
            <h1 className="text-4xl md:text-5xl font-bold tracking-tight text-slate-900 dark:text-white">
              Nebula
              <span className="bg-gradient-to-r from-blue-500 via-cyan-500 to-violet-500 bg-clip-text text-transparent"> Faucet</span>
            </h1>
          </div>
          <p className="text-slate-500 dark:text-slate-400 max-w-xl mx-auto">
            Fund local wallets with development tokens so you can exercise transfers, explorer updates, and governance flows without manual setup.
          </p>

          {/* Network strip */}
          {network && (
            <div className="flex items-center justify-center flex-wrap gap-2 mt-5 text-xs">
              <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full glass-strong font-mono text-slate-700 dark:text-slate-200">
                <Activity className="w-3.5 h-3.5 text-blue-500 dark:text-cyan-400" /> Tip #{fmt(network.height)}
              </span>
              <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full glass-strong font-mono text-slate-700 dark:text-slate-200">
                <Gauge className="w-3.5 h-3.5 text-emerald-500" /> Base fee {fmt(network.baseFee)}
              </span>
              <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full glass-strong font-mono text-slate-700 dark:text-slate-200">
                <ShieldCheck className="w-3.5 h-3.5 text-violet-500" /> Finalized #{fmt(network.finalized)}
              </span>
            </div>
          )}
        </div>

        {/* ── Hero stats ─────────────────────────────────────────── */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-6">
          {heroStats.map(({ icon: Icon, label, value, sub, chip }) => (
            <div key={label} className="p-4 rounded-2xl glass-strong hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all">
              <div className="flex items-center gap-2 mb-3">
                <div className={`w-8 h-8 rounded-lg bg-gradient-to-br ${chip} flex items-center justify-center text-white shadow-md`}>
                  <Icon className="w-4 h-4" />
                </div>
                <span className="text-[11px] uppercase tracking-wider text-slate-500 dark:text-slate-400">{label}</span>
              </div>
              <div className="text-2xl font-bold text-slate-900 dark:text-white tabular-nums">{value}</div>
              <p className="text-xs text-slate-400 dark:text-slate-500 mt-1">{sub}</p>
            </div>
          ))}
        </div>

        {!backendOnline && (
          <div className="mb-6 px-4 py-3 rounded-xl bg-amber-500/15 border border-amber-500/30 text-sm text-amber-400 flex items-center gap-2">
            <WifiOff className="w-4 h-4 shrink-0" />
            <span>Node not reachable at {RPC_URL}. Start the testnet first — the faucet credits the node's state trie directly.</span>
          </div>
        )}

        <div className="grid lg:grid-cols-[1fr_320px] gap-4">
          {/* ── Request card ─────────────────────────────────────── */}
          <div className="p-5 rounded-2xl glass-strong animate-fade-in">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-blue-500 to-cyan-600 flex items-center justify-center text-white shadow-md">
                <Droplets className="w-4 h-4" />
              </div>
              <div>
                <h2 className="text-lg font-semibold text-slate-900 dark:text-white">Request testnet funds</h2>
                <p className="text-xs text-slate-400 dark:text-slate-500 mt-0.5">One drip per address, 100 NBL each.</p>
              </div>
            </div>

            {/* Remaining progress */}
            <div className="mb-5">
              <div className="flex items-center justify-between text-xs mb-1.5">
                <span className="text-slate-500 dark:text-slate-400">Lifetime requests</span>
                <span className="text-slate-600 dark:text-slate-300 font-medium tabular-nums">{usedRequests} / 10 used</span>
              </div>
              <div className="h-2 rounded-full bg-slate-200 dark:bg-slate-700/50 overflow-hidden">
                <div className="h-full rounded-full bg-gradient-to-r from-blue-500 to-cyan-500 transition-all"
                  style={{ width: `${(usedRequests / 10) * 100}%` }} />
              </div>
            </div>

            <div className="mb-3">
              <label className="text-xs uppercase text-slate-500 dark:text-slate-400 mb-2 block flex items-center gap-1">
                Wallet address
                <button onClick={() => { navigator.clipboard.writeText(DEMO_ADDR); setStatus('Demo address copied'); setStatusType('success'); setTimeout(() => setStatus(''), 2000) }} className="text-blue-500 dark:text-blue-400 hover:text-blue-600 dark:hover:text-blue-300">
                  <Copy className="w-3 h-3" />
                </button>
              </label>
              <input value={address} onChange={e => setAddress(e.target.value)}
                placeholder="0x..."
                className="w-full px-3 py-3 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-800 dark:text-slate-200 placeholder-slate-400 dark:placeholder-slate-500 font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/40" />
            </div>

            {/* Quick address chips */}
            <div className="mb-4">
              <span className="text-[10px] uppercase tracking-wider text-slate-400 dark:text-slate-500 block mb-1.5">Quick fill</span>
              <div className="flex flex-wrap gap-1.5">
                {TEST_ADDRESSES.map((a) => (
                  <button key={a} onClick={() => setAddress(a)}
                    className={`px-2.5 py-1 rounded-lg text-[11px] font-mono transition-all ${
                      address === a
                        ? 'bg-blue-500/20 text-blue-600 dark:text-blue-300 border border-blue-500/30'
                        : 'bg-slate-200/50 dark:bg-slate-700/50 text-slate-500 dark:text-slate-400 border border-slate-300/50 dark:border-slate-600/50 hover:bg-slate-300/50 dark:hover:bg-slate-600/50'
                    }`}>
                    {shorten(a)}
                  </button>
                ))}
              </div>
            </div>

            <button onClick={requestTokens} disabled={submitting || !!cooldownEnd}
              className="w-full flex items-center justify-center gap-2 px-4 py-3 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 text-white text-sm font-semibold shadow-lg shadow-blue-600/20 hover:opacity-90 hover:-translate-y-0.5 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:translate-y-0 transition-all">
              {submitting ? <><RefreshCw className="w-4 h-4 animate-spin" /> Processing...</> : cooldownEnd ? <><Clock className="w-4 h-4" /> Wait {cooldownHours}h {cooldownMinutes}m</> : <><Droplets className="w-4 h-4" /> Request {DRIP_AMOUNT} NBL</>}
            </button>

            {status && (
              <div className={`mt-4 p-3 rounded-xl text-sm ${
                statusType === 'success' ? 'bg-emerald-50 dark:bg-emerald-600/20 border border-emerald-200 dark:border-emerald-500/30 text-emerald-700 dark:text-emerald-300' :
                statusType === 'error' ? 'bg-red-50 dark:bg-red-600/20 border border-red-200 dark:border-red-500/30 text-red-700 dark:text-red-300' :
                'bg-blue-50 dark:bg-blue-600/20 border border-blue-200 dark:border-blue-500/30 text-blue-700 dark:text-blue-300'
              }`}>
                <div className="flex items-center gap-2">
                  {statusType === 'success' ? <CheckCircle className="w-4 h-4 shrink-0" /> : statusType === 'error' ? <AlertCircle className="w-4 h-4 shrink-0" /> : <Clock className="w-4 h-4 shrink-0" />}
                  <span className="break-all">{status}</span>
                </div>
              </div>
            )}

            {cooldownEnd && (
              <p className="text-xs text-slate-500 dark:text-slate-500 mt-3">
                Next request available in {cooldownHours}h {cooldownMinutes}m
              </p>
            )}

            {/* Balance check */}
            <div className="mt-5 p-4 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
              <div className="flex items-center justify-between mb-2">
                <span className="flex items-center gap-1.5 text-xs uppercase tracking-wider text-slate-500 dark:text-slate-400">
                  <Wallet className="w-3.5 h-3.5" /> On-chain balance
                </span>
                <button onClick={() => fetchBalance(address)} className="p-1.5 rounded-lg hover:bg-slate-200/60 dark:hover:bg-slate-600/50 text-slate-400 dark:text-slate-500" title="Refresh balance">
                  <RefreshCw className={`w-3.5 h-3.5 ${balanceLoading ? 'animate-spin' : ''}`} />
                </button>
              </div>
              {balanceLoading ? (
                <div className="h-8 rounded-lg bg-slate-200/60 dark:bg-slate-600/40 animate-pulse" />
              ) : balance ? (
                <div className="flex items-baseline gap-2">
                  <span className="text-2xl font-bold text-slate-900 dark:text-white tabular-nums">{balance.nbl}</span>
                  <span className="text-sm text-slate-400 dark:text-slate-500">NBL</span>
                  <span className="ml-auto text-xs text-slate-400 dark:text-slate-500 tabular-nums">nonce {balance.nonce}</span>
                </div>
              ) : (
                <p className="text-sm text-slate-400 dark:text-slate-500">No balance for this address yet.</p>
              )}
            </div>
          </div>

          {/* ── Right column ─────────────────────────────────────── */}
          <div className="space-y-4">
            <div className="p-4 rounded-2xl glass-strong">
              <div className="flex items-center justify-between mb-2">
                <span className="flex items-center gap-1.5 text-xs uppercase tracking-wider text-slate-500 dark:text-slate-400">
                  <History className="w-3.5 h-3.5" /> Drip history
                </span>
                <span className="text-[10px] text-slate-400 dark:text-slate-500">Last 10</span>
              </div>
              {drips.length > 0 ? (
                <div className="space-y-2 max-h-64 overflow-y-auto scrollbar-thin">
                  {drips.map((d, i) => (
                    <div key={i} className="flex items-center justify-between p-2.5 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
                      <div className="min-w-0">
                        <p className="text-xs font-mono text-slate-800 dark:text-slate-200 truncate">{shorten(d.address)}</p>
                        <p className="text-[10px] text-slate-400 dark:text-slate-500">{formatTime(d.timestamp)}</p>
                      </div>
                      <span className="text-xs font-bold text-emerald-500 dark:text-emerald-400 shrink-0 ml-2">+{d.amount} NBL</span>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-sm text-slate-400 dark:text-slate-500 py-4 text-center">No drips yet.</p>
              )}
            </div>

            <div className="p-4 rounded-2xl glass-strong">
              <span className="text-xs uppercase tracking-wider text-slate-500 dark:text-slate-400">Last request</span>
              <div className="text-xl font-bold text-slate-900 dark:text-white mt-1.5 tabular-nums">
                {lastRequest ? formatTime(lastRequest) : '—'}
              </div>
            </div>

            <div className="p-4 rounded-2xl bg-gradient-to-br from-blue-600/10 via-cyan-600/10 to-teal-600/10 border border-blue-500/10">
              <p className="text-xs font-semibold text-blue-600 dark:text-blue-300 mb-1.5">How it works</p>
              <ul className="space-y-1.5 text-xs text-slate-500 dark:text-slate-400">
                <li>Each request sends <strong className="text-slate-700 dark:text-slate-200">{DRIP_AMOUNT} test tokens</strong> to one local wallet address.</li>
                <li>Addresses are limited to <strong className="text-slate-700 dark:text-slate-200">one request every 24h</strong> (browser-side) and the node enforces a 60s server cooldown.</li>
                <li>Usage is capped at <strong className="text-slate-700 dark:text-slate-200">10 total requests</strong> per address.</li>
              </ul>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
