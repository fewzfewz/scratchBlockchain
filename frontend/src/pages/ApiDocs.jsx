import { useState, useEffect } from 'react'
import SwaggerUI from 'swagger-ui-react'
import 'swagger-ui-react/swagger-ui.css'
import {
  FileJson, Activity, Gauge, ShieldCheck, WifiOff, BookOpen,
  Server, Terminal, Layers, Zap, ArrowLeft,
} from 'lucide-react'
import { Link } from 'react-router-dom'
import { getApiUrl } from '../lib/chain.js'
import { PageHeader, LiveBadge } from '../components/ui.jsx'

const ENDPOINT_COUNT = 17

const fmt = (v) => v == null || isNaN(Number(v)) ? '--' : Number(v).toLocaleString()

const STATS = [
  { icon: Server, label: 'Endpoints', value: String(ENDPOINT_COUNT), sub: 'REST + JSON-RPC', chip: 'from-blue-600 to-cyan-600' },
  { icon: Layers, label: 'Spec', value: 'OpenAPI 3.0', sub: 'docs/openapi.yaml', chip: 'from-indigo-600 to-violet-600' },
  { icon: Terminal, label: 'Base URL', value: 'localhost:8545', sub: 'HTTP JSON-RPC', chip: 'from-emerald-600 to-teal-600' },
  { icon: Zap, label: 'Auth', value: 'None', sub: 'Public testnet', chip: 'from-amber-500 to-orange-600' },
]

export default function ApiDocs() {
  const [backendOnline, setBackendOnline] = useState(true)
  const [network, setNetwork] = useState(null)

  // Node health + network info polling (matches Explorer/Faucet/Docs/Governance)
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

  return (
    <div className="relative min-h-screen overflow-hidden">

      <div className="relative z-10 py-12 max-w-7xl mx-auto px-4 animate-fade-in">
        {/* ── Header ─────────────────────────────────────────────── */}
        <PageHeader icon={FileJson} eyebrow="API reference" title="Node RPC API"
          description="Every HTTP route the node serves, with request and response shapes and a live playground."
          actions={<LiveBadge online={backendOnline} label={network ? `#${Number(network.height).toLocaleString()}` : 'Online'} />} />

        {!backendOnline && (
          <div className="mb-8 px-4 py-3 rounded-xl bg-amber-500/15 border border-amber-500/30 text-sm text-amber-400 flex items-center gap-2">
            <WifiOff className="w-4 h-4 shrink-0" />
            <span>Node not reachable at {getApiUrl()}. The interactive calls below need a running testnet — start it to try them.</span>
          </div>
        )}

        {/* ── Quick stats ───────────────────────────────────────── */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-8">
          {STATS.map(({ icon: Icon, label, value, sub, chip }) => (
            <div key={label} className="p-4 rounded-2xl glass-strong hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all">
              <div className="flex items-center gap-2 mb-3">
                <div className={`w-8 h-8 rounded-lg bg-gradient-to-br ${chip} flex items-center justify-center text-white shadow-md`}>
                  <Icon className="w-4 h-4" />
                </div>
                <span className="text-[11px] uppercase tracking-wider text-slate-500 dark:text-slate-400">{label}</span>
              </div>
              <div className="text-xl font-bold text-slate-900 dark:text-white tabular-nums truncate">{value}</div>
              <p className="text-xs text-slate-400 dark:text-slate-500 mt-1">{sub}</p>
            </div>
          ))}
        </div>

        {/* ── Toolbar ───────────────────────────────────────────── */}
        <div className="flex items-center justify-between gap-3 mb-4">
          <Link
            to="/docs"
            className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl glass text-xs font-medium text-slate-600 dark:text-slate-300 hover:-translate-y-0.5 transition-all"
          >
            <ArrowLeft className="w-3.5 h-3.5" />
            Back to docs
          </Link>
          <a
            href="/docs"
            className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl glass text-xs font-medium text-slate-600 dark:text-slate-300 hover:-translate-y-0.5 transition-all"
          >
            <BookOpen className="w-3.5 h-3.5" />
            Read the guide
          </a>
        </div>

        {/* ── Swagger UI ────────────────────────────────────────── */}
        <div className="swagger-ui-wrapper">
          <SwaggerUI url="/openapi.yaml" />
        </div>
      </div>
    </div>
  )
}
