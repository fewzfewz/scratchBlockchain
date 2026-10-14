import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { Coins, Search, Inbox, Calculator, ArrowRight } from 'lucide-react'
import { getApiUrl } from '../../lib/chain.js'
import { Card, Stat, Field, Button, EmptyState, inputClass } from '../../components/ui.jsx'

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

  // The node's reward rule (node/src/rewards.rs): each block pays its
  // proposer 10 NBL and splits 1 NBL among the validators that signed the
  // previous block. Proposers are chosen in proportion to stake.
  const PROPOSER_REWARD = 10
  const VOTE_REWARD = 1

  useEffect(() => {
    let active = true
    const fetch = async () => {
      try {
        const [sr, vr] = await Promise.all([
          window.fetch(`${getApiUrl()}/status`),
          window.fetch(`${getApiUrl()}/validators`),
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
      const res = await window.fetch(`${getApiUrl()}/delegations/${address}`)
      if (!res.ok) throw new Error()
      const data = await res.json()
      setDelegations(Array.isArray(data) ? data : data.delegations || [])
    } catch { setDelegations([]) }
    finally { setChecked(true) }
  }

  if (loading) {
    return <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">{[0, 1, 2, 3].map((i) => <div key={i} className="h-24 rounded-2xl glass-strong animate-pulse" />)}</div>
  }
  if (error) return <Card><EmptyState title="Unavailable">{error}</EmptyState></Card>

  const totalNbl = (stats?.totalStake || 0) / 1e18
  const stake = parseFloat(stakeAmount) || 0
  const share = stake > 0 ? stake / (totalNbl + stake) : 0
  const n = (stats?.activeValidators || 0) + 1
  const per1000 = 1000 * (share * PROPOSER_REWARD + VOTE_REWARD / n)

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
        <Stat icon={Coins} label="Total staked" value={`${totalNbl.toLocaleString(undefined, { maximumFractionDigits: 0 })} NBL`} />
        <Stat label="Validators" value={fmt(stats?.activeValidators)} sub={`${fmt(stats?.totalValidators)} registered`} />
        <Stat label="Avg commission" value={`${(stats?.avgCommission || 0).toFixed(1)}%`} />
        <Stat label="Issued per block" value={`${PROPOSER_REWARD + VOTE_REWARD} NBL`} sub="10 proposer + 1 to signers" />
      </div>

      <div className="grid lg:grid-cols-2 gap-6">
        <Card title="Reward estimate" icon={Calculator} description="For a validator running a node and signing every block.">
          <Field label="Your stake (NBL)">
            <input value={stakeAmount} onChange={(e) => setStakeAmount(e.target.value)} inputMode="decimal" className={inputClass} />
          </Field>
          <div className="mt-5 grid grid-cols-2 gap-3">
            <Stat label="Share of stake" value={`${(share * 100).toFixed(2)}%`} sub="≈ share of blocks you propose" />
            <Stat label="Per 1,000 blocks" value={`${per1000.toLocaleString(undefined, { maximumFractionDigits: 2 })} NBL`} sub="before commission" />
          </div>
          <p className="mt-4 text-xs text-slate-500">
            Rewards follow the node's rule: {PROPOSER_REWARD} NBL to each block's proposer, {VOTE_REWARD} NBL split among the previous block's signers.
          </p>
          <Link to="/validators/onboard" className="mt-4 inline-flex items-center gap-1.5 text-sm font-medium text-blue-600 dark:text-blue-300 hover:underline">
            Register as a validator <ArrowRight className="w-3.5 h-3.5" />
          </Link>
        </Card>

        <Card title="Delegations" icon={Search} description="Look up an address's delegations.">
          <form onSubmit={(e) => { e.preventDefault(); fetchDelegations() }} className="flex gap-2">
            <input value={address} onChange={(e) => setAddress(e.target.value.trim())} placeholder="0x…" aria-label="Delegator address" className={`${inputClass} font-mono`} />
            <Button type="submit" icon={Search}>Look up</Button>
          </form>
          {checked && (delegations?.length ? (
            <ul className="mt-4 divide-y divide-slate-100 dark:divide-slate-800">
              {delegations.map((d, i) => (
                <li key={i} className="flex items-center justify-between py-2.5 text-sm">
                  <span className="font-mono text-xs text-slate-500">{shorten(d.validator_address || d.validator)}</span>
                  <span className="tabular-nums text-slate-900 dark:text-white">{fmtStake(d.amount)}</span>
                </li>
              ))}
            </ul>
          ) : (
            <EmptyState icon={Inbox} title="No delegations">This address hasn't delegated to any validator.</EmptyState>
          ))}
        </Card>
      </div>
    </div>
  )
}
