import { useEffect, useState } from 'react'
import { ArrowLeft, ShieldCheck } from 'lucide-react'
import { getApiUrl } from '../../lib/chain.js'
import { Card, KeyValue, Badge, EmptyState, Stat } from '../../components/ui.jsx'

const shorten = (v, s = 10, e = 8) => (!v ? '—' : v.length <= s + e + 3 ? v : `${v.slice(0, s)}…${v.slice(-e)}`)
const fmt = (v) => (v == null || isNaN(Number(v)) ? '—' : Number(v).toLocaleString())
const nbl = (s) => (!s || isNaN(Number(s)) ? 0 : Number(s) / 1e18)
const fmtNbl = (n) => `${n.toLocaleString(undefined, { maximumFractionDigits: 2 })} NBL`
const totalStake = (v) => nbl(v.stake) + nbl(v.total_delegated)

/** [label, tone] for a validator's state. */
function status(v) {
  if (v.tombstoned) return ['Slashed', 'red']
  if (v.jailed) return ['Jailed', 'amber']
  if (v.is_active === false) return ['Inactive', 'neutral']
  return ['Active', 'green']
}

function uptime(v) {
  const scheduled = (v.blocks_produced || 0) + (v.blocks_missed || 0)
  return scheduled > 0 ? ((v.blocks_produced || 0) / scheduled) * 100 : null
}

export default function ValidatorsTab() {
  const [validators, setValidators] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [selected, setSelected] = useState(null)

  useEffect(() => {
    let active = true
    const load = async () => {
      try {
        const res = await window.fetch(`${getApiUrl()}/validators`)
        if (!res.ok) throw new Error('Failed to fetch')
        const data = await res.json()
        if (!active) return
        setValidators(Array.isArray(data) ? data : data.validators || [])
        setError('')
      } catch { if (active) setError('Unable to fetch validator data.') }
      finally { if (active) setLoading(false) }
    }
    load()
    const interval = setInterval(load, 5000)
    return () => { active = false; clearInterval(interval) }
  }, [])

  const sorted = [...validators].sort((a, b) => totalStake(b) - totalStake(a))
  const networkStake = sorted.reduce((sum, v) => sum + totalStake(v), 0)

  if (selected) {
    const v = selected
    const up = uptime(v)
    return (
      <Card>
        <button type="button" onClick={() => setSelected(null)}
          className="inline-flex items-center gap-1.5 text-xs font-medium text-blue-600 dark:text-blue-300 hover:underline mb-4">
          <ArrowLeft className="w-3.5 h-3.5" /> All validators
        </button>
        <div className="flex flex-wrap items-center gap-3 mb-6">
          <h2 className="font-mono text-lg font-semibold text-slate-900 dark:text-white">{shorten(v.address, 12, 8)}</h2>
          <Badge tone={status(v)[1]}>{status(v)[0]}</Badge>
        </div>
        <div className="grid sm:grid-cols-3 gap-3 mb-6">
          <Stat label="Voting power" value={networkStake ? `${((totalStake(v) / networkStake) * 100).toFixed(1)}%` : '—'} sub={fmtNbl(totalStake(v))} />
          <Stat label="Uptime" value={up != null ? `${up.toFixed(1)}%` : '—'} sub={`${fmt(v.blocks_missed)} missed`} />
          <Stat label="Blocks produced" value={fmt(v.blocks_produced)} />
        </div>
        <KeyValue rows={[
          ['Address', v.address || '—', true],
          ['Public key', shorten(v.public_key, 14, 10), true],
          ['Own stake', fmtNbl(nbl(v.stake))],
          ['Delegated', fmtNbl(nbl(v.total_delegated))],
          ['Delegators', fmt(v.delegator_count)],
          ['Commission', `${v.commission_rate}%`],
          ['Missed turns in a row', fmt(v.missed_in_row || 0)],
          ...(v.jailed && !v.tombstoned ? [['Jailed until block', fmt(v.jailed_until)]] : []),
          ...(v.slashed && v.slashed !== '0' ? [['Slashed', fmtNbl(nbl(v.slashed))]] : []),
        ]} />
      </Card>
    )
  }

  return (
    <Card
      title="Validators"
      icon={ShieldCheck}
      description={`${sorted.length} registered · a block needs signatures from more than 2/3 of the stake · proposers are picked in proportion to stake · set changes apply every epoch`}
      padded={false}
    >
      {loading ? (
        <div className="p-5 space-y-2">{[0, 1, 2, 3].map((i) => <div key={i} className="h-12 rounded-xl bg-slate-100 dark:bg-slate-800/60 animate-pulse" />)}</div>
      ) : error ? (
        <EmptyState title="Unavailable">{error}</EmptyState>
      ) : sorted.length === 0 ? (
        <EmptyState title="No validators" />
      ) : (
        <div>
          <div className="hidden md:grid grid-cols-[40px_1.3fr_1.4fr_90px_90px] gap-4 px-6 py-2.5 text-[11px] uppercase tracking-wider text-slate-500 border-y border-slate-100 dark:border-slate-800">
            <span>#</span><span>Validator</span><span>Stake</span><span className="text-right">Uptime</span><span className="text-right">Produced</span>
          </div>
          {sorted.map((v, i) => {
            const share = networkStake ? (totalStake(v) / networkStake) * 100 : 0
            const up = uptime(v)
            return (
              <button key={v.address || i} type="button" onClick={() => setSelected(v)}
                className="w-full grid grid-cols-[40px_1fr_auto] md:grid-cols-[40px_1.3fr_1.4fr_90px_90px] gap-4 items-center px-6 py-3 text-left text-sm border-b last:border-0 border-slate-100 dark:border-slate-800 hover:bg-slate-50 dark:hover:bg-slate-800/40 transition-colors">
                <span className="text-slate-400 tabular-nums">{i + 1}</span>
                <span className="min-w-0">
                  <span className="block font-mono text-xs text-slate-900 dark:text-white truncate">{shorten(v.address, 10, 6)}</span>
                  <span className="block text-xs text-slate-500">
                    {v.commission_rate}% commission
                    {status(v)[0] !== 'Active' && <Badge tone={status(v)[1]} className="ml-2 !h-5">{status(v)[0]}</Badge>}
                  </span>
                </span>
                <span className="min-w-0">
                  <span className="flex items-center justify-between gap-2 text-xs">
                    <span className="tabular-nums text-slate-900 dark:text-white">{fmtNbl(totalStake(v))}</span>
                    <span className="tabular-nums text-slate-500 hidden md:inline">{share.toFixed(1)}%</span>
                  </span>
                  <span className="hidden md:block mt-1.5 h-1.5 rounded-full bg-slate-100 dark:bg-slate-800 overflow-hidden">
                    <span className="block h-full rounded-full bg-gradient-to-r from-blue-600 to-cyan-500" style={{ width: `${share}%` }} />
                  </span>
                </span>
                <span className="hidden md:block text-right tabular-nums text-slate-600 dark:text-slate-300">{up != null ? `${up.toFixed(1)}%` : '—'}</span>
                <span className="hidden md:block text-right tabular-nums text-slate-500">{fmt(v.blocks_produced)}</span>
              </button>
            )
          })}
        </div>
      )}
    </Card>
  )
}
