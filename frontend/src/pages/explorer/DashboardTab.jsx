import { useEffect, useState } from 'react'

const API_URL = 'http://localhost:8545'

const fmt = (v) => v == null || isNaN(Number(v)) ? '--' : Number(v).toLocaleString()
const shorten = (v, s = 10, e = 8) => !v ? '--' : v.length <= s + e + 3 ? v : `${v.slice(0, s)}...${v.slice(-e)}`

export default function DashboardTab() {
  const [status, setStatus] = useState(null)
  const [mempool, setMempool] = useState(null)
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
    fetch()
    const interval = setInterval(fetch, 3000)
    return () => { active = false; clearInterval(interval) }
  }, [])

  const txs = mempool?.transactions ?? []
  const finalized = status?.finalized_height ?? 0
  const current = status?.height ?? 0
  const gap = Math.max(current - finalized, 0)

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <div className={`flex items-center gap-2 px-3 py-1.5 rounded-full text-xs ${
          error ? 'bg-red-500/10 text-red-400' : 'bg-emerald-500/10 text-emerald-400'
        }`}>
          <span className={`w-2 h-2 rounded-full ${error ? 'bg-red-400' : 'bg-emerald-400'}`} />
          {error ? 'Node offline' : 'Node connected'}
        </div>
        <span className="text-xs text-slate-400 dark:text-slate-500">{lastUpdated ? `Updated ${lastUpdated.toLocaleTimeString()}` : ''}</span>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        {[
          { label: 'Chain Height', value: fmt(current), desc: 'Current tip' },
          { label: 'Finalized Height', value: fmt(finalized), desc: 'Latest finalized block' },
          { label: 'Mempool Load', value: fmt(status?.mempool_size ?? txs.length), desc: 'Pending transactions' },
          { label: 'Finality Gap', value: fmt(gap), desc: 'Tip to finalized distance' },
        ].map(({ label, value, desc }) => (
          <div key={label} className="p-4 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40">
            <span className="text-xs uppercase tracking-wider text-slate-500 dark:text-slate-400">{label}</span>
            <div className="text-2xl font-bold text-slate-900 dark:text-white mt-2">{value}</div>
            <p className="text-xs text-slate-400 dark:text-slate-500 mt-1">{desc}</p>
          </div>
        ))}
      </div>

      <div className="grid md:grid-cols-2 gap-4">
        <div className="p-5 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40">
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Chain Snapshot</h3>
          <div className="space-y-2">
            {[
              ['RPC endpoint', API_URL],
              ['Synchronization', error ? 'Disconnected' : 'Healthy'],
              ['Pending capacity', txs.length > 0 ? `${fmt(txs.length)} tx visible` : 'Queue clear'],
            ].map(([label, value]) => (
              <div key={label} className="flex justify-between items-center p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
                <span className="text-sm text-slate-500 dark:text-slate-400">{label}</span>
                <strong className="text-sm text-slate-900 dark:text-white">{value}</strong>
              </div>
            ))}
          </div>
          {!error && (
            <div className="mt-4 p-3 rounded-xl bg-gradient-to-r from-blue-500/10 to-orange-500/10 border border-blue-500/10 text-xs text-slate-400 dark:text-slate-500">
              This view refreshes every 3 seconds.
            </div>
          )}
        </div>

        <div className="p-5 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40">
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Mempool Transactions</h3>
          {loading ? (
            <div className="text-sm text-slate-400 dark:text-slate-500">Loading...</div>
          ) : txs.length > 0 ? (
            <div className="space-y-2 max-h-64 overflow-y-auto">
              {txs.slice(0, 8).map((tx, i) => (
                <div key={i} className="flex justify-between items-center p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
                  <div>
                    <span className="text-sm text-slate-900 dark:text-white">Nonce {fmt(tx.nonce)}</span>
                    <p className="text-xs text-slate-400 dark:text-slate-500">{shorten(tx.to || tx.recipient || tx.hash)}</p>
                  </div>
                  <span className="text-xs text-slate-400 dark:text-slate-500">{fmt(tx.gas_limit ?? tx.gas ?? 0)} gas</span>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-sm text-slate-400 dark:text-slate-500">No pending transactions.</div>
          )}
        </div>
      </div>
    </div>
  )
}
