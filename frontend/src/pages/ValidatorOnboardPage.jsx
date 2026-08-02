import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import {
  Shield, CheckCircle, AlertCircle, Server, Cpu, Wifi, ExternalLink,
  Copy, RefreshCw, BookOpen, Activity, Gauge,
} from 'lucide-react'

const API_URL = 'http://localhost:8545'
const GRAFANA_URL = 'http://localhost:3000/d/validator-onboarding/validator-onboarding'
const ONBOARDING_DOC = 'https://github.com/fewzfewz/scratchBlockchain/blob/main/docs/validator-onboarding.md'

const CHECKS = [
  { id: 'health', label: 'Node health', path: '/health' },
  { id: 'status', label: 'Chain status', path: '/status' },
  { id: 'validators', label: 'Validator set', path: '/validators' },
  { id: 'peers', label: 'Peer connectivity', path: '/peers' },
]

export default function ValidatorOnboardPage() {
  const [results, setResults] = useState({})
  const [loading, setLoading] = useState(false)
  const [registerAddr, setRegisterAddr] = useState('')
  const [registerMsg, setRegisterMsg] = useState('')

  const runChecks = async () => {
    setLoading(true)
    const next = {}
    for (const check of CHECKS) {
      try {
        const r = await window.fetch(`${API_URL}${check.path}`)
        next[check.id] = { ok: r.ok, detail: r.ok ? 'OK' : `HTTP ${r.status}` }
      } catch (e) {
        next[check.id] = { ok: false, detail: e.message }
      }
    }
    setResults(next)
    setLoading(false)
  }

  useEffect(() => {
    runChecks()
  }, [])

  const registerValidator = async () => {
    if (!registerAddr) {
      setRegisterMsg('Enter a validator address (0x...)')
      return
    }
    try {
      const r = await window.fetch(`${API_URL}/validators/register`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          address: registerAddr,
          public_key: registerAddr,
          stake: '1000000000000000000',
          commission_rate: 10,
        }),
      })
      const text = await r.text()
      setRegisterMsg(r.ok ? `Registered: ${text}` : `Failed: ${text}`)
      if (r.ok) runChecks()
    } catch (e) {
      setRegisterMsg('Error: ' + e.message)
    }
  }

  const steps = [
    { n: 1, title: 'Provision hardware', body: '4+ CPU cores, 16 GB RAM, 500 GB SSD. Open P2P 26656 and metrics 26657.' },
    { n: 2, title: 'Build the node', body: 'cargo build -p node --release' },
    { n: 3, title: 'Generate keys', body: 'Use scripts/setup-validator.sh or import an Ed25519 keypair.' },
    { n: 4, title: 'Configure genesis', body: 'Point --genesis at genesis.json and set data_dir in node config.' },
    { n: 5, title: 'Join the network', body: 'POST /connect_peer with bootstrap multiaddrs from deployment/local.' },
    { n: 6, title: 'Register stake', body: 'POST /validators/register (min 1 NBL) or delegate via POST /delegate.' },
    { n: 7, title: 'Monitor & alert', body: 'Watch Grafana Validator Onboarding dashboard; Alertmanager fires on validator down / stalled consensus.' },
  ]

  return (
    <div className="relative min-h-[70vh] overflow-hidden animate-fade-in">
      <div className="absolute inset-0 bg-grid pointer-events-none" />
      <div className="relative z-10 max-w-5xl mx-auto px-4 py-10">
        <div className="text-center mb-8">
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full glass text-xs text-emerald-600 dark:text-emerald-400 mb-4">
            <Shield className="w-3.5 h-3.5" />
            Validator operator guide
          </div>
          <h1 className="text-3xl font-bold text-slate-900 dark:text-white mb-2">Run a Validator</h1>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Onboarding checklist, health probes, and monitoring links
          </p>
        </div>

        <div className="grid lg:grid-cols-2 gap-4 mb-6">
          <div className="p-5 rounded-2xl glass-strong">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 flex items-center gap-2">
                <Activity className="w-4 h-4 text-blue-500" />
                Pre-flight checks
              </h2>
              <button onClick={runChecks} disabled={loading}
                className="p-2 rounded-lg hover:bg-slate-200/50 dark:hover:bg-slate-700/50">
                <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
              </button>
            </div>
            <div className="space-y-2">
              {CHECKS.map(({ id, label }) => {
                const r = results[id]
                return (
                  <div key={id} className="flex items-center justify-between p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
                    <span className="text-sm text-slate-700 dark:text-slate-200">{label}</span>
                    {r ? (
                      r.ok ? (
                        <span className="flex items-center gap-1 text-xs text-emerald-500"><CheckCircle className="w-3.5 h-3.5" /> {r.detail}</span>
                      ) : (
                        <span className="flex items-center gap-1 text-xs text-red-400"><AlertCircle className="w-3.5 h-3.5" /> {r.detail}</span>
                      )
                    ) : (
                      <span className="text-xs text-slate-400">…</span>
                    )}
                  </div>
                )
              })}
            </div>
          </div>

          <div className="p-5 rounded-2xl glass-strong">
            <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4 flex items-center gap-2">
              <Gauge className="w-4 h-4 text-violet-500" />
              Register validator (testnet)
            </h2>
            <input
              value={registerAddr}
              onChange={(e) => setRegisterAddr(e.target.value)}
              placeholder="Validator address 0x..."
              className="w-full px-3 py-2 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm font-mono mb-3"
            />
            <button onClick={registerValidator}
              className="w-full py-2.5 rounded-xl bg-gradient-to-r from-emerald-600 to-teal-600 text-white text-sm font-medium">
              POST /validators/register
            </button>
            {registerMsg && <p className="text-xs mt-2 text-slate-500 dark:text-slate-400">{registerMsg}</p>}
            <div className="mt-4 flex flex-wrap gap-2">
              <a href={GRAFANA_URL} target="_blank" rel="noopener noreferrer"
                className="inline-flex items-center gap-1 text-xs px-3 py-1.5 rounded-lg bg-slate-200/60 dark:bg-slate-700/50">
                <ExternalLink className="w-3 h-3" /> Grafana dashboard
              </a>
              <a href={ONBOARDING_DOC} target="_blank" rel="noopener noreferrer"
                className="inline-flex items-center gap-1 text-xs px-3 py-1.5 rounded-lg bg-slate-200/60 dark:bg-slate-700/50">
                <BookOpen className="w-3 h-3" /> Full docs
              </a>
              <Link to="/explorer" className="inline-flex items-center gap-1 text-xs px-3 py-1.5 rounded-lg bg-slate-200/60 dark:bg-slate-700/50">
                <Cpu className="w-3 h-3" /> Explorer
              </Link>
            </div>
          </div>
        </div>

        <div className="p-5 rounded-2xl glass-strong">
          <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4 flex items-center gap-2">
            <Server className="w-4 h-4" />
            Onboarding steps
          </h2>
          <div className="space-y-3">
            {steps.map(({ n, title, body }) => (
              <div key={n} className="flex gap-3 p-3 rounded-xl bg-slate-100/50 dark:bg-slate-700/25">
                <div className="w-7 h-7 rounded-lg bg-gradient-to-br from-blue-500 to-cyan-600 flex items-center justify-center text-white text-xs font-bold shrink-0">
                  {n}
                </div>
                <div>
                  <p className="text-sm font-medium text-slate-800 dark:text-slate-100">{title}</p>
                  <p className="text-xs text-slate-500 dark:text-slate-400 mt-0.5">{body}</p>
                </div>
              </div>
            ))}
          </div>
          <p className="text-[11px] text-slate-400 mt-4 flex items-center gap-1">
            <Wifi className="w-3 h-3" />
            Alerts: ValidatorDown, ConsensusStalled, LowPeerCount → Alertmanager (Slack/PagerDuty via env vars)
          </p>
        </div>
      </div>
    </div>
  )
}
