import { useEffect, useState } from 'react'

const API_URL = 'http://localhost:8545'
const fmt = (v) => v == null || isNaN(Number(v)) ? '--' : Number(v).toLocaleString()
const fmtStake = (s) => !s ? '--' : (Number(s) / 1e18).toFixed(2) + ' NBL'

export default function StakingTab() {
  const [stats, setStats] = useState(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [address, setAddress] = useState('')
  const [delegations, setDelegations] = useState([])

  useEffect(() => {
    let active = true
    const fetch = async () => {
      try {
        const [sr, vr] = await Promise.all([
          window.fetch(`${API_URL}/status`),
          window.fetch(`${API_URL}/validators`),
        ])
        if (!sr.ok || !vr.ok) throw new Error('Failed to fetch')
        const sd = await sr.json()
        const vd = await vr.json()
        const vals = Array.isArray(vd) ? vd : vd.validators || []
        if (!active) return
        const totalStake = vals.reduce((s, v) => s + Number(v.stake || 0), 0)
        const activeCount = vals.filter(v => v.is_active !== false).length
        setStats({
          totalValidators: vals.length, activeValidators: activeCount, totalStake,
          avgCommission: vals.length > 0 ? vals.reduce((s, v) => s + (v.commission_rate || 0.1), 0) / vals.length : 0,
        })
        setError('')
      } catch { if (active) setError('Unable to fetch staking data.') }
      finally { if (active) setLoading(false) }
    }
    fetch()
    const interval = setInterval(fetch, 15000)
    return () => { active = false; clearInterval(interval) }
  }, [])

  const fetchDelegations = async () => {
    if (!address) return
    try {
      const res = await window.fetch(`${API_URL}/delegations/${address}`)
      if (!res.ok) throw new Error()
      setDelegations(await res.json())
    } catch { setDelegations([]) }
  }

  if (loading) return <div className="text-sm text-slate-400 dark:text-slate-500">Loading staking data...</div>

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        {[
          { label: 'Active Validators', value: fmt(stats?.activeValidators), sub: `Of ${fmt(stats?.totalValidators)} total` },
          { label: 'Total Staked', value: fmtStake(stats?.totalStake), sub: 'Network bonded stake' },
          { label: 'Avg Commission', value: stats ? (stats.avgCommission * 100).toFixed(1) + '%' : '--', sub: 'Across all validators' },
          { label: 'Inflation Rate', value: '~5.2%', sub: 'Current annual rate' },
        ].map(({ label, value, sub }) => (
          <div key={label} className="p-4 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40">
            <span className="text-xs uppercase tracking-wider text-slate-500 dark:text-slate-400">{label}</span>
            <div className="text-2xl font-bold text-slate-900 dark:text-white mt-2">{value}</div>
            <p className="text-xs text-slate-400 dark:text-slate-500 mt-1">{sub}</p>
          </div>
        ))}
      </div>

      <div className="grid md:grid-cols-2 gap-4">
        <div className="p-5 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40">
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Check Delegations</h3>
          <div className="flex gap-2 mb-4">
            <input value={address} onChange={e => setAddress(e.target.value)} placeholder="Enter address (0x...)"
              className="flex-1 px-3 py-2 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-700 dark:text-slate-200 placeholder-slate-400 dark:placeholder-slate-500 font-mono" />
            <button onClick={fetchDelegations} className="px-4 py-2 rounded-xl bg-blue-600/80 hover:bg-blue-600 text-white text-sm">Check</button>
          </div>
          {delegations.length > 0 ? (
            <div className="space-y-2">
              {delegations.map((d, i) => (
                <div key={i} className="flex justify-between p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
                  <span className="text-sm text-slate-500 dark:text-slate-400">Validator: {d.validator_address || d.validator || '--'}</span>
                  <strong className="text-sm text-slate-900 dark:text-white">{fmtStake(d.amount)}</strong>
                </div>
              ))}
            </div>
          ) : address ? (
            <div className="text-sm text-slate-400 dark:text-slate-500">No delegations found for this address.</div>
          ) : (
            <div className="text-sm text-slate-400 dark:text-slate-500">Enter a wallet address to check its delegations.</div>
          )}
        </div>

        <div className="p-5 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40">
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Estimate Rewards</h3>
          <div className="text-sm text-slate-400 dark:text-slate-500">Rewards estimation coming soon.</div>
        </div>
      </div>
    </div>
  )
}
