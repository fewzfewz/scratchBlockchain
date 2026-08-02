import { useState, useEffect } from 'react'
import {
  BookOpen, Zap, Shield, Layers, Code, FileText, Copy, Check, FileJson,
  Search, ChevronLeft, ChevronRight, Activity, Gauge, ShieldCheck, WifiOff,
  Terminal, Server, Download, User, ArrowLeftRight, AlertTriangle,
  ArrowRight, Rocket, Compass,
} from 'lucide-react'

const RPC_URL = 'http://localhost:8545'

const SECTIONS = [
  { id: 'intro', title: 'Introduction', icon: BookOpen },
  { id: 'install', title: 'Installation', icon: Download },
  { id: 'quickstart', title: 'Quick Start', icon: Zap },
  { id: 'architecture', title: 'Architecture', icon: Layers },
  { id: 'accounts', title: 'Accounts', icon: User },
  { id: 'transactions', title: 'Transactions', icon: ArrowLeftRight },
  { id: 'rpc', title: 'RPC API', icon: Terminal },
  { id: 'errors', title: 'Error Codes', icon: AlertTriangle },
  { id: 'local-dev', title: 'Local Dev', icon: Server },
]

const fmt = (v) => v == null || isNaN(Number(v)) ? '--' : Number(v).toLocaleString()

