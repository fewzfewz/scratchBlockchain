import { useEffect, useState, lazy, Suspense } from 'react'
import { Link } from 'react-router-dom'
import {
  Compass, Wallet, Droplets, Vote, BookOpen, FileCode, ArrowRight, Shield, Zap, Layers,
  Cpu, GitBranch, Image, Rocket, Package, FileJson, CheckCircle2, Timer, Network,
  Sparkles, Terminal, Activity, Copy, Check,
} from 'lucide-react'
import PageShell from '../components/PageShell.jsx'
import { getApiUrl, walletAddress, shorten } from '../lib/chain.js'

// three.js is large; load it only for the hero panel, after first paint.
const BlockchainScene = lazy(() => import('../components/BlockchainScene.jsx'))

const PRODUCTS = [
  { to: '/explorer', icon: Compass, title: 'Live Explorer', desc: 'Inspect finalized blocks, transactions, validators, and state in real time.', tag: 'Real-time' },
  { to: '/validators', icon: Shield, title: 'Staking & Validators', desc: 'Delegate NBL or onboard a validator node to earn block rewards and fees.', tag: 'Earn Yield' },
  { to: '/defi', icon: Layers, title: 'DeFi & SimpleAMM', desc: 'Deploy pools, provide liquidity, and swap tokens with sub-second finality.', tag: 'AMM DEX' },
  { to: '/bridge', icon: GitBranch, title: 'Ethereum Bridge', desc: 'Lock ETH and mint NBL via 2-of-3 independent relayer attestations.', tag: 'Cross-chain' },
  { to: '/deploy', icon: Rocket, title: 'EVM & WASM Deploy', desc: 'Ship Solidity ERC-20/721 contracts or high-speed Rust/WASM modules.', tag: 'Dual VM' },
  { to: '/governance', icon: Vote, title: 'On-Chain Governance', desc: 'Create proposals, vote with staked NBL, and execute treasury grants.', tag: 'DAO' },
]

const MORE = [
  { to: '/wallet', icon: Wallet, label: 'Wallet' },
  { to: '/nft', icon: Image, label: 'NFTs' },
  { to: '/wasm', icon: Cpu, label: 'WASM' },
  { to: '/faucet', icon: Droplets, label: 'Faucet' },
  { to: '/sdk', icon: Package, label: 'SDKs' },
  { to: '/api-docs', icon: FileJson, label: 'API Docs' },
  { to: '/docs', icon: BookOpen, label: 'Docs' },
]

const FACTS = [
  { icon: Shield, value: '>2/3', label: 'BFT stake quorum', note: 'Single-round deterministic finality' },
  { icon: Timer, value: '<1 s', label: 'Block finality', note: 'Zero forks, zero reorgs' },
  { icon: Zap, value: '~9k/s', label: 'Transfers per core', note: 'Rust revm + wasmtime execution' },
  { icon: Network, value: '2-of-3', label: 'Bridge relayer threshold', note: 'Verified in node consensus' },
]

const fmt = (v) => (v == null ? '—' : Number(v).toLocaleString())
const hex = (bytes) => (Array.isArray(bytes) ? '0x' + bytes.map((b) => b.toString(16).padStart(2, '0')).join('') : '')

function StatCard({ label, value, accent }) {
  return (
    <div className="rounded-2xl glass px-4 py-3.5 border border-slate-200/70 dark:border-slate-800/80">
      <div className="text-[11px] uppercase tracking-wider text-slate-500 dark:text-slate-400">{label}</div>
      <div className={`mt-1 font-display text-2xl font-semibold tabular-nums ${accent || 'text-slate-900 dark:text-white'}`}>
        {value}
      </div>
    </div>
  )
}

