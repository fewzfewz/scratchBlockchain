import { useEffect, useState } from 'react'

const API_URL = 'http://localhost:9933'
const shorten = (v, s = 10, e = 8) => !v ? '--' : v.length <= s + e + 3 ? v : `${v.slice(0, s)}...${v.slice(-e)}`
const fmt = (v) => v == null || isNaN(Number(v)) ? '--' : Number(v).toLocaleString()
const fmtStake = (s) => !s ? '--' : (Number(s) / 1e18).toFixed(2) + ' NBL'

export default function ValidatorsTab() {
  const [validators, setValidators] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [selected, setSelected] = useState(null)

  useEffect(() => {
    let active = true
    const fetch = async () => {
      try {
        const res = await window.fetch(`${API_URL}/validators`)
        if (!res.ok) throw new Error('Failed to fetch')
        const data = await res.json()
        if (!active) return
        setValidators(Array.isArray(data) ? data : data.validators || [])
        setError('')
      } catch { if (active) setError('Unable to fetch validator data.') }
      finally { if (active) setLoading(false) }
    }
    fetch()
    const interval = setInterval(fetch, 5000)
    return () => { active = false; clearInterval(interval) }
  }, [])

  if (loading) return <div className="text-sm text-slate-400 dark:text-slate-500">Loading validators...</div>
  if (error) return <div className="text-sm text-red-400">{error}</div>

  if (selected) {
    const v = selected
    return (
      <div className="p-5 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40">
        <button onClick={() => setSelected(null)} className="text-xs text-blue-400 hover:text-blue-300 mb-4">&larr; Back</button>
        <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-4">{shorten(v.address || v.public_key, 8, 8)}</h3>
        <div className="space-y-2">
          {[
            ['Address', v.address || '--'],
            ['Stake', fmtStake(v.stake)],
            ['Status', v.is_active !== false ? 'Active' : 'Inactive'],
            ['Commission', `${(v.commission_rate || 0.1) * 100}%`],
            ['Blocks Produced', fmt(v.blocks_produced)],
            ['Blocks Missed', fmt(v.blocks_missed)],
            ['Delegators', fmt(v.delegator_count)],
            ['Total Delegated', fmtStake(v.total_delegated)],
            ['Uptime', v.blocks_produced && v.blocks_missed
              ? ((v.blocks_produced / (v.blocks_produced + v.blocks_missed)) * 100).toFixed(2) + '%'
              : '--'],
          ].map(([label, value]) => (
            <div key={label} className="flex justify-between items-center p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
              <span className="text-sm text-slate-500 dark:text-slate-400">{label}</span>
              <strong className="text-sm text-slate-900 dark:text-white">{value}</strong>
            </div>
          ))}
        </div>
      </div>
    )
  }

  return (
    <div className="p-5 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40">
      <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Active Validators ({validators.length})</h3>
      {validators.length === 0 ? (
        <div className="text-sm text-slate-400 dark:text-slate-500">No validators found.</div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-xs uppercase text-slate-400 dark:text-slate-500 border-b border-slate-200 dark:border-slate-700/50">
                {['Rank', 'Validator', 'Stake', 'Commission', 'Status', 'Uptime', 'Delegators'].map(h => (
                  <th key={h} className="text-left p-3 font-medium">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {validators.map((v, i) => (
                <tr key={v.address || i} onClick={() => setSelected(v)} className="border-b border-slate-100 dark:border-slate-700/30 hover:bg-blue-50 dark:hover:bg-blue-500/5 cursor-pointer">
                  <td className="p-3 text-slate-400 dark:text-slate-500">{i + 1}</td>
                  <td className="p-3"><code className="text-xs">{shorten(v.address || v.public_key, 8, 8)}</code></td>
                  <td className="p-3 text-right">{fmtStake(v.stake)}</td>
                  <td className="p-3">{(v.commission_rate || 0.1) * 100}%</td>
                  <td className="p-3">
                    <span className={`px-2 py-0.5 rounded-full text-xs ${
                      v.is_active !== false ? 'bg-emerald-500/10 text-emerald-400' : 'bg-red-500/10 text-red-400'
                    }`}>{v.is_active !== false ? 'Active' : 'Inactive'}</span>
                  </td>
                  <td className="p-3 text-right">
                    {v.blocks_produced && v.blocks_missed
                      ? ((v.blocks_produced / (v.blocks_produced + v.blocks_missed)) * 100).toFixed(1) + '%'
                      : '--'}
                  </td>
                  <td className="p-3 text-right">{fmt(v.delegator_count)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
