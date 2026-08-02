import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import {
  Compass, Wallet, Droplets, Vote, BookOpen, FileCode, ExternalLink,
  ArrowRight, ArrowDown, Shield, Zap, Layers, Cpu, Activity, Layers2,
  Github, Blocks, Server, Database, Workflow, Terminal, GitFork, Boxes,
} from 'lucide-react'

const API_URL = 'http://localhost:8545'

const APPS = [
  { to: '/explorer', icon: Compass, title: 'Explorer', desc: 'Live blocks, transactions, validators & staking', color: 'from-blue-500/20 to-cyan-600/10 border-blue-500/20', chip: 'from-blue-500 to-cyan-600' },
  { to: '/wallet', icon: Wallet, title: 'Wallet', desc: 'Generate Ed25519 keys, check balance, send transactions', color: 'from-emerald-500/20 to-emerald-600/10 border-emerald-500/20', chip: 'from-emerald-500 to-teal-600' },
  { to: '/faucet', icon: Droplets, title: 'Faucet', desc: 'Request test tokens for local development', color: 'from-amber-500/20 to-amber-600/10 border-amber-500/20', chip: 'from-amber-500 to-orange-600' },
  { to: '/governance', icon: Vote, title: 'Governance', desc: 'Proposals, voting, treasury & analytics', color: 'from-violet-500/20 to-violet-600/10 border-violet-500/20', chip: 'from-violet-500 to-purple-600' },
  { to: '/docs', icon: BookOpen, title: 'Docs', desc: 'Architecture, API reference, quick start guides', color: 'from-rose-500/20 to-rose-600/10 border-rose-500/20', chip: 'from-rose-500 to-pink-600' },
  { to: '/api-docs', icon: FileCode, title: 'API Docs', desc: 'Interactive Swagger playground for all 17 endpoints', color: 'from-indigo-500/20 to-indigo-600/10 border-indigo-500/20', chip: 'from-indigo-500 to-blue-600' },
  { to: '/sdk', icon: Layers, title: 'SDK Portal', desc: 'JavaScript SDK, CLI tools & contract templates', color: 'from-teal-500/20 to-teal-600/10 border-teal-500/20', chip: 'from-teal-500 to-emerald-600' },
  { to: '/developer-portal', icon: ExternalLink, title: 'Dev Portal', desc: 'Onboarding, starter kits & resources', color: 'from-fuchsia-500/20 to-fuchsia-600/10 border-fuchsia-500/20', chip: 'from-fuchsia-500 to-pink-600' },
]

const PILLARS = [
  { icon: Layers2, title: 'Modular', desc: 'Consensus, execution, networking and storage as independent, composable layers.' },
  { icon: Shield, title: 'Secure', desc: 'BFT consensus, signature-verified blocks, rate limiting and key-pair isolation.' },
  { icon: Zap, title: 'Fast', desc: 'EIP-1559 pricing, prioritized mempool and a lean, no-frills execution engine.' },
  { icon: Cpu, title: 'Developer-first', desc: 'One-click Docker testnet, SDKs, starter kits and a complete RPC surface.' },
]

const STACK = [
  { icon: Layers2, layer: 'Frontend', detail: 'React SPA · Wallet · Explorer · Governance' },
  { icon: Server, layer: 'Node RPC', detail: '17 HTTP endpoints · warp · rate-limited' },
  { icon: GitFork, layer: 'Consensus', detail: 'BFT with locking rounds · 2/3 quorum' },
  { icon: Workflow, layer: 'Execution', detail: 'EVM · EIP-1559 gas · receipts' },
  { icon: Boxes, layer: 'Networking', detail: 'libp2p · gossipsub · Kademlia DHT' },
  { icon: Database, layer: 'Storage', detail: 'RocksDB · Merkle Patricia Trie' },
]