function CopySnippet({ label, cmd }) {
  const [copied, setCopied] = useState(false)
  return (
    <div className="flex items-center justify-between gap-3 rounded-xl bg-slate-900/5 dark:bg-slate-950/70 border border-slate-200 dark:border-slate-800 px-3.5 py-2.5">
      <div className="min-w-0">
        <div className="text-[10px] uppercase tracking-wider text-slate-500 font-semibold">{label}</div>
        <code className="block truncate text-xs sm:text-sm font-mono text-slate-800 dark:text-cyan-300">{cmd}</code>
      </div>
      <button
        type="button"
        onClick={() => { navigator.clipboard.writeText(cmd); setCopied(true); setTimeout(() => setCopied(false), 1800) }}
        className="shrink-0 p-1.5 rounded-lg hover:bg-slate-200/70 dark:hover:bg-slate-800 text-slate-500 dark:text-slate-400 transition-colors"
        aria-label={`Copy ${label}`}
      >
        {copied ? <Check className="w-4 h-4 text-emerald-500" /> : <Copy className="w-4 h-4" />}
      </button>
    </div>
  )
}

export default function Home() {
  const [status, setStatus] = useState(null)
  const [validators, setValidators] = useState(null)
  const [baseFeeGwei, setBaseFeeGwei] = useState(null)
  const [blocks, setBlocks] = useState([])
  const hasWallet = !!walletAddress()

  useEffect(() => {
    let active = true
    const load = async () => {
      const api = getApiUrl()
      try {
        const [sr, vr, br, gr] = await Promise.all([
          window.fetch(`${api}/status`),
          window.fetch(`${api}/validators`),
          window.fetch(`${api}/block/latest`),
          window.fetch(`${api}/gas_price`).catch(() => null),
        ])
        const [sd, vd, bd] = await Promise.all([sr.json(), vr.json(), br.json()])
        const gd = gr && gr.ok ? await gr.json().catch(() => null) : null
        if (!active) return
        setStatus({ online: true, ...sd })
        setValidators(vd?.validators?.length ?? vd?.count ?? null)
        if (gd?.base_fee != null) {
          setBaseFeeGwei((Number(gd.base_fee) / 1e9).toFixed(2))
        }
        const latest = bd?.block?.header?.slot ?? 0
        const recent = await Promise.all(
          Array.from({ length: 6 }, (_, i) => latest - i)
            .filter((s) => s >= 0)
            .map((s) => window.fetch(`${api}/block/${s}`).then((r) => r.json()).then((d) => d.block).catch(() => null)),
        )
        if (active) setBlocks(recent.filter(Boolean))
      } catch {
        if (active) setStatus({ online: false })
      }
    }
    load()
    const t = setInterval(load, 4000)
    return () => { active = false; clearInterval(t) }
  }, [])

  const online = status?.online

  return (
    <PageShell>
      {/* ── Hero ─────────────────────────────────────────────────── */}
      <section className="max-w-7xl mx-auto px-4 pt-12 lg:pt-20 pb-14 grid lg:grid-cols-[1.2fr_1fr] gap-10 items-center">
        <div>
          <div className="inline-flex items-center gap-2 h-8 px-3.5 rounded-full glass border border-slate-200 dark:border-slate-800 text-xs font-medium text-slate-700 dark:text-slate-200 mb-6">
            <span className={`w-2 h-2 rounded-full ${online ? 'bg-emerald-400 animate-pulse-dot' : status ? 'bg-rose-400' : 'bg-slate-400'}`} />
            {online
              ? <>Nebula Testnet Live <span className="text-slate-400">·</span> <span className="font-mono text-cyan-600 dark:text-cyan-400">Block #{fmt(status.height)}</span></>
              : status ? 'RPC Offline — reconnecting' : 'Connecting to Nebula RPC…'}
          </div>

          <h1 className="text-4xl sm:text-5xl xl:text-[3.75rem] font-bold tracking-tight leading-[1.06] text-slate-900 dark:text-white">
            The Modular Chain<br />
            <span className="text-aurora">Built for Speed &amp; Yield.</span>
          </h1>

          <p className="mt-5 text-base sm:text-lg text-slate-600 dark:text-slate-300 max-w-xl leading-relaxed">
            Sub-second BFT finality, dual EVM + WASM smart contracts, an on-chain AMM, and published JavaScript &amp; Python SDKs — all connected live to the Nebula network.
          </p>

          <div className="mt-7 flex flex-wrap gap-3">
            <Link to="/wallet" className="group inline-flex items-center gap-2 h-11 px-5 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-500 text-white text-sm font-semibold shadow-lg shadow-blue-600/25 hover:opacity-95 transition">
              <Wallet className="w-4 h-4" /> {hasWallet ? 'Open Wallet' : 'Create Wallet'}
              <ArrowRight className="w-4 h-4 group-hover:translate-x-0.5 transition-transform" />
            </Link>
            <Link to="/faucet" className="inline-flex items-center gap-2 h-11 px-5 rounded-xl glass text-sm font-semibold text-slate-900 dark:text-white hover:border-cyan-400/50 transition">
              <Droplets className="w-4 h-4 text-cyan-500" /> Get Free NBL
            </Link>
            <Link to="/sdk" className="inline-flex items-center gap-2 h-11 px-5 rounded-xl glass text-sm font-semibold text-slate-900 dark:text-white hover:border-blue-400/50 transition">
              <Terminal className="w-4 h-4 text-blue-500" /> Developer SDKs
            </Link>
          </div>

          <div className="mt-8 grid grid-cols-2 sm:grid-cols-4 gap-3 max-w-2xl">
            <StatCard label="Block Height" value={fmt(status?.height)} accent="text-cyan-600 dark:text-cyan-400" />
            <StatCard label="Validators" value={fmt(validators)} />
            <StatCard label="Base Fee" value={baseFeeGwei != null ? `${baseFeeGwei} gwei` : '—'} />
            <StatCard label="Mempool" value={fmt(status?.mempool_size)} />
          </div>
        </div>

        <div className="relative h-[360px] lg:h-[460px] rounded-3xl glass-strong aurora-border overflow-hidden shadow-2xl shadow-blue-500/10">
          <Suspense fallback={<div className="absolute inset-0 animate-pulse bg-slate-100 dark:bg-slate-900" />}>
            <BlockchainScene className="absolute inset-0" />
          </Suspense>

          <div className="absolute top-4 left-4 right-4 flex items-center justify-between pointer-events-none">
            <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-white/80 dark:bg-slate-950/75 backdrop-blur border border-slate-200/70 dark:border-slate-800 text-[11px] font-medium text-slate-700 dark:text-slate-200">
              <Activity className="w-3.5 h-3.5 text-emerald-500" />
              {online ? `Finalized #${fmt(status?.finalized_height ?? status?.height)}` : 'Awaiting RPC'}
            </span>
            <span className="px-3 py-1 rounded-full bg-white/80 dark:bg-slate-950/75 backdrop-blur border border-slate-200/70 dark:border-slate-800 text-[11px] font-mono text-slate-600 dark:text-slate-300">
              Chain ID: {status?.chain_id ?? 1}
            </span>
          </div>

          <div className="absolute bottom-0 inset-x-0 p-5 bg-gradient-to-t from-white/95 via-white/70 dark:from-slate-950/95 dark:via-slate-950/70 to-transparent">
            <div className="flex items-center justify-between gap-4">
              <div>
                <div className="text-xs font-semibold text-slate-900 dark:text-white">Live BFT Consensus Ring</div>
                <div className="text-xs text-slate-500 dark:text-slate-400">EVM + WASM execution · Deterministic finality</div>
              </div>
              <Link to="/explorer" className="shrink-0 inline-flex items-center gap-1 text-xs font-semibold text-blue-600 dark:text-cyan-400 hover:underline">
                Explorer <ArrowRight className="w-3.5 h-3.5" />
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* ── Live SDK Install Bar ─────────────────────────────────── */}
      <section className="max-w-7xl mx-auto px-4 pb-14">
        <div className="rounded-2xl glass-strong p-5 sm:p-6 border border-slate-200/70 dark:border-slate-800/80">
          <div className="flex flex-wrap items-center justify-between gap-3 mb-4">
            <div className="flex items-center gap-2">
              <Sparkles className="w-4 h-4 text-cyan-500" />
              <h2 className="text-sm font-semibold text-slate-900 dark:text-white">Build on Nebula in seconds with published SDKs</h2>
            </div>
            <Link to="/sdk" className="text-xs font-semibold text-blue-600 dark:text-cyan-400 hover:underline inline-flex items-center gap-1">
              Full SDK &amp; AI-Agent Guide <ArrowRight className="w-3.5 h-3.5" />
            </Link>
          </div>
          <div className="grid md:grid-cols-3 gap-3">
            <CopySnippet label="JavaScript / TypeScript (npm)" cmd="npm install nebula-chain-sdk" />
            <CopySnippet label="Python / AI Agents (PyPI)" cmd="pip install nebula-chain-sdk" />
            <CopySnippet label="Developer CLI (npm)" cmd="npx nebula-chain-cli" />
          </div>
        </div>
      </section>

      {/* ── Get started ──────────────────────────────────────────── */}
      <section className="max-w-7xl mx-auto px-4 pb-16">
        <h2 className="text-2xl font-semibold text-slate-900 dark:text-white mb-6">Start in three steps</h2>
        <ol className="grid md:grid-cols-3 gap-4">
          {[
            { n: 1, to: '/wallet', title: 'Create a wallet', desc: 'Generate an Ed25519 keypair locally in your browser.', done: hasWallet },
            { n: 2, to: '/faucet', title: 'Claim test NBL', desc: 'Get free NBL from the live faucet in under two seconds.' },
            { n: 3, to: '/defi', title: 'Stake, swap, or deploy', desc: 'Provide AMM liquidity, delegate to validators, or launch contracts.' },
          ].map(({ n, to, title, desc, done }) => (
            <li key={n}>
              <Link to={to} className="group flex gap-4 p-5 rounded-2xl glass h-full hover:-translate-y-0.5 transition-all">
                <span className={`w-9 h-9 rounded-full flex items-center justify-center shrink-0 text-sm font-semibold ${done ? 'bg-emerald-500/15 text-emerald-500' : 'bg-blue-500/10 text-blue-600 dark:text-blue-300'}`}>
                  {done ? <CheckCircle2 className="w-5 h-5" /> : n}
                </span>
                <span>
                  <span className="flex items-center gap-1.5 font-semibold text-slate-900 dark:text-white">
                    {title}<ArrowRight className="w-3.5 h-3.5 opacity-0 -translate-x-1 group-hover:opacity-100 group-hover:translate-x-0 transition" />
                  </span>
                  <span className="block mt-1 text-sm text-slate-500">{desc}</span>
                </span>
              </Link>
            </li>
          ))}
        </ol>
      </section>

      {/* ── Live blocks ──────────────────────────────────────────── */}
      <section className="max-w-7xl mx-auto px-4 pb-16">
        <div className="flex items-end justify-between mb-4">
          <div>
            <h2 className="text-2xl font-semibold text-slate-900 dark:text-white">Latest finalized blocks</h2>
            <p className="text-sm text-slate-500 mt-1">Streamed directly from {getApiUrl()}</p>
          </div>
          <Link to="/explorer" className="text-sm font-medium text-blue-600 dark:text-blue-300 hover:underline inline-flex items-center gap-1">
            Open Explorer <ArrowRight className="w-3.5 h-3.5" />
          </Link>
        </div>
        <div className="rounded-2xl glass-strong overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-[11px] uppercase tracking-wider text-slate-500 border-b border-slate-200 dark:border-slate-800">
                  <th className="px-5 py-3 font-medium">Block</th>
                  <th className="px-5 py-3 font-medium">Proposer</th>
                  <th className="px-5 py-3 font-medium text-right">Txs</th>
                  <th className="px-5 py-3 font-medium text-right">Gas used</th>
                  <th className="px-5 py-3 font-medium text-right">Signers</th>
                </tr>
              </thead>
              <tbody>
                {blocks.length === 0 && (
                  <tr><td colSpan={5} className="px-5 py-10 text-center text-slate-500">{status && !online ? 'Node offline — waiting for connection.' : 'Loading live blocks…'}</td></tr>
                )}
                {blocks.map((b) => (
                  <tr key={b.header.slot} className="border-b last:border-0 border-slate-100 dark:border-slate-800/70 hover:bg-slate-50/70 dark:hover:bg-slate-800/30 transition-colors">
                    <td className="px-5 py-3 font-mono font-medium">
                      <Link to={`/explorer?block=${b.header.slot}`} className="text-blue-600 dark:text-cyan-400 hover:underline">
                        #{fmt(b.header.slot)}
                      </Link>
                    </td>
                    <td className="px-5 py-3 font-mono text-slate-500">{shorten(hex(b.header.proposer), 8, 6) || '—'}</td>
                    <td className="px-5 py-3 text-right tabular-nums">{b.extrinsics?.length ?? 0}</td>
                    <td className="px-5 py-3 text-right tabular-nums text-slate-500">{fmt(b.header.gas_used)}</td>
                    <td className="px-5 py-3 text-right tabular-nums text-slate-500">{b.commit?.precommits?.length ?? '—'}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </section>

      {/* ── Products ─────────────────────────────────────────────── */}
      <section className="max-w-7xl mx-auto px-4 pb-16">
        <div className="flex flex-wrap items-end justify-between gap-4 mb-6">
          <div>
            <h2 className="text-2xl font-semibold text-slate-900 dark:text-white">Full-stack on-chain ecosystem</h2>
            <p className="text-sm text-slate-500 mt-1">Every module is live and wired to the Nebula RPC.</p>
          </div>
        </div>
        <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {PRODUCTS.map(({ to, icon: Icon, title, desc, tag }) => (
            <Link key={to} to={to} className="group p-6 rounded-2xl glass card-hover-3d border border-slate-200/70 dark:border-slate-800/80">
              <div className="flex items-center justify-between">
                <span className="w-10 h-10 rounded-xl bg-gradient-to-br from-blue-500/15 to-cyan-500/15 text-blue-600 dark:text-cyan-300 flex items-center justify-center">
                  <Icon className="w-5 h-5" />
                </span>
                <span className="text-[10px] font-semibold uppercase tracking-wider px-2.5 py-1 rounded-full bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-300">
                  {tag}
                </span>
              </div>
              <div className="mt-4 flex items-center gap-1.5 font-semibold text-slate-900 dark:text-white">
                {title}<ArrowRight className="w-3.5 h-3.5 text-slate-400 group-hover:translate-x-0.5 transition-transform" />
              </div>
              <p className="mt-1.5 text-sm text-slate-500 leading-relaxed">{desc}</p>
            </Link>
          ))}
        </div>
        <div className="mt-5 flex flex-wrap gap-2">
          {MORE.map(({ to, icon: Icon, label }) => (
            <Link key={to} to={to} className="inline-flex items-center gap-2 h-9 px-3.5 rounded-lg border border-slate-200 dark:border-slate-800 text-sm text-slate-600 dark:text-slate-300 hover:border-slate-300 dark:hover:border-slate-700 transition-colors">
              <Icon className="w-4 h-4 text-slate-400" /> {label}
            </Link>
          ))}
        </div>
      </section>

      {/* ── Under the hood ───────────────────────────────────────── */}
      <section className="max-w-7xl mx-auto px-4 pb-8">
        <div className="rounded-3xl glass-strong p-8 lg:p-10">
          <div className="flex flex-wrap items-end justify-between gap-4 mb-8">
            <div>
              <h2 className="text-2xl font-semibold text-slate-900 dark:text-white">Under the hood</h2>
              <p className="text-sm text-slate-500 mt-1">Modular Rust architecture built for deterministic finality and throughput.</p>
            </div>
            <Link to="/docs" className="text-sm font-medium text-blue-600 dark:text-blue-300 hover:underline inline-flex items-center gap-1">
              Architecture Docs <ArrowRight className="w-3.5 h-3.5" />
            </Link>
          </div>
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-8">
            {FACTS.map(({ icon: Icon, value, label, note }) => (
              <div key={label}>
                <Icon className="w-5 h-5 text-cyan-500" />
                <div className="mt-3 font-display text-3xl font-semibold text-slate-900 dark:text-white">{value}</div>
                <div className="mt-1 text-sm text-slate-600 dark:text-slate-300">{label}</div>
                <div className="mt-0.5 text-xs text-slate-400">{note}</div>
              </div>
            ))}
          </div>
          <div className="mt-10 pt-6 border-t border-slate-200 dark:border-slate-800 flex flex-wrap gap-x-8 gap-y-2 text-xs text-slate-500">
            <span className="inline-flex items-center gap-1.5"><FileCode className="w-3.5 h-3.5" /> Rust · libp2p · revm · wasmtime</span>
            <span>LtHash + Patricia state root</span>
            <span>Stake-weighted proposers</span>
            <span>Participation-based rewards</span>
          </div>
        </div>
      </section>
    </PageShell>
  )
}