function CopyBtn({ text }) {
  const [copied, setCopied] = useState(false)
  return (
    <button
      onClick={() => { navigator.clipboard.writeText(text); setCopied(true); setTimeout(() => setCopied(false), 2000) }}
      className="text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 transition-colors"
    >
      {copied ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
    </button>
  )
}

const LANG_STYLES = {
  bash: { dot: 'bg-emerald-400', badge: 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400' },
  text: { dot: 'bg-slate-400', badge: 'bg-slate-500/15 text-slate-600 dark:text-slate-400' },
}

function CodeBlock({ code, lang = '' }) {
  const style = LANG_STYLES[lang] || { dot: 'bg-blue-400', badge: 'bg-blue-500/15 text-blue-600 dark:text-blue-400' }
  return (
    <div className="group rounded-xl border border-slate-200 dark:border-slate-700/50 bg-slate-50 dark:bg-slate-800/80 overflow-hidden mb-4 hover:border-slate-300 dark:hover:border-slate-600/60 transition-colors">
      {lang && (
        <div className="flex items-center justify-between px-4 py-1.5 bg-slate-100 dark:bg-black/30 border-b border-slate-200 dark:border-slate-700/50">
          <span className="flex items-center gap-2">
            <span className={`w-1.5 h-1.5 rounded-full ${style.dot}`} />
            <span className={`text-[10px] uppercase tracking-wider font-semibold px-1.5 py-0.5 rounded ${style.badge}`}>{lang}</span>
          </span>
          <CopyBtn text={code} />
        </div>
      )}
      <pre className="p-4 overflow-x-auto text-sm text-slate-700 dark:text-slate-300 font-mono leading-relaxed scrollbar-thin"><code>{code}</code></pre>
    </div>
  )
}

function DocsSkeleton() {
  return (
    <div className="space-y-5 animate-fade-in">
      <div className="h-10 w-2/3 rounded-xl glass-strong animate-pulse" />
      <div className="h-4 w-full rounded-lg glass-strong animate-pulse" />
      <div className="h-4 w-5/6 rounded-lg glass-strong animate-pulse" />
      <div className="grid md:grid-cols-3 gap-4">
        {[0, 1, 2].map(i => <div key={i} className="h-28 rounded-xl glass-strong animate-pulse" />)}
      </div>
      <div className="h-40 rounded-xl glass-strong animate-pulse" />
      <div className="h-4 w-3/4 rounded-lg glass-strong animate-pulse" />
      <div className="h-4 w-1/2 rounded-lg glass-strong animate-pulse" />
    </div>
  )
}

function StartHere({ onSelect }) {
  const links = [
    { id: 'install', title: 'Install Nebula', desc: 'Build from source or use Docker Compose', icon: Download, chip: 'from-blue-600 to-cyan-600' },
    { id: 'quickstart', title: 'Quick Start', desc: 'Spin up a 3-node testnet in 60 seconds', icon: Zap, chip: 'from-amber-500 to-orange-600' },
    { id: 'rpc', title: 'RPC API', desc: 'All 17 endpoints with example responses', icon: Terminal, chip: 'from-emerald-600 to-teal-600' },
    { id: 'local-dev', title: 'Local Dev', desc: 'Port map, test accounts, and workflows', icon: Server, chip: 'from-violet-600 to-purple-600' },
  ]
  return (
    <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-3 mt-8">
      {links.map(({ id, title, desc, icon: Icon, chip }) => (
        <button
          key={id}
          onClick={() => onSelect(id)}
          className="text-left p-4 rounded-2xl glass-strong hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all group"
        >
          <div className="flex items-center justify-between mb-3">
            <div className={`w-8 h-8 rounded-lg bg-gradient-to-br ${chip} flex items-center justify-center text-white shadow-md`}>
              <Icon className="w-4 h-4" />
            </div>
            <ArrowRight className="w-4 h-4 text-slate-400 group-hover:text-blue-400 group-hover:translate-x-0.5 transition-all" />
          </div>
          <div className="text-sm font-semibold text-slate-800 dark:text-white">{title}</div>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">{desc}</p>
        </button>
      ))}
    </div>
  )
}

export default function DocsPage() {
  const [activeSection, setActiveSection] = useState('intro')
  const [query, setQuery] = useState('')
  const [progress, setProgress] = useState(0)
  const [backendOnline, setBackendOnline] = useState(true)
  const [network, setNetwork] = useState(null)
  const [loading, setLoading] = useState(true)

  // Node health + network info polling (matches Explorer/Faucet/Governance)
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

  // Brief skeleton so the page feels live
  useEffect(() => {
    const t = setTimeout(() => setLoading(false), 350)
    return () => clearTimeout(t)
  }, [])

  // Scroll to top on section change
  useEffect(() => {
    window.scrollTo({ top: 0, behavior: 'smooth' })
  }, [activeSection])

  // Reading progress bar
  useEffect(() => {
    const onScroll = () => {
      const el = document.documentElement
      const max = el.scrollHeight - el.clientHeight
      setProgress(max > 0 ? (el.scrollTop / max) * 100 : 0)
    }
    window.addEventListener('scroll', onScroll, { passive: true })
    return () => window.removeEventListener('scroll', onScroll)
  }, [])

  // Keyboard navigation (skip inputs)
  useEffect(() => {
    const onKey = (e) => {
      if (e.target && (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA')) return
      if (e.key === 'ArrowDown') { e.preventDefault(); setActiveSection(prev => SECTIONS[Math.min(SECTIONS.findIndex(s => s.id === prev) + 1, SECTIONS.length - 1)].id) }
      if (e.key === 'ArrowUp') { e.preventDefault(); setActiveSection(prev => SECTIONS[Math.max(SECTIONS.findIndex(s => s.id === prev) - 1, 0)].id) }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  const filteredSections = SECTIONS.filter(s =>
    s.title.toLowerCase().includes(query.trim().toLowerCase())
  )

  const idx = SECTIONS.findIndex(s => s.id === activeSection)
  const prev = idx > 0 ? SECTIONS[idx - 1] : null
  const next = idx < SECTIONS.length - 1 ? SECTIONS[idx + 1] : null

  const select = (id) => { setActiveSection(id); setQuery('') }

  return (
    <div className="relative min-h-screen overflow-hidden">
      {/* Reading progress bar */}
      <div className="fixed top-0 left-0 right-0 z-50 h-0.5 bg-transparent pointer-events-none">
        <div
          className="h-full bg-gradient-to-r from-blue-500 via-cyan-500 to-violet-500 transition-[width] duration-150"
          style={{ width: `${progress}%` }}
        />
      </div>

      {/* Aurora blobs + grid backdrop */}
      <div className="absolute -top-40 -left-40 w-[38rem] h-[38rem] rounded-full opacity-25 animate-float pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(14,165,233,0.45), transparent 70%)' }} />
      <div className="absolute top-40 -right-40 w-[34rem] h-[34rem] rounded-full opacity-20 animate-float-alt pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(139,92,246,0.4), transparent 70%)' }} />
      <div className="absolute inset-0 bg-grid pointer-events-none" />

      <div className="relative z-10 max-w-6xl mx-auto px-4 py-8 md:py-10 animate-fade-in">
        {/* ── Header ─────────────────────────────────────────────── */}
        <header className="text-center mb-8">
          <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full glass text-xs font-medium text-slate-600 dark:text-slate-300 mb-5">
            <span className={`w-2 h-2 rounded-full ${backendOnline ? 'bg-emerald-400 animate-pulse-dot' : 'bg-red-400'}`} />
            {backendOnline ? 'Local testnet live' : 'Node offline'}
            <span className="text-slate-400 dark:text-slate-500">· Docs v1.0</span>
          </div>

          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-blue-600 to-cyan-600 flex items-center justify-center text-white shadow-lg shadow-blue-600/25">
              <BookOpen className="w-6 h-6" />
            </div>
            <h1 className="text-4xl md:text-5xl font-bold tracking-tight text-slate-900 dark:text-white">
              Nebula
              <span className="bg-gradient-to-r from-blue-500 via-cyan-500 to-violet-500 bg-clip-text text-transparent"> Docs</span>
            </h1>
          </div>
          <p className="text-slate-500 dark:text-slate-400 max-w-xl mx-auto">
            Everything you need to build on Nebula — from zero to a live local testnet in minutes.
          </p>

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
            <span>Node not reachable at {RPC_URL}. The docs still work — start the testnet to try the live endpoints.</span>
          </div>
        )}

        <div className="grid lg:grid-cols-[260px_1fr] gap-8 items-start">
          {/* ── Sidebar ──────────────────────────────────────────── */}
          <aside className="lg:sticky lg:top-8">
            <div className="hidden lg:block space-y-4">
              <div className="p-3 rounded-2xl glass-strong">
                <div className="relative">
                  <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-slate-400" />
                  <input
                    value={query}
                    onChange={e => setQuery(e.target.value)}
                    placeholder="Search docs..."
                    className="w-full pl-9 pr-8 py-2 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-700 dark:text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500/50 focus:ring-1 focus:ring-blue-500/20 transition-all"
                  />
                  {query && (
                    <button onClick={() => setQuery('')} className="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200">
                      <ChevronRight className="w-3.5 h-3.5 rotate-90" />
                    </button>
                  )}
                </div>
              </div>

              <nav className="p-2 rounded-2xl glass-strong space-y-0.5">
                {filteredSections.length === 0 && (
                  <p className="px-3 py-2 text-xs text-slate-500 dark:text-slate-400">No sections match "{query.trim()}"</p>
                )}
                {filteredSections.map(({ id, title, icon: Icon }, i) => (
                  <button
                    key={id}
                    onClick={() => select(id)}
                    className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-xl text-sm transition-all ${
                      activeSection === id
                        ? 'bg-gradient-to-r from-blue-600 to-cyan-600 text-white shadow-md shadow-blue-600/20'
                        : 'text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 hover:bg-slate-200/50 dark:hover:bg-slate-700/40'
                    }`}
                  >
                    <Icon className="w-4 h-4 shrink-0" />
                    <span className="truncate flex-1 text-left">{title}</span>
                    <span className={`text-[10px] font-mono ${activeSection === id ? 'text-white/70' : 'text-slate-400 dark:text-slate-500'}`}>{String(i + 1).padStart(2, '0')}</span>
                  </button>
                ))}
              </nav>

              <div className="p-3 rounded-2xl glass text-xs text-slate-500 dark:text-slate-400">
                <div className="flex items-center justify-between mb-1.5">
                  <span className="flex items-center gap-1.5 font-medium text-slate-600 dark:text-slate-300">
                    <Compass className="w-3.5 h-3.5 text-blue-500 dark:text-cyan-400" />
                    Section {idx + 1} of {SECTIONS.length}
                  </span>
                  <span className="font-mono">{Math.round(progress)}%</span>
                </div>
                <div className="h-1.5 rounded-full bg-slate-200 dark:bg-slate-700/50 overflow-hidden">
                  <div className="h-full bg-gradient-to-r from-blue-500 via-cyan-500 to-violet-500 transition-all duration-150" style={{ width: `${(idx + 1) / SECTIONS.length * 100}%` }} />
                </div>
                <p className="mt-2 text-[10px] text-slate-400 dark:text-slate-500">Tip: use ↑ / ↓ keys to navigate sections</p>
              </div>
            </div>

            {/* Mobile section chips */}
            <div className="lg:hidden -mx-4 px-4 mb-6 flex gap-2 overflow-x-auto scrollbar-thin pb-1">
              {SECTIONS.map(({ id, title }) => (
                <button
                  key={id}
                  onClick={() => setActiveSection(id)}
                  className={`shrink-0 px-3.5 py-1.5 rounded-full text-xs font-medium transition-all ${
                    activeSection === id
                      ? 'bg-gradient-to-r from-blue-600 to-cyan-600 text-white shadow-md shadow-blue-600/20'
                      : 'glass-strong text-slate-500 dark:text-slate-400'
                  }`}
                >
                  {title}
                </button>
              ))}
            </div>
          </aside>

          {/* ── Content ──────────────────────────────────────────── */}
          <main className="min-w-0">
          {loading ? <DocsSkeleton /> : (
          <div key={activeSection} className="animate-slide-up">
          {activeSection === 'intro' && (
            <section>
              <h1 className="text-4xl font-bold text-slate-900 dark:text-white mb-4 bg-gradient-to-r from-blue-500 via-cyan-500 to-violet-500 bg-clip-text text-transparent">Introduction to Nebula</h1>
              <p className="text-lg text-blue-600 dark:text-blue-300 mb-4">A high-performance, modular blockchain built in Rust.</p>
              <p className="text-slate-600 dark:text-slate-400 mb-8 leading-relaxed">Nebula is designed for scalability and flexibility, featuring a multi-VM execution environment, validator-based consensus with instant finality, and built-in Layer 2 support.</p>
              <div className="grid md:grid-cols-3 gap-4">
                {[
                  { icon: Zap, title: 'High Performance', desc: 'Parallel transaction execution and high-throughput mempool.' },
                  { icon: Shield, title: 'Secure', desc: 'Ed25519 signatures and GRANDPA-style finality gadget.' },
                  { icon: Layers, title: 'Modular', desc: 'Pluggable consensus, execution, and data availability layers.' },
                ].map(({ icon: Icon, title, desc }) => (
                  <div key={title} className="p-5 rounded-xl glass-strong hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all">
                    <div className="w-9 h-9 rounded-lg bg-gradient-to-br from-blue-600 to-cyan-600 flex items-center justify-center text-white shadow-md mb-3">
                      <Icon className="w-4 h-4" />
                    </div>
                    <h4 className="text-slate-800 dark:text-white font-semibold mb-1">{title}</h4>
                    <p className="text-sm text-slate-500 dark:text-slate-400">{desc}</p>
                  </div>
                ))}
              </div>

              <h2 className="text-xl font-bold text-slate-900 dark:text-white mt-10 mb-1 flex items-center gap-2">
                <Rocket className="w-5 h-5 text-blue-500 dark:text-cyan-400" />
                Start here
              </h2>
              <p className="text-sm text-slate-500 dark:text-slate-400 mb-2">Jump straight to the section you need.</p>
              <StartHere onSelect={select} />
            </section>
          )}

          {activeSection === 'install' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">Installation</h2>
              <p className="text-slate-600 dark:text-slate-400 mb-4">Prerequisites: Rust (latest stable) and Docker.</p>
              <CodeBlock lang="bash" code={`# Clone the repository
git clone https://github.com/fewzfewz/scratchBlockchain.git
cd nebula

# Build release binary
cargo build --release

# Or run with Docker Compose (recommended for local dev)
docker compose up -d`} />
              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">System Requirements</h3>
              <ul className="text-sm text-slate-600 dark:text-slate-400 space-y-1 list-disc list-inside mb-4">
                <li>Rust 1.80+ (stable toolchain)</li>
                <li>Docker Engine 24+ with Compose v2</li>
                <li>4 GB RAM minimum (8 GB recommended for devnet)</li>
                <li>Node.js 18+ (for frontends)</li>
              </ul>
            </section>
          )}

          {activeSection === 'quickstart' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">Quick Start</h2>
              <p className="text-slate-600 dark:text-slate-400 mb-4">Deploy a local 3-node testnet in under 60 seconds.</p>

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">1. Single-node devnet</h3>
              <CodeBlock lang="bash" code={`./scripts/deploy.sh

# Check node status
curl http://localhost:8545/status

# Expected response:
# {"height":42,"peers":2,"uptime_secs":120,"consensus":"BFT","finalized":41}`} />

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">2. Multi-node testnet</h3>
              <CodeBlock lang="bash" code={`cd deployment/local
docker-compose up -d

# Each node exposes an API:
# validator1: http://localhost:8545
# validator2: http://localhost:8546
# validator3: http://localhost:8547
# rpc1:       http://localhost:8548
# rpc2:       http://localhost:8549`} />

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">3. Verify consensus</h3>
              <CodeBlock lang="bash" code={`# Check all nodes are on the same block height
for port in 8545 8546 8547 8548 8549; do
  echo "Node $port: $(curl -s http://localhost:$port/status | grep -o '"height":[0-9]*')"
done`} />
            </section>
          )}

          {activeSection === 'architecture' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">Architecture</h2>
              <div className="space-y-6">
                <div className="p-5 rounded-xl glass-strong">
                  <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Consensus Layer</h3>
                  <p className="text-sm text-slate-600 dark:text-slate-400 leading-relaxed">Validator-based BFT consensus with Ed25519 signatures and GRANDPA-style finality gadget. Blocks are finalized in batches for instant irreversibility. Validator set is managed through on-chain governance.</p>
                  <div className="mt-3 grid grid-cols-3 gap-3 text-xs">
                    {[
                      ['Finality', '~2s (batch)'],
                      ['Block time', '1s'],
                      ['Max validators', '32 (configurable)'],
                    ].map(([k, v]) => (
                      <div key={k} className="p-2 rounded-lg bg-slate-100 dark:bg-slate-700/40 text-center">
                        <span className="text-slate-500 dark:text-slate-400">{k}</span>
                        <br />
                        <span className="font-semibold text-slate-800 dark:text-slate-200">{v}</span>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="p-5 rounded-xl glass-strong">
                  <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Networking Layer</h3>
                  <p className="text-sm text-slate-600 dark:text-slate-400 mb-3">P2P discovery via libp2p with Gossipsub for block and transaction propagation.</p>
                  <ul className="text-sm text-slate-600 dark:text-slate-400 space-y-1 list-disc list-inside">
                    <li>Peer reputation system prevents spam and DoS</li>
                    <li>Automatic NAT traversal with hole-punching</li>
                    <li>Encrypted transport with Noise protocol</li>
                    <li>Configurable gossip topics for scalability</li>
                  </ul>
                </div>

                <div className="p-5 rounded-xl glass-strong">
                  <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Storage Layer</h3>
                  <p className="text-sm text-slate-600 dark:text-slate-400 mb-3">High-performance storage layer using RocksDB (default) with Sled legacy support.</p>
                  <ul className="text-sm text-slate-600 dark:text-slate-400 space-y-1 list-disc list-inside">
                    <li>Trie-based state storage for efficient Merkle proofs</li>
                    <li>Pruning: configurable archive vs. recent-only modes</li>
                    <li>Snapshot-based state sync for fast node bootstrap</li>
                    <li>Column families for separate block/trie/metadata stores</li>
                  </ul>
                </div>
              </div>
            </section>
          )}

          {activeSection === 'accounts' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">Accounts</h2>
              <p className="text-slate-600 dark:text-slate-400 mb-6">Nebula uses Ed25519 key pairs for account identity. Addresses are derived from the public key.</p>

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Key Format</h3>
              <p className="text-sm text-slate-600 dark:text-slate-400 mb-3">Each account is identified by a 20-byte address (truncated SHA-256 hash of the Ed25519 public key). The full 32-byte public key is used for signature verification.</p>
              <CodeBlock lang="text" code={`Public key:  0x02aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899 (32 bytes)
Address:      0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18 (20 bytes)
Private key:  0xdeadbeef... (64 bytes, keep secret!)`} />

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Account State</h3>
              <div className="overflow-x-auto mb-4 rounded-2xl glass-strong">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-slate-200 dark:border-slate-700">
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Field</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Type</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Description</th>
                    </tr>
                  </thead>
                  <tbody className="text-slate-700 dark:text-slate-300">
                    {[
                      ['nonce', 'u64', 'Transaction count (increments with each tx)'],
                      ['balance', 'U256', 'Account balance in wei'],
                      ['storage_root', 'H256', 'Merkle root of contract storage (EOA: zero)'],
                      ['code_hash', 'H256', 'Keccak hash of contract bytecode (EOA: zero)'],
                    ].map(([field, type, desc]) => (
                      <tr key={field} className="border-b border-slate-100 dark:border-slate-700/50">
                        <td className="py-2 font-mono text-xs">{field}</td>
                        <td className="py-2 text-xs text-slate-500 dark:text-slate-400">{type}</td>
                        <td className="py-2 text-xs text-slate-500 dark:text-slate-400">{desc}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </section>
          )}

          {activeSection === 'transactions' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">Transactions</h2>
              <p className="text-slate-600 dark:text-slate-400 mb-6">Nebula uses an EIP-1559 style transaction format with Ed25519 signatures.</p>

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Transaction Fields</h3>
              <div className="overflow-x-auto mb-6 rounded-2xl glass-strong">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-slate-200 dark:border-slate-700">
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Field</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Type</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Description</th>
                    </tr>
                  </thead>
                  <tbody className="text-slate-700 dark:text-slate-300">
                    {[
                      ['sender', '[u8; 20]', 'Sender address (derived from public key)'],
                      ['to', '[u8; 20]', 'Recipient address (or empty for contract deploy)'],
                      ['nonce', 'u64', 'Sender nonce (prevents replay attacks)'],
                      ['value', 'U256', 'Amount to transfer in wei'],
                      ['gas_limit', 'u64', 'Maximum gas units for this tx'],
                      ['max_fee_per_gas', 'u128', 'Maximum total fee per gas unit (wei)'],
                      ['max_priority_fee_per_gas', 'u128', 'Maximum tip for validators (wei)'],
                      ['payload', 'bytes', 'Calldata (for contract calls) or bytecode (for deploy)'],
                      ['chain_id', 'u64', 'Chain identifier (prevents cross-chain replay)'],
                      ['signature', '[u8; 64]', 'Ed25519 signature over the tx hash'],
                    ].map(([field, type, desc]) => (
                      <tr key={field} className="border-b border-slate-100 dark:border-slate-700/50">
                        <td className="py-2 font-mono text-xs">{field}</td>
                        <td className="py-2 text-xs text-slate-500 dark:text-slate-400">{type}</td>
                        <td className="py-2 text-xs text-slate-500 dark:text-slate-400">{desc}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Submit Transaction</h3>
              <CodeBlock lang="bash" code={`curl -X POST http://localhost:8545/submit_tx \\
  -H "Content-Type: application/json" \\
  -d '{
    "sender": [116,55,...],
    "to": [116,...],
    "nonce": 0,
    "value": 1000000000000000000,
    "gas_limit": 21000,
    "max_fee_per_gas": 1000000000,
    "max_priority_fee_per_gas": 100000000,
    "payload": [],
    "chain_id": 1,
    "signature": [1,2,...]
  }'`} />
            </section>
          )}

          {activeSection === 'rpc' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">RPC API</h2>
              <p className="text-slate-600 dark:text-slate-400 mb-6">Interact with the node via HTTP JSON-RPC on port 8545. All endpoints return JSON responses.</p>

              <div className="flex flex-wrap items-center gap-3 mb-6">
                <p className="text-sm text-slate-600 dark:text-slate-400">An OpenAPI 3.0 spec is available at <code className="text-xs text-blue-600 dark:text-blue-400">docs/openapi.yaml</code>.</p>
                <a href="/api-docs"
                  className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-blue-600/10 text-blue-600 dark:text-blue-400 text-xs font-medium hover:bg-blue-600/20 transition-all">
                  <FileJson className="w-3.5 h-3.5" />
                  Interactive API Docs
                </a>
              </div>

              <div className="space-y-4 mb-8">
                {[
                  { method: 'GET', path: '/status', desc: 'Node status: height, finalized height, mempool size, peer count.', real: '{"height":0,"finalized_height":0,"mempool_size":0,"peer_count":0}' },
                  { method: 'GET', path: '/health', desc: 'Health check (returns 200 when node is operational).', real: '{"status":"healthy","version":"0.1.0"}' },
                  { method: 'GET', path: '/balance/{address}', desc: 'Query account balance and nonce by 20-byte hex address.', real: '{"address":"0x...","balance":"1000000000000000000","nonce":0}' },
                  { method: 'GET', path: '/block/{height}', desc: 'Get block by number (use 0 for genesis).', real: '{"block":{...},"error":null}' },
                  { method: 'GET', path: '/block/latest', desc: 'Get the most recent block.', real: '{"block":{...},"error":null}' },
                  { method: 'GET', path: '/block/hash/{hash}', desc: 'Get block by 32-byte hex hash.', real: '{"block":{...},"error":null}' },
                  { method: 'GET', path: '/tx/{hash}', desc: 'Get transaction receipt by 32-byte hex hash.', real: '{"receipt":{...},"error":null}' },
                  { method: 'GET', path: '/mempool', desc: 'Pending transactions in the mempool queue.', real: '{"size":0,"transactions":[]}' },
                  { method: 'GET', path: '/gas_price', desc: 'EIP-1559 gas price suggestions (base + 3 priority tiers).', real: '{"base_fee":"1000000000","suggested_priority_fee_low":"1000000000","suggested_priority_fee_medium":"2000000000","suggested_priority_fee_high":"5000000000","block_height":0}' },
                  { method: 'GET', path: '/fee_history/{count}', desc: 'Historical base fee and gas used ratio for last N blocks (max 100).', real: '{"base_fee_per_gas":[],"gas_used_ratio":[],"oldest_block":0}' },
                  { method: 'POST', path: '/estimate_gas', desc: 'Estimate gas for a transaction. Body: {from, to, data}.', real: '{"estimated_gas":23100,"base_fee":"1000000000","total_cost_estimate":"23100000000000","estimated_priority_fee":"100000000"}' },
                  { method: 'GET', path: '/validators', desc: 'Active validator set from on-chain state.', real: '{"validators":[],"count":0}' },
                  { method: 'GET', path: '/delegations/{address}', desc: 'Staking delegations for an address.', real: '{"delegations":[],"address":"0x..."}' },
                  { method: 'GET', path: '/peers', desc: 'Connected peers list.', real: '{"peers":[],"count":0}' },
                  { method: 'POST', path: '/connect_peer', desc: 'Dial a peer by libp2p multiaddress. Body: {multiaddr}.', real: '{"status":"success"}' },
                  { method: 'POST', path: '/submit_tx', desc: 'Submit a signed transaction (see Transaction schema).', real: '{"status":"success","hash":"abc123..."}' },
                  { method: 'GET', path: '/metrics', desc: 'Prometheus-format metrics (text/plain).', real: '# HELP ... TYPE ... gauge ...' },
                ].map(({ method, path, desc, real }) => (
                  <div key={path} className="glass-strong rounded-xl p-4 hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all">
                    <div className="flex items-start gap-3">
                      <span className={`shrink-0 px-2 py-0.5 rounded text-xs font-bold mt-0.5 ${
                        method === 'GET' ? 'bg-blue-100 dark:bg-blue-500/20 text-blue-600 dark:text-blue-400' : 'bg-emerald-100 dark:bg-emerald-500/20 text-emerald-600 dark:text-emerald-400'
                      }`}>{method}</span>
                      <div className="min-w-0 flex-1">
                        <code className="text-sm text-slate-700 dark:text-slate-200 font-semibold break-all">{path}</code>
                        <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">{desc}</p>
                        {real && (
                          <div className="mt-2 p-2 rounded-lg bg-slate-800/10 dark:bg-slate-700/40">
                            <code className="text-[10px] text-slate-600 dark:text-slate-400 break-all">{real}</code>
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
              </div>

              <div className="p-5 rounded-xl glass-strong">
                <h3 className="text-sm font-semibold text-slate-800 dark:text-white mb-2">RPC URL Configuration</h3>
                <p className="text-sm text-slate-600 dark:text-slate-400">All frontends connect to the RPC endpoint. You can configure the URL in each app's settings panel. Default: <code className="text-xs text-blue-600 dark:text-blue-400">http://localhost:8545</code></p>
              </div>
            </section>
          )}

          {activeSection === 'errors' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">Error Codes</h2>
              <p className="text-slate-600 dark:text-slate-400 mb-6">The RPC API returns standardized error codes in JSON format.</p>

              <div className="overflow-x-auto rounded-2xl glass-strong">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-slate-200 dark:border-slate-700">
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Code</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Description</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Typical Cause</th>
                    </tr>
                  </thead>
                  <tbody className="text-slate-700 dark:text-slate-300">
                    {[
                      ['400', 'Bad Request', 'Invalid address format, missing fields, or malformed JSON'],
                      ['401', 'Unauthorized', 'Signature verification failed or invalid nonce'],
                      ['404', 'Not Found', 'Block height not found, account has no state, or transaction unknown'],
                      ['429', 'Too Many Requests', 'Rate limit exceeded (configurable in node config)'],
                      ['500', 'Internal Error', 'Node state error or internal runtime failure'],
                      ['503', 'Service Unavailable', 'Node still syncing or in bootstrap mode'],
                    ].map(([code, desc, cause]) => (
                      <tr key={code} className="border-b border-slate-100 dark:border-slate-700/50">
                        <td className="py-2.5 font-mono text-xs text-red-600 dark:text-red-400 font-semibold">{code}</td>
                        <td className="py-2.5 text-xs">{desc}</td>
                        <td className="py-2.5 text-xs text-slate-500 dark:text-slate-400">{cause}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              <div className="mt-6 p-4 rounded-xl bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/20">
                <p className="text-xs text-red-600 dark:text-red-300">
                  <strong>Note:</strong> Error responses follow the format: <code className="text-xs">{'{"error":"message","code":400}'}</code>
                </p>
              </div>
            </section>
          )}

          {activeSection === 'local-dev' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">Local Development</h2>
              <p className="text-slate-600 dark:text-slate-400 mb-6">Everything you need to run a full local development environment.</p>

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Service Port Map</h3>
              <div className="overflow-x-auto mb-6 rounded-2xl glass-strong">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-slate-200 dark:border-slate-700">
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Service</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Port</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Description</th>
                    </tr>
                  </thead>
                  <tbody className="text-slate-700 dark:text-slate-300">
                    {[
                      ['RPC Node', '8545', 'Primary JSON-RPC endpoint for chain interactions'],
                      ['Faucet Backend', '3001', 'Test token distribution service'],
                      ['Frontend (Unified)', '5173', 'Wallet, explorer, governance, docs, portals'],
                      ['Metrics', '9090', 'Prometheus metrics endpoint'],
                      ['P2P', '26656', 'Libp2p peer-to-peer networking'],
                    ].map(([service, port, desc]) => (
                      <tr key={service} className="border-b border-slate-100 dark:border-slate-700/50">
                        <td className="py-2 text-xs font-medium text-slate-800 dark:text-slate-200">{service}</td>
                        <td className="py-2 font-mono text-xs text-blue-600 dark:text-blue-400">{port}</td>
                        <td className="py-2 text-xs text-slate-500 dark:text-slate-400">{desc}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Test Data</h3>
              <p className="text-sm text-slate-600 dark:text-slate-400 mb-3">These addresses are pre-loaded in the genesis state with test tokens:</p>
              <CodeBlock lang="text" code={`# Test Addresses (pre-funded)
0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18  — Primary test account
0x8fD8fB8fB8fB8fD8fB8fB8fD8fB8fB8fD8fB8fD  — Validator #2
0x5B38Da6a701c568545dCfcB03FcB875f56beddC4  — Validator #3

# Faucet
curl -X POST http://localhost:3006/faucet \\
  -H "Content-Type: application/json" \\
  -d '{"address":"0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18"}'`} />

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Common Workflows</h3>
              <div className="space-y-3 mb-6">
                {[
                  ['Reset Devnet', `docker compose down -v && docker compose up -d\n# Removes all state and starts fresh`],
                  ['Check Logs', `docker compose logs -f validator-1`],
                  ['Query Balance', `curl http://localhost:8545/balance/0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18`],
                  ['Send Tokens', `# Use the wallet frontend at http://localhost:5173/wallet`],
                ].map(([title, cmd]) => (
                  <div key={title} className="p-4 rounded-xl glass-strong">
                    <h4 className="text-sm font-semibold text-slate-800 dark:text-white mb-1">{title}</h4>
                    <code className="text-xs text-slate-600 dark:text-slate-400 block">{cmd}</code>
                  </div>
                ))}
              </div>
            </section>
          )}
          </div>
          )}

          {/* ── Prev / next navigation ─────────────────────────── */}
          <div className="mt-10 grid grid-cols-2 gap-3">
            {prev ? (
              <button
                onClick={() => setActiveSection(prev.id)}
                className="text-left p-4 rounded-2xl glass-strong hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all group"
              >
                <span className="text-[11px] uppercase tracking-wider text-slate-400 flex items-center gap-1">
                  <ChevronLeft className="w-3 h-3" /> Previous
                </span>
                <span className="block mt-1.5 text-sm font-semibold text-slate-800 dark:text-white group-hover:text-blue-500 dark:group-hover:text-cyan-400 transition-colors">
                  {prev.title}
                </span>
              </button>
            ) : <div />}
            {next ? (
              <button
                onClick={() => setActiveSection(next.id)}
                className="text-right p-4 rounded-2xl glass-strong hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all group"
              >
                <span className="text-[11px] uppercase tracking-wider text-slate-400 flex items-center gap-1 justify-end">
                  Next <ChevronRight className="w-3 h-3" />
                </span>
                <span className="block mt-1.5 text-sm font-semibold text-slate-800 dark:text-white group-hover:text-blue-500 dark:group-hover:text-cyan-400 transition-colors">
                  {next.title}
                </span>
              </button>
            ) : <div />}
          </div>
        </main>
      </div>
    </div>
    </div>
  )
}
