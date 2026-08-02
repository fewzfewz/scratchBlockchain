import { useEffect, useState } from 'react'
import { Cpu, ArrowLeft, ShieldCheck, Clock, Coins, Users, Wallet, TrendingUp, Percent } from 'lucide-react'

const API_URL = 'http://localhost:8545'
const shorten = (v, s = 10, e = 8) => !v ? '--' : v.length <= s + e + 3 ? v : `${v.slice(0, s)}...${v.slice(-e)}`
const fmt = (v) => v == null || isNaN(Number(v)) ? '--' : Number(v).toLocaleString()
const fmtStake = (s) => !s ? '--' : `${Number(s).toLocaleString()} NBL`

const RANK_COLORS = ['from-amber-400 to-yellow-500', 'from-slate-300 to-slate-400', 'from-orange-400 to-red-400']

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

  if (loading) {
    return (
      <div className="space-y-3">
        {[0, 1, 2].map((i) => (
          <div key={i} className="h-28 rounded-2xl glass-strong animate-pulse" />
        ))}
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

  if (selected) {
    const v = selected
    const uptime = v.blocks_produced && v.blocks_missed
      ? ((v.blocks_produced / (v.blocks_produced + v.blocks_missed)) * 100).toFixed(2)
      : null
    const rows = [
      { icon: Wallet, label: 'Address', value: v.address || '--', mono: true },
      { icon: Coins, label: 'Stake', value: fmtStake(v.stake) },
      { icon: Percent, label: 'Commission', value: `${v.commission_rate}%` },
      { icon: Cpu, label: 'Blocks Produced', value: fmt(v.blocks_produced) },
      { icon: Clock, label: 'Blocks Missed', value: fmt(v.blocks_missed) },
      { icon: Users, label: 'Delegators', value: fmt(v.delegator_count) },
      { icon: TrendingUp, label: 'Total Delegated', value: fmtStake(v.total_delegated) },
    ]
    return (
      <div className="rounded-2xl glass-strong overflow-hidden">
        <div className="bg-gradient-to-r from-blue-600/10 via-cyan-600/10 to-violet-600/10 border-b border-slate-200/50 dark:border-slate-700/40 px-5 py-4">
          <button onClick={() => setSelected(null)}
            className="inline-flex items-center gap-1.5 text-xs font-medium text-blue-600 dark:text-cyan-400 hover:underline mb-3">
            <ArrowLeft className="w-3.5 h-3.5" /> All validators
          </button>
          <div className="flex items-center gap-3">
            <div className="w-11 h-11 rounded-xl bg-gradient-to-br from-blue-500 to-cyan-600 flex items-center justify-center text-white shadow-lg">
              <Cpu className="w-5 h-5" />
            </div>
            <div>
              <h3 className="text-lg font-bold text-slate-900 dark:text-white font-mono">{shorten(v.address || v.public_key, 12, 10)}</h3>
              <span className={`inline-flex items-center gap-1.5 text-xs mt-0.5 ${v.is_active !== false ? 'text-emerald-500' : 'text-red-400'}`}>
                <ShieldCheck className="w-3.5 h-3.5" />
                {v.is_active !== false ? 'Active validator' : 'Inactive'}
              </span>
            </div>
          </div>
        </div>
        <div className="p-5 grid sm:grid-cols-2 gap-2">
          {rows.map(({ icon: Icon, label, value, mono }) => (
            <div key={label} className="flex justify-between items-center gap-3 p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
              <span className="flex items-center gap-2 text-sm text-slate-500 dark:text-slate-400">
                <Icon className="w-3.5 h-3.5" /> {label}
              </span>
              <strong className={`text-sm text-slate-900 dark:text-white tabular-nums truncate ${mono ? 'font-mono' : ''}`}>{value}</strong>
            </div>
          ))}
          <div className="sm:col-span-2 flex justify-between items-center gap-3 p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
            <span className="flex items-center gap-2 text-sm text-slate-500 dark:text-slate-400">
              <TrendingUp className="w-3.5 h-3.5" /> Uptime
            </span>
            <strong className="text-sm text-slate-900 dark:text-white tabular-nums">
              {uptime ? `${uptime}%` : '--'}
            </strong>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between px-1">
        <div>
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300">Active Validators</h3>
          <p className="text-xs text-slate-400 dark:text-slate-500 mt-0.5">BFT set · {validators.length} validators · 2/3 quorum</p>
        </div>
        <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-emerald-500/10 text-emerald-500 text-xs">
          <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse-dot" />
          {validators.length} online
        </span>
      </div>

      {validators.length === 0 ? (
        <div className="p-8 rounded-2xl glass-strong text-center">
          <Cpu className="w-8 h-8 text-slate-300 dark:text-slate-600 mx-auto mb-2" />
          <p className="text-sm text-slate-400 dark:text-slate-500">No validators found.</p>
          <p className="text-xs text-slate-400 dark:text-slate-500 mt-1">The validator set is registered at genesis.</p>
        </div>
      ) : (
        validators.map((v, i) => {
          const uptime = v.blocks_produced && v.blocks_missed
            ? Math.round((v.blocks_produced / (v.blocks_produced + v.blocks_missed)) * 100)
            : null
          return (
            <button key={v.address || i} onClick={() => setSelected(v)}
              className="w-full text-left p-4 rounded-2xl glass hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all">
              <div className="flex items-center gap-3">
                <div className={`w-8 h-8 shrink-0 rounded-lg bg-gradient-to-br ${RANK_COLORS[i % RANK_COLORS.length]} flex items-center justify-center text-white text-xs font-bold shadow-md`}>
                  {i + 1}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-sm font-semibold text-slate-900 dark:text-white truncate">{shorten(v.address || v.public_key, 12, 10)}</span>
                    {v.is_active !== false && (
                      <span className="px-1.5 py-0.5 rounded-full bg-emerald-500/10 text-emerald-500 text-[10px] font-medium">ACTIVE</span>
                    )}
                  </div>
                  <div className="text-xs text-slate-400 dark:text-slate-500 mt-0.5">{fmtStake(v.stake)} · {v.commission_rate}% commission</div>
                </div>
                <div className="text-right shrink-0 hidden sm:block">
                  <div className="text-sm font-bold text-slate-900 dark:text-white tabular-nums">{uptime != null ? `${uptime}%` : '--'}</div>
                  <div className="text-[10px] uppercase tracking-wider text-slate-400 dark:text-slate-500">Uptime</div>
                </div>
                {uptime != null && (
                  <div className="w-20 hidden md:block">
                    <div className="h-1.5 rounded-full bg-slate-200 dark:bg-slate-700/50 overflow-hidden">
                      <div className="h-full rounded-full bg-gradient-to-r from-emerald-500 to-teal-500" style={{ width: `${uptime}%` }} />
                    </div>
                  </div>
                )}
              </div>
            </button>
          )
        })
      )}
    </div>
  )
}
