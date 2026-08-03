import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import {
  Compass, Wallet, Droplets, Vote, BookOpen, FileCode, ExternalLink,
  ArrowRight, ArrowDown, Shield, Zap, Layers, Cpu, Activity, Layers2,
  Github, Blocks, Server, Database, Workflow, Terminal, GitFork, Boxes,
  History, GitBranch, Image, Rocket,
} from 'lucide-react'
import PageShell from '../components/PageShell.jsx'
import AnimatedSection, { StaggerGrid } from '../components/AnimatedSection.jsx'

const API_URL = () => localStorage.getItem('nebula_rpc_url') || 'http://localhost:8545'

const APPS = [
  { to: '/explorer', icon: Compass, title: 'Explorer', desc: 'Blocks, address lookup, validators & staking', color: 'from-blue-500/20 to-cyan-600/10 border-blue-500/20', chip: 'from-blue-500 to-cyan-600' },
  { to: '/wallet', icon: Wallet, title: 'Wallet', desc: 'Ed25519 keys, balance, send & tx history', color: 'from-emerald-500/20 to-emerald-600/10 border-emerald-500/20', chip: 'from-emerald-500 to-teal-600' },
  { to: '/history', icon: History, title: 'Tx History', desc: 'Etherscan-style address search', color: 'from-violet-500/20 to-fuchsia-600/10 border-violet-500/20', chip: 'from-violet-500 to-fuchsia-600' },
  { to: '/deploy', icon: Rocket, title: 'Deploy', desc: 'ERC20 & ERC721 smart contracts', color: 'from-orange-500/20 to-orange-600/10 border-orange-500/20', chip: 'from-orange-500 to-amber-600' },
  { to: '/contracts', icon: FileCode, title: 'Contracts', desc: 'Read & write deployed contracts', color: 'from-indigo-500/20 to-indigo-600/10 border-indigo-500/20', chip: 'from-indigo-500 to-blue-600' },
  { to: '/defi', icon: Layers, title: 'DeFi', desc: 'On-chain swaps & liquidity', color: 'from-cyan-500/20 to-cyan-600/10 border-cyan-500/20', chip: 'from-cyan-500 to-blue-600' },
  { to: '/bridge', icon: GitBranch, title: 'Bridge', desc: 'Ethereum ↔ Nebula cross-chain', color: 'from-emerald-500/20 to-teal-600/10 border-emerald-500/20', chip: 'from-emerald-500 to-teal-600' },
  { to: '/nft', icon: Image, title: 'NFT', desc: 'Mint & browse ERC721', color: 'from-pink-500/20 to-rose-600/10 border-pink-500/20', chip: 'from-pink-500 to-rose-600' },
  { to: '/faucet', icon: Droplets, title: 'Faucet', desc: 'Test tokens for dev', color: 'from-amber-500/20 to-amber-600/10 border-amber-500/20', chip: 'from-amber-500 to-orange-600' },
  { to: '/governance', icon: Vote, title: 'Governance', desc: 'Proposals & treasury', color: 'from-violet-500/20 to-violet-600/10 border-violet-500/20', chip: 'from-violet-500 to-purple-600' },
  { to: '/docs', icon: BookOpen, title: 'Docs', desc: 'Architecture & guides', color: 'from-rose-500/20 to-rose-600/10 border-rose-500/20', chip: 'from-rose-500 to-pink-600' },
  { to: '/api-docs', icon: FileCode, title: 'API Docs', desc: '32-route Swagger playground', color: 'from-indigo-500/20 to-indigo-600/10 border-indigo-500/20', chip: 'from-indigo-500 to-blue-600' },
  { to: '/sdk', icon: Layers, title: 'SDK Portal', desc: 'JavaScript SDK & CLI', color: 'from-teal-500/20 to-teal-600/10 border-teal-500/20', chip: 'from-teal-500 to-emerald-600' },
  { to: '/developer-portal', icon: ExternalLink, title: 'Dev Portal', desc: 'Starter kits & resources', color: 'from-fuchsia-500/20 to-fuchsia-600/10 border-fuchsia-500/20', chip: 'from-fuchsia-500 to-pink-600' },
]

