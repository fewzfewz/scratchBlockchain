import { useState, useEffect } from 'react'
import { Droplets, CheckCircle, AlertCircle, Clock, Copy, WifiOff } from 'lucide-react'

const RPC_URL = 'http://localhost:8545'
const DRIP_AMOUNT = 100
const COOLDOWN_MS = 24 * 60 * 60 * 1000

const DEMO_ADDR = '0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18'

export default function FaucetPage() {
  const [address, setAddress] = useState(DEMO_ADDR)
  const [status, setStatus] = useState('')
  const [statusType, setStatusType] = useState('')
  const [remaining, setRemaining] = useState(10)
  const [totalDistributed, setTotalDistributed] = useState(0)
  const [cooldownEnd, setCooldownEnd] = useState(null)
  const [submitting, setSubmitting] = useState(false)
  const [backendOnline, setBackendOnline] = useState(true)

  useEffect(() => {
    window.fetch(`${RPC_URL}/health`)
      .then(r => { if (r.ok) setBackendOnline(true); else setBackendOnline(false) })
      .catch(() => setBackendOnline(false))
  }, [])

  useEffect(() => {
    const saved = JSON.parse(localStorage.getItem(`faucet_${address}`) || '{}')
    if (saved.remaining !== undefined) setRemaining(saved.remaining)
    if (saved.total !== undefined) setTotalDistributed(saved.total)
    if (saved.lastRequest) updateCooldown(saved.lastRequest)
  }, [address])

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
  }

  const saveData = (addr, data) => localStorage.setItem(`faucet_${addr}`, JSON.stringify(data))

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
      setStatus(`✓ ${DRIP_AMOUNT} tokens sent to ${address}`)
      setStatusType('success')
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

  return (
    <div className="relative min-h-screen">
      <div className="fixed w-[30rem] h-[30rem] rounded-full opacity-25 pointer-events-none"
        style={{ top: '-10rem', left: '-8rem', background: 'radial-gradient(circle, rgba(14,165,233,0.44), transparent 70%)' }} />
      <div className="fixed w-[30rem] h-[30rem] rounded-full opacity-25 pointer-events-none"
        style={{ right: '-10rem', bottom: '-12rem', background: 'radial-gradient(circle, rgba(251,146,60,0.32), transparent 70%)' }} />

      <div className="relative z-10 max-w-4xl mx-auto px-4 py-8">
        <div className="grid md:grid-cols-[1.2fr_0.8fr] gap-4 p-6 rounded-2xl glass mb-6">
          <div>
            <p className="text-xs uppercase tracking-widest text-blue-400 font-medium">Scratch Blockchain</p>
            <h1 className="text-4xl md:text-5xl font-bold text-slate-900 dark:text-white mt-2" style={{ lineHeight: 0.96 }}>Nebula Faucet</h1>
            <p className="text-slate-500 dark:text-slate-400 mt-3">Fund local wallets with development tokens so you can exercise transfers, explorer updates, and governance flows without manual setup.</p>
          </div>
          <div className="space-y-3">
            <div className="p-4 rounded-xl bg-gradient-to-r from-blue-500/10 to-orange-500/10 border border-blue-500/10">
              <span className="text-xs uppercase tracking-wider text-blue-400">Drip size</span>
              <strong className="block text-xl text-white mt-1">{DRIP_AMOUNT} NBL</strong>
            </div>
            <div className="p-4 rounded-xl bg-gradient-to-r from-blue-500/10 to-orange-500/10 border border-blue-500/10">
              <span className="text-xs uppercase tracking-wider text-blue-400">Cooldown</span>
              <strong className="block text-xl text-white mt-1">24 hours</strong>
            </div>
          </div>
        </div>

        {!backendOnline && (
          <div className="mb-4 px-4 py-3 rounded-xl bg-amber-500/15 border border-amber-500/30 text-sm text-amber-400 flex items-center gap-2">
            <WifiOff className="w-4 h-4 shrink-0" />
            <span>Node not reachable at {RPC_URL}. Start the node first.</span>
          </div>
        )}

        <div className="grid md:grid-cols-[1fr_280px] gap-4">
          <div className="p-5 rounded-2xl glass animate-fade-in">
            <h2 className="text-xl font-semibold text-slate-900 dark:text-white mb-4">Request testnet funds</h2>
            <ul className="space-y-2 mb-4">
              {[
                'Each request sends 100 test tokens to one local wallet address.',
                'Addresses are limited to one request every 24 hours.',
                'Usage is capped at 10 total requests per address.',
              ].map((msg, i) => (
                <li key={i} className="text-sm text-slate-500 dark:text-slate-400 flex items-start gap-2">
                  <span className="text-blue-500 dark:text-blue-400 mt-0.5">•</span> {msg}
                </li>
              ))}
            </ul>

            <div className="mb-3">
              <label className="text-xs uppercase text-slate-500 dark:text-slate-400 mb-2 block flex items-center gap-1">
                Wallet address
                <button onClick={() => { navigator.clipboard.writeText(DEMO_ADDR); setStatus('Demo address copied'); setStatusType('success'); setTimeout(() => setStatus(''), 2000) }} className="text-blue-500 dark:text-blue-400 hover:text-blue-600 dark:hover:text-blue-300">
                  <Copy className="w-3 h-3" />
                </button>
              </label>
              <input value={address} onChange={e => setAddress(e.target.value)}
                placeholder="0x..."
                className="w-full px-3 py-3 rounded-xl bg-slate-50 dark:bg-slate-700/60 border border-slate-200 dark:border-slate-600/60 text-sm text-slate-800 dark:text-slate-200 placeholder-slate-400 dark:placeholder-slate-500 font-mono" />
            </div>

            <div className="mb-4 p-3 rounded-xl bg-blue-50 dark:bg-blue-500/10 border border-blue-200 dark:border-blue-500/20">
              <p className="text-xs text-blue-600 dark:text-blue-300">
                <strong>Demo address pre-filled.</strong> This is a test address from the governance mock data — it works out of the box with the local node.
              </p>
            </div>

            <button onClick={requestTokens} disabled={submitting || !!cooldownEnd}
              className="w-full flex items-center justify-center gap-2 px-4 py-3 rounded-xl bg-gradient-to-r from-blue-500 to-cyan-600 text-white text-sm font-medium hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed transition-all">
              {submitting ? 'Processing...' : cooldownEnd ? `Wait ${cooldownHours}h ${cooldownMinutes}m` : 'Request tokens'}
            </button>

            {status && (
              <div className={`mt-4 p-3 rounded-xl text-sm ${
                statusType === 'success' ? 'bg-emerald-50 dark:bg-emerald-600/20 border border-emerald-200 dark:border-emerald-500/30 text-emerald-700 dark:text-emerald-300' :
                statusType === 'error' ? 'bg-red-50 dark:bg-red-600/20 border border-red-200 dark:border-red-500/30 text-red-700 dark:text-red-300' :
                'bg-blue-50 dark:bg-blue-600/20 border border-blue-200 dark:border-blue-500/30 text-blue-700 dark:text-blue-300'
              }`}>
                <div className="flex items-center gap-2">
                  {statusType === 'success' ? <CheckCircle className="w-4 h-4" /> : statusType === 'error' ? <AlertCircle className="w-4 h-4" /> : <Clock className="w-4 h-4" />}
                  {status}
                </div>
              </div>
            )}

            {cooldownEnd && (
              <p className="text-xs text-slate-500 dark:text-slate-500 mt-3">
                Next request available in {cooldownHours}h {cooldownMinutes}m
              </p>
            )}
          </div>

          <div className="space-y-3">
            <div className="p-4 rounded-2xl glass">
              <span className="text-xs uppercase tracking-wider text-slate-500 dark:text-slate-400">Remaining requests</span>
              <div className="text-3xl font-bold text-slate-900 dark:text-white mt-2">{remaining}</div>
            </div>
            <div className="p-4 rounded-2xl glass">
              <span className="text-xs uppercase tracking-wider text-slate-500 dark:text-slate-400">Tokens received</span>
              <div className="text-3xl font-bold text-slate-900 dark:text-white mt-2">{totalDistributed}</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
