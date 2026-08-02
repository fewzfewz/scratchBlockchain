import { useEffect, useState } from 'react'
import { Activity, ShieldCheck, Layers, Gauge, Blocks, Inbox, RefreshCw } from 'lucide-react'

const API_URL = 'http://localhost:8545'

const fmt = (v) => v == null || isNaN(Number(v)) ? '--' : Number(v).toLocaleString()
const shorten = (v, s = 10, e = 8) => !v ? '--' : v.length <= s + e + 3 ? v : `${v.slice(0, s)}...${v.slice(-e)}`

export default function DashboardTab() {
  const [status, setStatus] = useState(null)
  const [mempool, setMempool] = useState(null)
  const [blocks, setBlocks] = useState([])
  const [loading, setLoading] = useState(true)
  const [lastUpdated, setLastUpdated] = useState(null)
  const [error, setError] = useState('')

  useEffect(() => {
    let active = true
    const fetch = async () => {
      try {
        const [sr, mr] = await Promise.all([
          window.fetch(`${API_URL}/status`),
          window.fetch(`${API_URL}/mempool`),
        ])
        if (!sr.ok || !mr.ok) throw new Error('Node returned unexpected response')
        const [sd, md] = await Promise.all([sr.json(), mr.json()])
        if (!active) return
        setStatus(sd); setMempool(md); setLastUpdated(new Date()); setError('')
      } catch { if (active) setError('Unable to reach the local node at http://localhost:8545.') }
      finally { if (active) setLoading(false) }
    }
    const fetchBlocks = async () => {
      try {
        const res = await window.fetch(`${API_URL}/block/latest`)
        if (!res.ok) throw new Error('offline')
        const { block } = await res.json()
        if (!block) return
        const latest = block.header.slot
        const from = Math.max(latest - 5, 0)
        const reqs = []
        for (let s = latest; s >= from; s--) {
          reqs.push(
            window.fetch(`${API_URL}/block/${s}`)
              .then((r) => r.json())
              .then((d) => d.block)
              .catch(() => null),
          )
        }
        const found = (await Promise.all(reqs)).filter(Boolean)
        if (active && found.length) setBlocks(found)
      } catch { /* keep last known blocks */ }
    }
    fetch()
    fetchBlocks()
    const interval = setInterval(() => { fetch(); fetchBlocks() }, 3000)
    return () => { active = false; clearInterval(interval) }
  }, [])

  const txs = mempool?.transactions ?? []
  const finalized = status?.finalized_height ?? 0
  const current = status?.height ?? 0
  const gap = Math.max(current - finalized, 0)

  const stats = [
    { label: 'Chain Height', value: fmt(current), desc: 'Current tip', icon: Activity, chip: 'from-blue-500 to-cyan-600' },
    { label: 'Finalized Height', value: fmt(finalized), desc: 'Latest finalized block', icon: ShieldCheck, chip: 'from-emerald-500 to-teal-600' },
    { label: 'Mempool Load', value: fmt(status?.mempool_size ?? txs.length), desc: 'Pending transactions', icon: Inbox, chip: 'from-amber-500 to-orange-600' },
    { label: 'Finality Gap', value: fmt(gap), desc: 'Tip to finalized distance', icon: Gauge, chip: 'from-violet-500 to-purple-600' },
  ]

  return (
    <div className="space-y-4">
      {/* Status strip */}
      <div className="flex items-center justify-between flex-wrap gap-2">
        <div className={`flex items-center gap-2 px-3 py-1.5 rounded-full text-xs ${
          error ? 'bg-red-500/10 text-red-400' : 'bg-emerald-500/10 text-emerald-400'
        }`}>
          <span className={`w-2 h-2 rounded-full ${error ? 'bg-red-400' : 'bg-emerald-400 animate-pulse-dot'}`} />
          {error ? 'Node offline' : 'Node connected'}
        </div>
        <span className="inline-flex items-center gap-1.5 text-xs text-slate-400 dark:text-slate-500">
          <RefreshCw className="w-3 h-3" />
          {lastUpdated ? `Updated ${lastUpdated.toLocaleTimeString()} · auto-refresh 3s` : 'Connecting…'}
        </span>
      </div>

      {/* Stat cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
        {stats.map(({ label, value, desc, icon: Icon, chip }) => (
          <div key={label} className="p-4 rounded-2xl glass-strong hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all">
            <div className="flex items-center gap-2 mb-3">
              <div className={`w-8 h-8 rounded-lg bg-gradient-to-br ${chip} flex items-center justify-center text-white shadow-md`}>
                <Icon className="w-4 h-4" />
              </div>
              <span className="text-[11px] uppercase tracking-wider text-slate-500 dark:text-slate-400">{label}</span>
            </div>
            <div className="text-2xl font-bold text-slate-900 dark:text-white tabular-nums">{value}</div>
            <p className="text-xs text-slate-400 dark:text-slate-500 mt-1">{desc}</p>
          </div>
        ))}
      </div>

      <div className="grid lg:grid-cols-2 gap-4">
        {/* Chain snapshot */}
        <div className="p-5 rounded-2xl glass-strong">
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Chain Snapshot</h3>
          <div className="space-y-2">
            {[
              ['RPC endpoint', API_URL],
              ['Synchronization', error ? 'Disconnected' : 'Healthy'],
              ['Pending capacity', txs.length > 0 ? `${fmt(txs.length)} tx visible` : 'Queue clear'],
              ['Connected peers', fmt(status?.peer_count)],
            ].map(([label, value]) => (
              <div key={label} className="flex justify-between items-center p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
                <span className="text-sm text-slate-500 dark:text-slate-400">{label}</span>
                <strong className="text-sm text-slate-900 dark:text-white tabular-nums">{value}</strong>
              </div>
            ))}
          </div>
          {!error && (
            <div className="mt-4 p-3 rounded-xl bg-gradient-to-r from-blue-500/10 to-cyan-500/10 border border-blue-500/10 text-xs text-slate-400 dark:text-slate-500">
              BFT consensus finalizing blocks every second.
            </div>
          )}
        </div>

        {/* Mempool */}
        <div className="p-5 rounded-2xl glass-strong">
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Mempool Transactions</h3>
          {loading ? (
            <div className="space-y-2">
              {[0, 1, 2].map((i) => (
                <div key={i} className="h-14 rounded-xl bg-slate-100/60 dark:bg-slate-700/30 animate-pulse" />
              ))}
            </div>
          ) : txs.length > 0 ? (
            <div className="space-y-2 max-h-64 overflow-y-auto scrollbar-thin">
              {txs.slice(0, 8).map((tx, i) => (
                <div key={i} className="flex justify-between items-center p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
                  <div className="min-w-0">
                    <span className="text-sm text-slate-900 dark:text-white">Nonce {fmt(tx.nonce)}</span>
                    <p className="text-xs text-slate-400 dark:text-slate-500 truncate">{shorten(tx.to || tx.recipient || tx.hash)}</p>
                  </div>
                  <span className="text-xs text-slate-400 dark:text-slate-500 whitespace-nowrap ml-2">{fmt(tx.gas_limit ?? tx.gas ?? 0)} gas</span>
                </div>
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-8 text-center">
              <Inbox className="w-8 h-8 text-slate-300 dark:text-slate-600 mb-2" />
              <p className="text-sm text-slate-400 dark:text-slate-500">No pending transactions.</p>
              <p className="text-xs text-slate-400 dark:text-slate-500 mt-1">Submit one from the wallet or faucet.</p>
            </div>
          )}
        </div>
      </div>

      {/* Recent blocks */}
      <div className="p-5 rounded-2xl glass-strong">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Blocks className="w-4 h-4 text-blue-500 dark:text-cyan-400" />
            <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300">Recent Blocks</h3>
          </div>
          <span className="text-xs text-slate-400 dark:text-slate-500">Live from the tip</span>
        </div>
        {blocks.length > 0 ? (
          <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-3">
            {blocks.map((b) => (
              <div key={b.header.slot} className="p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
                <div className="flex items-center justify-between mb-2">
                  <span className="font-mono text-sm font-bold text-slate-900 dark:text-white">#{fmt(b.header.slot)}</span>
                  <span className="text-[11px] text-slate-400 dark:text-slate-500">{b.extrinsics?.length ?? 0} txns</span>
                </div>
                <div className="grid grid-cols-2 gap-2">
                  <div>
                    <div className="text-[10px] uppercase tracking-wider text-slate-400 dark:text-slate-500">Base fee</div>
                    <div className="text-sm font-semibold text-slate-800 dark:text-slate-200 tabular-nums">{fmt(b.header.base_fee)}</div>
                  </div>
                  <div>
                    <div className="text-[10px] uppercase tracking-wider text-slate-400 dark:text-slate-500">Gas used</div>
                    <div className="text-sm font-semibold text-slate-800 dark:text-slate-200 tabular-nums">{fmt(b.header.gas_used)}</div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="p-6 text-center text-sm text-slate-400 dark:text-slate-500">
            {error ? 'Node unreachable — start the testnet to see live blocks.' : 'Fetching blocks…'}
          </div>
        )}
      </div>
    </div>
  )
}