const PILLARS = [
  { icon: Layers2, title: 'Modular', desc: 'Consensus, execution, networking and storage as composable layers.' },
  { icon: Shield, title: 'Secure', desc: 'BFT consensus, verified blocks, rate limiting and key isolation.' },
  { icon: Zap, title: 'Fast', desc: 'EIP-1559 pricing, prioritized mempool, parallel block execution.' },
  { icon: Cpu, title: 'Developer-first', desc: 'Docker testnet, SDKs, starter kits and full RPC surface.' },
]

const STACK = [
  { icon: Layers2, layer: 'Frontend', detail: '16-route SPA · 3D dashboard · Wallet · DeFi · Bridge' },
  { icon: Server, layer: 'Node RPC', detail: '34 HTTP routes · bridge mint · call_contract' },
  { icon: GitFork, layer: 'Consensus', detail: 'BFT · 2/3 quorum · validator hot-reload' },
  { icon: Workflow, layer: 'Execution', detail: 'EVM · WASM · EIP-1559 · persistent state' },
  { icon: Boxes, layer: 'Networking', detail: 'libp2p · gossipsub · Kademlia DHT' },
  { icon: Database, layer: 'Storage', detail: 'RocksDB · Patricia trie · pruning' },
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
          window.fetch(`${API_URL()}/status`),
          window.fetch(`${API_URL()}/validators`),
        ])
        if (!sr.ok) throw new Error('node offline')
        const [sd, vd] = await Promise.all([sr.json(), vr.ok ? vr.json() : null])
        if (!active) return
        setStatus(sd)
        setValidatorCount(vd?.validators?.length ?? vd?.count ?? 3)
        setOnline(true)
      } catch { if (active) setOnline(false) }
    }
    const fetchBlocks = async () => {
      try {
        const res = await window.fetch(`${API_URL()}/block/latest`)
        if (!res.ok) throw new Error('offline')
        const { block } = await res.json()
        const latest = block.header.slot
        const reqs = []
        for (let s = latest; s >= Math.max(latest - 5, 0); s--) {
          reqs.push(window.fetch(`${API_URL()}/block/${s}`).then((r) => r.json()).then((d) => d.block).catch(() => null))
        }
        if (active) setRecentBlocks((await Promise.all(reqs)).filter(Boolean))
      } catch { /* keep */ }
    }
    fetchAll(); fetchBlocks()
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
    <PageShell variant="hero">
      <div className="max-w-7xl mx-auto px-4 pt-16 pb-20">
        {/* Hero — text left, 3D shows through PageShell on all viewports */}
        <div className="grid lg:grid-cols-2 gap-12 items-center mb-20 min-h-[420px]">
          <AnimatedSection className="text-center lg:text-left">
            <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full glass-strong text-xs font-medium text-slate-600 dark:text-slate-300 mb-6 animate-scale-in">
              <span className={`w-2 h-2 rounded-full ${online ? 'bg-emerald-400 animate-pulse-dot' : 'bg-red-400'}`} />
              {online ? 'Testnet live' : 'Node offline'}
              <span className="text-slate-400">· BFT · EVM · Bridge</span>
            </div>

            <h1 className="text-5xl md:text-6xl xl:text-7xl font-bold tracking-tight text-slate-900 dark:text-white mb-6 hero-text-glow">
              Modular blockchain
              <span className="block mt-2 bg-gradient-to-r from-cyan-400 via-blue-500 to-violet-500 bg-clip-text text-transparent animate-shimmer bg-[length:200%_100%]">
                in 3D motion.
              </span>
            </h1>

            <p className="text-slate-500 dark:text-slate-400 text-lg max-w-xl mx-auto lg:mx-0 mb-8">
              Nebula — full-stack testnet with wallet, DeFi, NFT, Ethereum bridge, and a live 3D dashboard.
            </p>

            <div className="flex flex-wrap items-center justify-center lg:justify-start gap-3 mb-10">
              <Link to="/explorer" className="group inline-flex items-center gap-2 px-6 py-3 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 text-white text-sm font-semibold shadow-lg shadow-cyan-600/30 hover:shadow-cyan-500/50 hover:-translate-y-1 transition-all animate-glow-pulse">
                <Compass className="w-4 h-4" /> Launch Explorer <ArrowRight className="w-4 h-4 group-hover:translate-x-1 transition-transform" />
              </Link>
              <Link to="/bridge" className="inline-flex items-center gap-2 px-6 py-3 rounded-xl glass-strong text-sm font-semibold hover:-translate-y-1 transition-all">
                <GitBranch className="w-4 h-4" /> Cross-Chain Bridge
              </Link>
            </div>

            <StaggerGrid className="grid grid-cols-2 gap-3 max-w-md mx-auto lg:mx-0">
              {stats.map(({ label, value, icon: Icon }) => (
                <div key={label} className="p-4 rounded-2xl glass-strong card-hover-3d">
                  <div className="flex items-center gap-1.5 mb-1">
                    <Icon className="w-3.5 h-3.5 text-cyan-500" />
                    <span className="text-[10px] uppercase tracking-wider text-slate-500">{label}</span>
                  </div>
                  <div className="text-xl font-bold tabular-nums">{value}</div>
                </div>
              ))}
            </StaggerGrid>
          </AnimatedSection>

          <div className="hidden lg:block h-[380px] rounded-3xl glass-strong border border-cyan-500/20 animate-scale-in overflow-hidden relative">
            <div className="absolute inset-0 flex items-center justify-center">
              <p className="text-xs text-slate-400/80 text-center px-6">
                Interactive 3D scene — block core, validator ring & chain satellites
              </p>
            </div>
          </div>
        </div>

        {/* Live blocks */}
        <AnimatedSection delay={100} className="mb-16">
          <div className="flex items-end justify-between mb-5">
            <div>
              <h2 className="text-2xl font-bold">Live chain activity</h2>
              <p className="text-sm text-slate-500 mt-1">Blocks finalized in real time</p>
            </div>
            <Link to="/explorer" className="text-sm text-cyan-500 font-medium hover:underline flex items-center gap-1">View all <ArrowRight className="w-3.5 h-3.5" /></Link>
          </div>
          {recentBlocks.length > 0 ? (
            <StaggerGrid className="grid sm:grid-cols-2 lg:grid-cols-3 gap-3">
              {recentBlocks.map((b) => (
                <div key={b.header.slot} className="p-4 rounded-2xl glass-strong card-hover-3d">
                  <div className="flex justify-between mb-2">
                    <span className="font-mono font-bold">#{fmt(b.header.slot)}</span>
                    <span className="text-xs text-slate-500 flex items-center gap-1"><Blocks className="w-3.5 h-3.5" />{b.extrinsics?.length ?? 0} tx</span>
                  </div>
                  <div className="grid grid-cols-2 gap-2 text-xs">
                    <div className="rounded-lg bg-slate-100/60 dark:bg-slate-800/40 p-2"><span className="text-slate-400">Base fee</span><div className="font-semibold tabular-nums">{fmt(b.header.base_fee)}</div></div>
                    <div className="rounded-lg bg-slate-100/60 dark:bg-slate-800/40 p-2"><span className="text-slate-400">Gas used</span><div className="font-semibold tabular-nums">{fmt(b.header.gas_used)}</div></div>
                  </div>
                </div>
              ))}
            </StaggerGrid>
          ) : (
            <div className="p-8 rounded-2xl glass text-center text-slate-500">{online ? 'Fetching blocks…' : 'Start the node to see live data.'}</div>
          )}
        </AnimatedSection>

        {/* Apps */}
        <AnimatedSection delay={150} className="mb-16">
          <h2 className="text-2xl font-bold mb-5">Explore the network</h2>
          <StaggerGrid className="grid sm:grid-cols-2 lg:grid-cols-4 gap-4">
            {APPS.map(({ to, icon: Icon, title, desc, color, chip }) => (
              <Link key={to} to={to} className={`group p-5 rounded-2xl bg-gradient-to-br ${color} border card-hover-3d backdrop-blur-sm`}>
                <div className={`w-11 h-11 rounded-xl bg-gradient-to-br ${chip} flex items-center justify-center text-white shadow-lg mb-3 group-hover:scale-110 transition-transform`}>
                  <Icon className="w-5 h-5" />
                </div>
                <h3 className="font-semibold mb-1">{title}</h3>
                <p className="text-sm text-slate-500 leading-relaxed">{desc}</p>
              </Link>
            ))}
          </StaggerGrid>
        </AnimatedSection>

        {/* Pillars + stack */}
        <AnimatedSection delay={200} className="mb-16">
          <div className="grid lg:grid-cols-2 gap-8">
            <div>
              <h2 className="text-2xl font-bold mb-4">Built for developers</h2>
              <div className="grid sm:grid-cols-2 gap-3">
                {PILLARS.map(({ icon: Icon, title, desc }) => (
                  <div key={title} className="p-4 rounded-2xl glass-strong card-hover-3d">
                    <Icon className="w-5 h-5 text-cyan-500 mb-2" />
                    <h3 className="font-semibold text-sm mb-1">{title}</h3>
                    <p className="text-xs text-slate-500">{desc}</p>
                  </div>
                ))}
              </div>
            </div>
            <div>
              <h2 className="text-2xl font-bold mb-4">Modular stack</h2>
              {STACK.map(({ icon: Icon, layer, detail }, i) => (
                <div key={layer}>
                  <div className="flex items-center gap-3 p-3 rounded-xl glass-strong card-hover-3d mb-1">
                    <div className="w-9 h-9 rounded-lg bg-gradient-to-br from-blue-500 to-cyan-600 flex items-center justify-center text-white"><Icon className="w-4 h-4" /></div>
                    <div><div className="text-sm font-semibold">{layer}</div><div className="text-xs text-slate-500">{detail}</div></div>
                  </div>
                  {i < STACK.length - 1 && <div className="flex justify-center py-0.5"><ArrowDown className="w-3 h-3 text-slate-400" /></div>}
                </div>
              ))}
            </div>
          </div>
        </AnimatedSection>

        {/* CTA */}
        <AnimatedSection delay={250}>
          <div className="rounded-3xl p-10 text-center glass-strong relative overflow-hidden animate-glow-pulse">
            <h2 className="text-2xl font-bold mb-3">Ready to build?</h2>
            <p className="text-slate-500 max-w-lg mx-auto mb-6">Wallet, faucet, bridge, and 3D explorer — all connected to your local node.</p>
            <div className="flex flex-wrap justify-center gap-3">
              <Link to="/wallet" className="px-6 py-3 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 text-white font-semibold hover:-translate-y-1 transition-all"><Wallet className="w-4 h-4 inline mr-2" />Create Wallet</Link>
              <Link to="/faucet" className="px-6 py-3 rounded-xl glass font-semibold hover:-translate-y-1 transition-all"><Droplets className="w-4 h-4 inline mr-2" />Get Tokens</Link>
              <a href="https://github.com/fewzfewz/scratchBlockchain" target="_blank" rel="noreferrer" className="px-6 py-3 rounded-xl glass font-semibold hover:-translate-y-1 transition-all"><Github className="w-4 h-4 inline mr-2" />GitHub</a>
            </div>
          </div>
        </AnimatedSection>
      </div>
    </PageShell>
  )
}
