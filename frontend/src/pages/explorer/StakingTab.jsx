import { useEffect, useState } from 'react'
import { Users, Coins, Percent, TrendingUp, Search, Inbox, Cpu } from 'lucide-react'

const API_URL = 'http://localhost:8545'
const fmt = (v) => v == null || isNaN(Number(v)) ? '--' : Number(v).toLocaleString()
const fmtStake = (s) => !s || isNaN(Number(s)) ? '--' : `${(Number(s) / 1e18).toLocaleString(undefined, { maximumFractionDigits: 2 })} NBL`
const shorten = (v, s = 12, e = 10) => !v ? '--' : v.length <= s + e + 3 ? v : `${v.slice(0, s)}...${v.slice(-e)}`

export default function StakingTab() {
  const [stats, setStats] = useState(null)
  const [validators, setValidators] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [address, setAddress] = useState('')
  const [delegations, setDelegations] = useState(null)
  const [checked, setChecked] = useState(false)
  const [stakeAmount, setStakeAmount] = useState('1000')
  const [selectedValidatorIdx, setSelectedValidatorIdx] = useState(0)

  const INFLATION_APR = 0.052

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
        const avgCommission = vals.length > 0
          ? vals.reduce((s, v) => s + Number(v.commission_rate || 0), 0) / vals.length
          : 0
        setValidators(vals)
        setStats({
          totalValidators: vals.length, activeValidators: activeCount, totalStake, avgCommission,
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
    setChecked(false)
    try {
      const res = await window.fetch(`${API_URL}/delegations/${address}`)
      if (!res.ok) throw new Error()
      const data = await res.json()
      setDelegations(Array.isArray(data) ? data : data.delegations || [])
    } catch { setDelegations([]) }
    finally { setChecked(true) }
  }

  if (loading) {
    return (
      <div className="space-y-3">
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
          {[0, 1, 2, 3].map((i) => <div key={i} className="h-24 rounded-2xl glass-strong animate-pulse" />)}
        </div>
        <div className="h-40 rounded-2xl glass-strong animate-pulse" />
      </div>
    )
  }
  if (error) {
    return (
      <div className="p-8 rounded-2xl glass-strong text-center">
        <p className="text-sm text-red-400">{error}</p>
      </div>
    )
  }

  const selectedValidator = validators[selectedValidatorIdx] || validators[0]
  const stake = parseFloat(stakeAmount) || 0
  const commissionPct = Number(selectedValidator?.commission_rate || stats?.avgCommission || 10)
  const commission = commissionPct / 100
  const grossYear = stake * INFLATION_APR
  const netYear = grossYear * (1 - commission)
  const netMonth = netYear / 12
  const netDay = netYear / 365

  const cards = [
    { icon: Users, label: 'Active Validators', value: fmt(stats?.activeValidators), sub: `Of ${fmt(stats?.totalValidators)} total`, chip: 'from-blue-500 to-cyan-600' },
    { icon: Coins, label: 'Total Staked', value: fmtStake(stats?.totalStake), sub: 'Network bonded stake', chip: 'from-emerald-500 to-teal-600' },
    { icon: Percent, label: 'Avg Commission', value: stats?.totalValidators ? `${stats.avgCommission.toFixed(1)}%` : '--', sub: 'Across all validators', chip: 'from-amber-500 to-orange-600' },
    { icon: TrendingUp, label: 'Inflation Rate', value: '~5.2%', sub: 'Current annual rate', chip: 'from-violet-500 to-purple-600' },
  ]

  return (
    <div className="space-y-4">
      {/* Stat cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
        {cards.map(({ icon: Icon, label, value, sub, chip }) => (
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

      <div className="grid lg:grid-cols-2 gap-4">
        {/* Delegation lookup */}
        <div className="p-5 rounded-2xl glass-strong">
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Check Delegations</h3>
          <div className="flex gap-2 mb-4">
            <input value={address} onChange={e => setAddress(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && fetchDelegations()}
              placeholder="Enter address (0x...)"
              className="flex-1 px-3 py-2 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-700 dark:text-slate-200 placeholder-slate-400 dark:placeholder-slate-500 font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/40" />
            <button onClick={fetchDelegations}
              className="inline-flex items-center gap-1.5 px-4 py-2 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 hover:opacity-90 text-white text-sm font-medium transition-all">
              <Search className="w-3.5 h-3.5" /> Check
            </button>
          </div>

          {checked && delegations != null && delegations.length > 0 ? (
            <div className="space-y-2">
              {delegations.map((d, i) => (
                <div key={i} className="flex justify-between items-center p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
                  <div className="min-w-0">
                    <span className="text-xs text-slate-400 dark:text-slate-500">Validator</span>
                    <p className="text-sm font-mono text-slate-900 dark:text-white truncate">{shorten(d.validator_address || d.validator || '--')}</p>
                  </div>
                  <strong className="text-sm text-slate-900 dark:text-white tabular-nums ml-2">{fmtStake(d.amount)}</strong>
                </div>
              ))}
            </div>
          ) : checked && address ? (
            <div className="flex flex-col items-center justify-center py-6 text-center">
              <Inbox className="w-7 h-7 text-slate-300 dark:text-slate-600 mb-2" />
              <p className="text-sm text-slate-400 dark:text-slate-500">No delegations found for this address.</p>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-6 text-center">
              <Inbox className="w-7 h-7 text-slate-300 dark:text-slate-600 mb-2" />
              <p className="text-sm text-slate-400 dark:text-slate-500">Enter a wallet address to check its delegations.</p>
            </div>
          )}
        </div>

        {/* Rewards estimator */}
        <div className="p-5 rounded-2xl glass-strong flex flex-col">
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Estimate Rewards</h3>
          <div className="space-y-3">
            <div>
              <label className="text-xs text-slate-500 dark:text-slate-400">Stake amount (NBL)</label>
              <input
                value={stakeAmount}
                onChange={(e) => setStakeAmount(e.target.value)}
                type="number"
                min="0"
                className="mt-1 w-full px-3 py-2 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-700 dark:text-slate-200 focus:outline-none focus:ring-2 focus:ring-violet-500/40"
              />
            </div>
            {validators.length > 0 && (
              <div>
                <label className="text-xs text-slate-500 dark:text-slate-400">Validator</label>
                <select
                  value={selectedValidatorIdx}
                  onChange={(e) => setSelectedValidatorIdx(Number(e.target.value))}
                  className="mt-1 w-full px-3 py-2 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-700 dark:text-slate-200 focus:outline-none focus:ring-2 focus:ring-violet-500/40"
                >
                  {validators.map((v, i) => (
                    <option key={v.address || i} value={i}>
                      {shorten(v.address || v.public_key, 8, 6)} — {v.commission_rate}% fee
                    </option>
                  ))}
                </select>
              </div>
            )}
            <div className="grid grid-cols-3 gap-2 pt-2">
              <div className="p-3 rounded-xl bg-violet-500/10 text-center">
                <p className="text-[10px] uppercase text-slate-500">Daily</p>
                <p className="text-sm font-semibold text-slate-900 dark:text-white tabular-nums">{netDay.toFixed(4)}</p>
              </div>
              <div className="p-3 rounded-xl bg-violet-500/10 text-center">
                <p className="text-[10px] uppercase text-slate-500">Monthly</p>
                <p className="text-sm font-semibold text-slate-900 dark:text-white tabular-nums">{netMonth.toFixed(2)}</p>
              </div>
              <div className="p-3 rounded-xl bg-violet-500/10 text-center">
                <p className="text-[10px] uppercase text-slate-500">Yearly</p>
                <p className="text-sm font-semibold text-slate-900 dark:text-white tabular-nums">{netYear.toFixed(2)}</p>
              </div>
            </div>
            <p className="text-[11px] text-slate-400 dark:text-slate-500">
              Based on {(INFLATION_APR * 100).toFixed(1)}% inflation APR minus {commissionPct.toFixed(1)}% validator commission.
            </p>
          </div>
        </div>
      </div>

      {/* Validator stake table */}
      {validators.length > 0 && (
        <div className="p-5 rounded-2xl glass-strong">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <Cpu className="w-4 h-4 text-blue-500 dark:text-cyan-400" />
              <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300">Validator Stake</h3>
            </div>
            <span className="text-xs text-slate-400 dark:text-slate-500">Genesis bond</span>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-xs uppercase text-slate-400 dark:text-slate-500 border-b border-slate-200 dark:border-slate-700/50">
                  {['#', 'Validator', 'Stake', 'Share', 'Commission', 'Status'].map(h => (
                    <th key={h} className="text-left p-3 font-medium">{h}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {validators.map((v, i) => {
                  const share = stats?.totalStake ? (Number(v.stake || 0) / stats.totalStake) * 100 : 0
                  return (
                    <tr key={v.address || i} className="border-b border-slate-100 dark:border-slate-700/30 hover:bg-slate-50 dark:hover:bg-slate-700/20">
                      <td className="p-3 text-slate-400 dark:text-slate-500">{i + 1}</td>
                      <td className="p-3"><code className="text-xs text-slate-800 dark:text-slate-200">{shorten(v.address || v.public_key)}</code></td>
                      <td className="p-3 font-semibold text-slate-900 dark:text-white tabular-nums">{fmtStake(v.stake)}</td>
                      <td className="p-3">
                        <div className="flex items-center gap-2">
                          <div className="w-20 h-1.5 rounded-full bg-slate-200 dark:bg-slate-700/50 overflow-hidden">
                            <div className="h-full rounded-full bg-gradient-to-r from-blue-500 to-cyan-500" style={{ width: `${share}%` }} />
                          </div>
                          <span className="text-xs text-slate-400 dark:text-slate-500 tabular-nums">{share.toFixed(1)}%</span>
                        </div>
                      </td>
                      <td className="p-3 text-slate-600 dark:text-slate-300 tabular-nums">{v.commission_rate}%</td>
                      <td className="p-3">
                        <span className={`px-2 py-0.5 rounded-full text-xs ${v.is_active !== false ? 'bg-emerald-500/10 text-emerald-400' : 'bg-red-500/10 text-red-400'}`}>
                          {v.is_active !== false ? 'Active' : 'Inactive'}
                        </span>
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  )
}
