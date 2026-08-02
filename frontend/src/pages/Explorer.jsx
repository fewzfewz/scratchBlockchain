import { useEffect, useState } from 'react'
import { Compass, Activity, Shield } from 'lucide-react'
import DashboardTab from './explorer/DashboardTab.jsx'
import ValidatorsTab from './explorer/ValidatorsTab.jsx'
import StakingTab from './explorer/StakingTab.jsx'
import BlocksTab from './explorer/BlocksTab.jsx'

const API_URL = 'http://localhost:8545'

const TABS = [
  { id: 'dashboard', label: 'Dashboard' },
  { id: 'blocks', label: 'Blocks' },
  { id: 'validators', label: 'Validators' },
  { id: 'staking', label: 'Staking' },
]

const fmt = (v) => v == null ? '--' : Number(v).toLocaleString()

export default function Explorer() {
  const [activeTab, setActiveTab] = useState('dashboard')
  const [online, setOnline] = useState(false)
  const [tip, setTip] = useState(null)

  useEffect(() => {
    let active = true
    const poll = async () => {
      try {
        const res = await window.fetch(`${API_URL}/status`)
        if (!res.ok) throw new Error('offline')
        const d = await res.json()
        if (!active) return
        setTip(d)
        setOnline(true)
      } catch { if (active) setOnline(false) }
    }
    poll()
    const t = setInterval(poll, 3000)
    return () => { active = false; clearInterval(t) }
  }, [])

  return (
    <div className="relative min-h-screen overflow-hidden">
      {/* Aurora blobs */}
      <div className="absolute -top-40 -right-40 w-[38rem] h-[38rem] rounded-full opacity-25 animate-float pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(14,165,233,0.45), transparent 70%)' }} />
      <div className="absolute top-40 -left-40 w-[34rem] h-[34rem] rounded-full opacity-20 animate-float-slow pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(139,92,246,0.4), transparent 70%)' }} />
      <div className="absolute inset-0 bg-grid pointer-events-none" />

      <div className="relative z-10 max-w-5xl mx-auto px-4 py-10 animate-fade-in">
        {/* ── Header ─────────────────────────────────────────────── */}
        <header className="text-center mb-8">
          <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full glass text-xs font-medium text-slate-600 dark:text-slate-300 mb-5">
            <span className={`w-2 h-2 rounded-full ${online ? 'bg-emerald-400 animate-pulse-dot' : 'bg-red-400'}`} />
            {online ? 'Local testnet live' : 'Node offline'}
            <span className="text-slate-400 dark:text-slate-500">· 5 nodes · BFT</span>
          </div>

          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-blue-600 to-cyan-600 flex items-center justify-center text-white shadow-lg shadow-blue-600/25">
              <Compass className="w-6 h-6" />
            </div>
            <h1 className="text-4xl md:text-5xl font-bold tracking-tight text-slate-900 dark:text-white">
              Nebula
              <span className="bg-gradient-to-r from-blue-500 via-cyan-500 to-violet-500 bg-clip-text text-transparent"> Explorer</span>
            </h1>
          </div>
          <p className="text-slate-500 dark:text-slate-400 max-w-xl mx-auto">
            A live control room for network activity, validators, and staking.
          </p>

          {online && tip && (
            <div className="flex items-center justify-center gap-2 mt-5 text-xs">
              <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full glass-strong font-mono text-slate-700 dark:text-slate-200">
                <Activity className="w-3.5 h-3.5 text-blue-500 dark:text-cyan-400" />
                Tip #{fmt(tip.height)}
              </span>
              <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full glass-strong font-mono text-slate-700 dark:text-slate-200">
                <Shield className="w-3.5 h-3.5 text-emerald-500" />
                Finalized #{fmt(tip.finalized_height)}
              </span>
            </div>
          )}
        </header>

        {/* ── Tab bar ────────────────────────────────────────────── */}
        <div className="flex gap-1 p-1 rounded-2xl glass-strong mb-6 max-w-md mx-auto">
          {TABS.map(({ id, label }) => (
            <button
              key={id}
              onClick={() => setActiveTab(id)}
              className={`flex-1 px-4 py-2.5 rounded-xl text-sm font-semibold transition-all ${
                activeTab === id
                  ? 'bg-gradient-to-r from-blue-600 to-cyan-600 text-white shadow-lg shadow-blue-600/25'
                  : 'text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 hover:bg-slate-200/50 dark:hover:bg-slate-700/40'
              }`}
            >
              {label}
            </button>
          ))}
        </div>

        <div key={activeTab} className="animate-in">
          {activeTab === 'dashboard' && <DashboardTab />}
          {activeTab === 'blocks' && <BlocksTab />}
          {activeTab === 'validators' && <ValidatorsTab />}
          {activeTab === 'staking' && <StakingTab />}
        </div>
      </div>
    </div>
  )
}