const STEPS = [
  {
    n: '01',
    title: 'Build the node',
    desc: 'Compile the Rust workspace. The node binary drives consensus, execution, and the RPC surface.',
    code: 'cargo build -p node --release',
  },
  {
    n: '02',
    title: 'Boot the testnet',
    desc: 'Spin up 3 validators + 2 RPC nodes with monitoring and an nginx gateway.',
    code: 'docker compose -f deployment/local/docker-compose.yml up -d',
  },
  {
    n: '03',
    title: 'Start building',
    desc: 'Open the dashboard, grab test tokens from the faucet, and submit your first transaction.',
    code: 'curl http://localhost:8545/status',
  },
]

const fmt = (v) => v == null ? '--' : Number(v).toLocaleString()

export default function Home() {
  const [status, setStatus] = useState(null)
  const [validatorCount, setValidatorCount] = useState(null)
  const [online, setOnline] = useState(true)
  const [recentBlocks, setRecentBlocks] = useState([])

  useEffect(() => {
    let active = true
    const fetchAll = async () => {
      try {
        const [sr, vr] = await Promise.all([
          window.fetch(`${API_URL}/status`),
          window.fetch(`${API_URL}/validators`),
        ])
        if (!sr.ok) throw new Error('node offline')
        const [sd, vd] = await Promise.all([sr.json(), vr.ok ? vr.json() : null])
        if (!active) return
        setStatus(sd)
        setValidatorCount(vd?.validators?.length ? vd.validators.length : 3)
        setOnline(true)
      } catch { if (active) setOnline(false) }
    }
    const fetchBlocks = async () => {
      try {
        const res = await window.fetch(`${API_URL}/block/latest`)
        if (!res.ok) throw new Error('offline')
        const { block } = await res.json()
        const latest = block.header.slot
        const from = Math.max(latest - 5, 0)
        const reqs = []
        for (let s = latest; s >= from; s--) {
          reqs.push(
            window.fetch(`${API_URL}/block/${s}`)
              .then((r) => r.json())
              .then((d) => d.block)
              .catch(() => null),
          )
        }
        const blocks = (await Promise.all(reqs)).filter(Boolean)
        if (active) setRecentBlocks(blocks)
      } catch { /* keep last known blocks */ }
    }
    fetchAll()
    fetchBlocks()
    const t = setInterval(() => { fetchAll(); fetchBlocks() }, 4000)
    return () => { active = false; clearInterval(t) }
  }, [])

  const stats = [
    { label: 'Chain Height', value: fmt(status?.height), icon: Activity },
    { label: 'Finalized', value: fmt(status?.finalized_height), icon: Shield },
    { label: 'Mempool', value: fmt(status?.mempool_size), icon: Layers2 },
    { label: 'Validators', value: validatorCount ?? '--', icon: Cpu },
  ]

  return (
    <div className="relative min-h-screen overflow-hidden">
      {/* Aurora blobs */}
      <div className="absolute -top-40 -left-40 w-[42rem] h-[42rem] rounded-full opacity-30 animate-float pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(14,165,233,0.45), transparent 70%)' }} />
      <div className="absolute top-24 -right-40 w-[38rem] h-[38rem] rounded-full opacity-25 animate-float-alt pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(139,92,246,0.4), transparent 70%)' }} />
      <div className="absolute -bottom-48 left-1/3 w-[40rem] h-[40rem] rounded-full opacity-20 animate-float-slow pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(251,146,60,0.35), transparent 70%)' }} />
      <div className="absolute inset-0 bg-grid pointer-events-none" />

      <div className="relative z-10 max-w-6xl mx-auto px-4 pt-20 pb-16 animate-fade-in">
        {/* ── Hero ─────────────────────────────────────────────── */}
        <div className="text-center mb-16">
          <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full glass text-xs font-medium text-slate-600 dark:text-slate-300 mb-6">
            <span className={`w-2 h-2 rounded-full ${online ? 'bg-emerald-400 animate-pulse-dot' : 'bg-red-400'}`} />
            {online ? 'Local testnet live' : 'Node offline'}
            <span className="text-slate-400 dark:text-slate-500">· 5 nodes · BFT</span>
          </div>

          <h1 className="text-5xl md:text-7xl font-bold tracking-tight text-slate-900 dark:text-white mb-6">
            Every blockchain tool
            <span className="block mt-1 bg-gradient-to-r from-blue-500 via-cyan-500 to-violet-500 bg-clip-text text-transparent">
              in one place.
            </span>
          </h1>

          <p className="text-slate-500 dark:text-slate-400 text-lg max-w-2xl mx-auto mb-8">
            Nebula is a high-performance modular blockchain. Spin up a full
            local testnet, explore blocks, manage wallets, and ship smart
            contracts — all from one dashboard.
          </p>

          <div className="flex flex-wrap items-center justify-center gap-3 mb-10">
            <Link to="/explorer" className="group inline-flex items-center gap-2 px-6 py-3 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 text-white text-sm font-semibold shadow-lg shadow-blue-600/25 hover:shadow-blue-600/40 hover:-translate-y-0.5 transition-all">
              <Compass className="w-4 h-4" />
              Launch Explorer
              <ArrowRight className="w-4 h-4 group-hover:translate-x-0.5 transition-transform" />
            </Link>
            <Link to="/docs" className="inline-flex items-center gap-2 px-6 py-3 rounded-xl glass text-sm font-semibold text-slate-700 dark:text-slate-200 hover:-translate-y-0.5 transition-all">
              <BookOpen className="w-4 h-4" />
              Read the Docs
            </Link>
            <a href="https://github.com/fewzfewz/scratchBlockchain" target="_blank" rel="noopener noreferrer"
              className="inline-flex items-center gap-2 px-6 py-3 rounded-xl glass text-sm font-semibold text-slate-700 dark:text-slate-200 hover:-translate-y-0.5 transition-all">
              <Github className="w-4 h-4" />
              GitHub
            </a>
          </div>

          {/* Live stats strip */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3 max-w-3xl mx-auto">
            {stats.map(({ label, value, icon: Icon }) => (
              <div key={label} className="p-4 rounded-2xl glass-strong hover:-translate-y-0.5 transition-all">
                <div className="flex items-center justify-center gap-1.5 mb-2">
                  <Icon className="w-3.5 h-3.5 text-blue-500 dark:text-cyan-400" />
                  <span className="text-[11px] uppercase tracking-wider text-slate-500 dark:text-slate-400">{label}</span>
                </div>
                <div className="text-2xl font-bold text-slate-900 dark:text-white tabular-nums">{value}</div>
              </div>
            ))}
          </div>
        </div>

        {/* ── Live block feed ──────────────────────────────────── */}
        <div className="mb-16">
          <div className="flex items-end justify-between mb-5">
            <div>
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white">Live chain activity</h2>
              <p className="text-sm text-slate-500 dark:text-slate-400 mt-1">Blocks being finalized by the testnet right now.</p>
            </div>
            <Link to="/explorer" className="hidden sm:inline-flex items-center gap-1.5 text-sm font-medium text-blue-600 dark:text-cyan-400 hover:underline">
              View all <ArrowRight className="w-3.5 h-3.5" />
            </Link>
          </div>

          {recentBlocks.length > 0 ? (
            <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-3">
              {recentBlocks.map((b) => (
                <div key={b.header.slot} className="p-4 rounded-2xl glass hover:bg-slate-100/70 dark:hover:bg-slate-800/50 transition-all">
                  <div className="flex items-center justify-between mb-3">
                    <span className="font-mono text-sm font-bold text-slate-900 dark:text-white">
                      #{fmt(b.header.slot)}
                    </span>
                    <span className="inline-flex items-center gap-1.5 text-[11px] text-slate-500 dark:text-slate-400">
                      <Blocks className="w-3.5 h-3.5" />
                      {b.extrinsics?.length ?? 0} txns
                    </span>
                  </div>
                  <div className="grid grid-cols-2 gap-2">
                    <div className="rounded-lg bg-slate-100/70 dark:bg-slate-700/30 px-3 py-2">
                      <div className="text-[10px] uppercase tracking-wider text-slate-400 dark:text-slate-500">Base fee</div>
                      <div className="text-sm font-semibold text-slate-800 dark:text-slate-200 tabular-nums">{fmt(b.header.base_fee)}</div>
                    </div>
                    <div className="rounded-lg bg-slate-100/70 dark:bg-slate-700/30 px-3 py-2">
                      <div className="text-[10px] uppercase tracking-wider text-slate-400 dark:text-slate-500">Gas used</div>
                      <div className="text-sm font-semibold text-slate-800 dark:text-slate-200 tabular-nums">{fmt(b.header.gas_used)}</div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="p-8 rounded-2xl glass text-center text-sm text-slate-400 dark:text-slate-500">
              {online ? 'Fetching blocks…' : 'Node unreachable — start the testnet to see live blocks.'}
            </div>
          )}
        </div>

        {/* ── App grid ─────────────────────────────────────────── */}
        <div className="mb-16">
          <div className="mb-5">
            <h2 className="text-2xl font-bold text-slate-900 dark:text-white">Explore the network</h2>
            <p className="text-sm text-slate-500 dark:text-slate-400 mt-1">Pick a tool and start building.</p>
          </div>

          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-4">
            {APPS.map(({ to, icon: Icon, title, desc, color, chip }) => (
              <Link key={to} to={to}
                className={`group relative p-5 rounded-2xl bg-gradient-to-br ${color} border backdrop-blur-sm hover:scale-[1.02] hover:shadow-xl hover:shadow-blue-500/10 transition-all`}>
                <div className={`w-11 h-11 rounded-xl bg-gradient-to-br ${chip} flex items-center justify-center text-white shadow-lg mb-4 group-hover:scale-110 transition-transform`}>
                  <Icon className="w-5 h-5" />
                </div>
                <h3 className="text-base font-semibold text-slate-800 dark:text-white mb-1">{title}</h3>
                <p className="text-sm text-slate-500 dark:text-slate-400 leading-relaxed">{desc}</p>
                <ArrowRight className="w-4 h-4 text-slate-400 dark:text-slate-500 mt-4 -translate-x-1 opacity-0 group-hover:translate-x-0 group-hover:opacity-100 transition-all" />
              </Link>
            ))}
          </div>
        </div>

        {/* ── Pillars ──────────────────────────────────────────── */}
        <div className="mb-16">
          <div className="text-center mb-8">
            <p className="text-xs uppercase tracking-widest text-blue-500 dark:text-blue-400 font-medium mb-2">Why Nebula</p>
            <h2 className="text-2xl md:text-3xl font-bold text-slate-900 dark:text-white">Built for developers</h2>
          </div>
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-4">
            {PILLARS.map(({ icon: Icon, title, desc }) => (
              <div key={title} className="p-5 rounded-2xl glass hover:bg-slate-100/60 dark:hover:bg-slate-800/50 transition-all">
                <div className="w-10 h-10 rounded-xl bg-blue-500/10 dark:bg-blue-500/15 border border-blue-500/20 flex items-center justify-center mb-3">
                  <Icon className="w-5 h-5 text-blue-600 dark:text-cyan-400" />
                </div>
                <h3 className="text-sm font-semibold text-slate-800 dark:text-white mb-1.5">{title}</h3>
                <p className="text-sm text-slate-500 dark:text-slate-400 leading-relaxed">{desc}</p>
              </div>
            ))}
          </div>
        </div>

        {/* ── Architecture ─────────────────────────────────────── */}
        <div className="mb-16">
          <div className="text-center mb-8">
            <p className="text-xs uppercase tracking-widest text-blue-500 dark:text-blue-400 font-medium mb-2">Under the hood</p>
            <h2 className="text-2xl md:text-3xl font-bold text-slate-900 dark:text-white">Modular by design</h2>
          </div>

          <div className="max-w-2xl mx-auto">
            {STACK.map(({ icon: Icon, layer, detail }, i) => (
              <div key={layer}>
                <div className="flex items-center gap-4 p-4 rounded-2xl glass-strong hover:-translate-y-0.5 transition-all">
                  <div className="w-10 h-10 shrink-0 rounded-xl bg-gradient-to-br from-blue-500 to-cyan-600 flex items-center justify-center text-white shadow-lg">
                    <Icon className="w-5 h-5" />
                  </div>
                  <div className="flex-1">
                    <div className="text-sm font-semibold text-slate-800 dark:text-white">{layer}</div>
                    <div className="text-xs text-slate-500 dark:text-slate-400 mt-0.5">{detail}</div>
                  </div>
                  <span className="text-[10px] font-mono text-slate-400 dark:text-slate-500">{String(i + 1).padStart(2, '0')}</span>
                </div>
                {i < STACK.length - 1 && (
                  <div className="flex justify-center py-1">
                    <ArrowDown className="w-4 h-4 text-slate-300 dark:text-slate-600" />
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>

        {/* ── Getting started ──────────────────────────────────── */}
        <div className="mb-16">
          <div className="text-center mb-8">
            <p className="text-xs uppercase tracking-widest text-blue-500 dark:text-blue-400 font-medium mb-2">Quick start</p>
            <h2 className="text-2xl md:text-3xl font-bold text-slate-900 dark:text-white">Up and running in minutes</h2>
          </div>

          <div className="grid md:grid-cols-3 gap-4">
            {STEPS.map(({ n, title, desc, code }) => (
              <div key={n} className="p-5 rounded-2xl glass hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all">
                <div className="flex items-center gap-3 mb-3">
                  <span className="font-mono text-lg font-bold bg-gradient-to-r from-blue-500 to-cyan-600 bg-clip-text text-transparent">{n}</span>
                  <span className="w-px h-6 bg-slate-200 dark:bg-slate-700" />
                  <span className="text-sm font-semibold text-slate-800 dark:text-white">{title}</span>
                </div>
                <p className="text-sm text-slate-500 dark:text-slate-400 leading-relaxed mb-4">{desc}</p>
                <div className="flex items-center gap-2 rounded-xl bg-slate-900 dark:bg-slate-950 px-3 py-2.5">
                  <Terminal className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                  <code className="text-[11px] text-slate-200 truncate">{code}</code>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* ── Footer CTA ───────────────────────────────────────── */}
        <div className="rounded-3xl p-8 md:p-12 text-center glass-strong relative overflow-hidden">
          <div className="absolute -top-24 left-1/2 -translate-x-1/2 w-96 h-96 rounded-full opacity-20 pointer-events-none"
            style={{ background: 'radial-gradient(circle, rgba(14,165,233,0.5), transparent 70%)' }} />
          <h2 className="relative text-2xl md:text-3xl font-bold text-slate-900 dark:text-white mb-3">Ready to explore?</h2>
          <p className="relative text-slate-500 dark:text-slate-400 max-w-xl mx-auto mb-6">
            Jump into the explorer, grab test tokens from the faucet, or read the
            architecture docs to understand how Nebula fits together.
          </p>
          <div className="relative flex flex-wrap items-center justify-center gap-3">
            <Link to="/wallet" className="inline-flex items-center gap-2 px-6 py-3 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 text-white text-sm font-semibold shadow-lg shadow-blue-600/25 hover:shadow-blue-600/40 hover:-translate-y-0.5 transition-all">
              <Wallet className="w-4 h-4" />
              Create a Wallet
            </Link>
            <Link to="/faucet" className="inline-flex items-center gap-2 px-6 py-3 rounded-xl glass text-sm font-semibold text-slate-700 dark:text-slate-200 hover:-translate-y-0.5 transition-all">
              <Droplets className="w-4 h-4" />
              Get Test Tokens
            </Link>
          </div>
        </div>
      </div>
    </div>
  )
}
