import { useState, useEffect } from 'react'
import { Droplets, Clock, Wallet, History, RefreshCw } from 'lucide-react'
import { Link } from 'react-router-dom'
import { getApiUrl, walletAddress } from '../lib/chain.js'
import { Page, PageHeader, Card, Field, Button, Notice, Stat, StatGrid, LiveBadge, EmptyState, inputClass } from '../components/ui.jsx'

const DRIP_AMOUNT = 10 // node caps a drip at 10 NBL (one transfer)
// Local dev: 60s cooldown (matches node RPC). Production UI can use 24h.
const COOLDOWN_MS = import.meta.env.DEV ? 60 * 1000 : 24 * 60 * 60 * 1000

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
  // Default to this browser's wallet; the faucet is for funding it.
  const [address, setAddress] = useState(() => walletAddress() || '')
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
  const [, setTick] = useState(0)

  // Node health + network info polling
  useEffect(() => {
    const poll = async () => {
      try {
        const [hr, sr, gr] = await Promise.all([
          window.fetch(`${getApiUrl()}/health`),
          window.fetch(`${getApiUrl()}/status`),
          window.fetch(`${getApiUrl()}/gas_price`),
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
      setTick((t) => t + 1)
    }, 1000)
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
      const r = await window.fetch(`${getApiUrl()}/balance/${addr}`)
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
        const res = await window.fetch(`${getApiUrl()}/faucet/request`, {
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
      setStatus(`${DRIP_AMOUNT} tokens on the way to ${address} — they arrive with the next block`)
      setStatusType('success')

      const drip = { address, amount: DRIP_AMOUNT, timestamp: now }
      const history = JSON.parse(localStorage.getItem('faucet_history') || '[]')
      const updated = [drip, ...history].slice(0, 20)
      localStorage.setItem('faucet_history', JSON.stringify(updated))
      setDrips(updated.slice(0, 10))

      setTimeout(() => fetchBalance(address), 4000)
      setTimeout(() => fetchBalance(address), 9000)
    } catch (e) {
      setStatus(`Error: ${e.message}`)
      setStatusType('error')
    } finally {
      setSubmitting(false)
    }
  }

  const cooldownRemaining = cooldownEnd ? Math.max(0, cooldownEnd - Date.now()) : 0
  const cooldownLabel = cooldownRemaining >= 3600000
    ? `${Math.floor(cooldownRemaining / 3600000)}h ${Math.floor((cooldownRemaining % 3600000) / 60000)}m`
    : cooldownRemaining >= 60000 ? `${Math.ceil(cooldownRemaining / 60000)}m` : `${Math.ceil(cooldownRemaining / 1000)}s`
  const usedRequests = Math.max(0, 10 - remaining)

  const mine = walletAddress()

  return (
    <Page width="max-w-5xl">
      <PageHeader
        icon={Droplets}
        eyebrow="Faucet"
        title="Get test NBL"
        description={`Each request sends ${DRIP_AMOUNT} NBL from the devnet funding account, included in the next block.`}
        actions={<LiveBadge online={network ? backendOnline : backendOnline ? undefined : false} label={`Block #${fmt(network?.height)}`} />}
      />

      <StatGrid>
        <Stat icon={Droplets} label="Per request" value={`${DRIP_AMOUNT} NBL`} />
        <Stat icon={Clock} label="Cooldown" value={import.meta.env.DEV ? '60 s' : '24 h'} sub="per address" />
        <Stat label="Requests left" value={`${remaining} / 10`} sub="this address, this browser" />
        <Stat label="Sent from here" value={`${fmt(totalDistributed)} NBL`} />
      </StatGrid>

      <div className="grid lg:grid-cols-[1.4fr_1fr] gap-6">
        <Card title="Request tokens" icon={Droplets}>
          <div className="space-y-4">
            <Field label="Recipient address" hint={address && address === mine ? 'Your wallet in this browser' : !mine ? <>No wallet yet — <Link to="/wallet" className="text-blue-600 dark:text-blue-300 hover:underline">create one</Link> to fill this in.</> : undefined}>
              <div className="flex gap-2">
                <input value={address} onChange={(e) => setAddress(e.target.value.trim())} placeholder="0x..." className={`${inputClass} font-mono`} />
                {mine && address !== mine && (
                  <Button variant="secondary" icon={Wallet} onClick={() => setAddress(mine)} title="Use my wallet">Mine</Button>
                )}
              </div>
            </Field>

            <div className="flex items-center justify-between rounded-xl bg-slate-50 dark:bg-slate-950/40 border border-slate-100 dark:border-slate-800 px-4 py-3">
              <div>
                <div className="text-[11px] uppercase tracking-wider text-slate-500">Current balance</div>
                <div className="font-display text-lg font-semibold tabular-nums text-slate-900 dark:text-white">
                  {balanceLoading ? '…' : balance ? `${balance.nbl} NBL` : '—'}
                </div>
              </div>
              <Button variant="ghost" size="sm" icon={RefreshCw} onClick={() => fetchBalance(address)} aria-label="Refresh balance">Refresh</Button>
            </div>

            <div>
              <div className="flex justify-between text-xs text-slate-500 mb-1.5">
                <span>Lifetime requests</span><span className="tabular-nums">{usedRequests} / 10 used</span>
              </div>
              <div className="h-1.5 rounded-full bg-slate-100 dark:bg-slate-800 overflow-hidden">
                <div className="h-full rounded-full bg-gradient-to-r from-blue-600 to-cyan-500 transition-all" style={{ width: `${(usedRequests / 10) * 100}%` }} />
              </div>
            </div>

            <Button onClick={requestTokens} loading={submitting} disabled={!!cooldownEnd || !backendOnline} icon={cooldownEnd ? Clock : Droplets} className="w-full">
              {cooldownEnd ? `Next request in ${cooldownLabel}` : `Request ${DRIP_AMOUNT} NBL`}
            </Button>
            <Notice type={statusType === 'error' ? 'err' : statusType === 'success' ? 'ok' : 'info'}>{status}</Notice>
            {!backendOnline && <Notice type="err">Node not reachable at {getApiUrl()} — start the devnet first.</Notice>}
          </div>
        </Card>

        <div className="space-y-6">
          <Card title="Recent requests" icon={History} padded>
            {drips.length === 0 ? (
              <EmptyState title="No requests yet">Requests from this browser show up here.</EmptyState>
            ) : (
              <ul className="divide-y divide-slate-100 dark:divide-slate-800 -my-2">
                {drips.map((d, i) => (
                  <li key={i} className="flex items-center justify-between py-2.5 text-sm">
                    <button type="button" onClick={() => setAddress(d.address)} className="font-mono text-xs text-slate-600 dark:text-slate-300 hover:text-blue-600">{shorten(d.address)}</button>
                    <span className="text-slate-900 dark:text-white tabular-nums">+{d.amount} NBL</span>
                    <span className="text-xs text-slate-500">{formatTime(d.timestamp)}</span>
                  </li>
                ))}
              </ul>
            )}
          </Card>
          <Card title="Other test addresses" padded>
            <div className="flex flex-col gap-1">
              {TEST_ADDRESSES.map((a) => (
                <button key={a} type="button" onClick={() => setAddress(a)} className="text-left font-mono text-xs text-slate-500 hover:text-blue-600 dark:hover:text-blue-300 truncate py-1">{a}</button>
              ))}
            </div>
          </Card>
        </div>
      </div>
    </Page>
  )
}
