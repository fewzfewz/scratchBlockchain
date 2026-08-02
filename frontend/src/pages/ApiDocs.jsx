import { useState, useEffect } from 'react'
import SwaggerUI from 'swagger-ui-react'
import 'swagger-ui-react/swagger-ui.css'
import {
  FileJson, Activity, Gauge, ShieldCheck, WifiOff, BookOpen,
  Server, Terminal, Layers, Zap, ArrowLeft,
} from 'lucide-react'
import { Link } from 'react-router-dom'

const RPC_URL = 'http://localhost:8545'
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
          window.fetch(`${RPC_URL}/health`),
          window.fetch(`${RPC_URL}/status`),
          window.fetch(`${RPC_URL}/gas_price`),
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
      {/* Aurora blobs + grid backdrop */}
      <div className="absolute -top-40 -left-40 w-[38rem] h-[38rem] rounded-full opacity-25 animate-float pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(14,165,233,0.45), transparent 70%)' }} />
      <div className="absolute top-40 -right-40 w-[34rem] h-[34rem] rounded-full opacity-20 animate-float-alt pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(139,92,246,0.4), transparent 70%)' }} />
      <div className="absolute inset-0 bg-grid pointer-events-none" />

      <div className="relative z-10 max-w-7xl mx-auto px-4 py-8 md:py-10 animate-fade-in">
        {/* ── Header ─────────────────────────────────────────────── */}
        <header className="text-center mb-8">
          <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full glass text-xs font-medium text-slate-600 dark:text-slate-300 mb-5">
            <span className={`w-2 h-2 rounded-full ${backendOnline ? 'bg-emerald-400 animate-pulse-dot' : 'bg-red-400'}`} />
            {backendOnline ? 'Local testnet live' : 'Node offline'}
            <span className="text-slate-400 dark:text-slate-500">· {ENDPOINT_COUNT} endpoints · OpenAPI 3.0</span>
          </div>

          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-indigo-600 to-blue-600 flex items-center justify-center text-white shadow-lg shadow-indigo-600/25">
              <FileJson className="w-6 h-6" />
            </div>
            <h1 className="text-4xl md:text-5xl font-bold tracking-tight text-slate-900 dark:text-white">
              Nebula
              <span className="bg-gradient-to-r from-indigo-500 via-blue-500 to-cyan-500 bg-clip-text text-transparent"> API Reference</span>
            </h1>
          </div>
          <p className="text-slate-500 dark:text-slate-400 max-w-xl mx-auto">
            Interactive documentation for every RPC endpoint. Try calls directly from your browser against{' '}
            <code className="text-xs text-blue-600 dark:text-cyan-400 font-mono">{RPC_URL}</code>.
          </p>

          {/* Network strip */}
          {network && (
            <div className="flex items-center justify-center flex-wrap gap-2 mt-5 text-xs">
              <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full glass-strong font-mono text-slate-700 dark:text-slate-200">
                <Activity className="w-3.5 h-3.5 text-blue-500 dark:text-cyan-400" /> Tip #{fmt(network.height)}
              </span>
              <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full glass-strong font-mono text-slate-700 dark:text-slate-200">
                <Gauge className="w-3.5 h-3.5 text-emerald-500" /> Base fee {fmt(network.baseFee)}
              </span>
              <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full glass-strong font-mono text-slate-700 dark:text-slate-200">
                <ShieldCheck className="w-3.5 h-3.5 text-violet-500" /> Finalized #{fmt(network.finalized)}
              </span>
            </div>
          )}
        </header>

        {!backendOnline && (
          <div className="mb-8 px-4 py-3 rounded-xl bg-amber-500/15 border border-amber-500/30 text-sm text-amber-400 flex items-center gap-2">
            <WifiOff className="w-4 h-4 shrink-0" />
            <span>Node not reachable at {RPC_URL}. The interactive calls below need a running testnet — start it to try them.</span>
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
