import { useEffect, useState, lazy, Suspense } from 'react'
import { Link } from 'react-router-dom'
import {
  Compass, Wallet, Droplets, Vote, BookOpen, FileCode, ArrowRight, Shield, Zap, Layers,
  Cpu, GitBranch, Image, Rocket, Package, FileJson, CheckCircle2, Timer, Network,
} from 'lucide-react'
import PageShell from '../components/PageShell.jsx'
import { getApiUrl, walletAddress, shorten } from '../lib/chain.js'

// three.js is large; load it only for the hero panel, after first paint.
const BlockchainScene = lazy(() => import('../components/BlockchainScene.jsx'))

const PRODUCTS = [
  { to: '/explorer', icon: Compass, title: 'Explorer', desc: 'Blocks, addresses, validators and staking, live.' },
  { to: '/wallet', icon: Wallet, title: 'Wallet', desc: 'Ed25519 keys in your browser. Send NBL in a click.' },
  { to: '/deploy', icon: Rocket, title: 'Deploy', desc: 'Ship a real ERC20 or ERC721 to the EVM.' },
  { to: '/defi', icon: Layers, title: 'DeFi', desc: 'Swap and provide liquidity against an on-chain pool.' },
  { to: '/bridge', icon: GitBranch, title: 'Bridge', desc: 'Lock ETH, receive NBL — minted by independent relayers.' },
  { to: '/governance', icon: Vote, title: 'Governance', desc: 'Propose, vote with your stake, execute on chain.' },
]

const MORE = [
  { to: '/nft', icon: Image, label: 'NFT' },
  { to: '/wasm', icon: Cpu, label: 'WASM' },
  { to: '/faucet', icon: Droplets, label: 'Faucet' },
  { to: '/docs', icon: BookOpen, label: 'Docs' },
  { to: '/api-docs', icon: FileJson, label: 'API' },
  { to: '/sdk', icon: Package, label: 'SDK' },
]

// Measured on the local devnet (see benchmarks/README.md).
const FACTS = [
  { icon: Shield, value: '>2/3', label: 'of stake finalizes a block', note: 'BFT, one round, no reorgs' },
  { icon: Timer, value: '<1 s', label: 'to finality at light load', note: 'local 4-validator devnet' },
  { icon: Zap, value: '~9k/s', label: 'transfers executed per core', note: 'block execution benchmark' },
  { icon: Network, value: '2-of-3', label: 'relayers sign every bridge mint', note: 'checked in consensus' },
]

const fmt = (v) => (v == null ? '—' : Number(v).toLocaleString())
const hex = (bytes) => (Array.isArray(bytes) ? '0x' + bytes.map((b) => b.toString(16).padStart(2, '0')).join('') : '')

function Stat({ label, value }) {
  return (
    <div>
      <div className="text-[11px] uppercase tracking-wider text-slate-500">{label}</div>
      <div className="mt-1 font-display text-2xl font-semibold tabular-nums text-slate-900 dark:text-white">{value}</div>
    </div>
  )
}

export default function Home() {
  const [status, setStatus] = useState(null)
  const [validators, setValidators] = useState(null)
  const [blocks, setBlocks] = useState([])
  const hasWallet = !!walletAddress()

  useEffect(() => {
    let active = true
    const load = async () => {
      const api = getApiUrl()
      try {
        const [sr, vr, br] = await Promise.all([
          window.fetch(`${api}/status`),
          window.fetch(`${api}/validators`),
          window.fetch(`${api}/block/latest`),
        ])
        const [sd, vd, bd] = await Promise.all([sr.json(), vr.json(), br.json()])
        if (!active) return
        setStatus({ online: true, ...sd })
        setValidators(vd?.validators?.length ?? vd?.count ?? null)
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
      <section className="max-w-7xl mx-auto px-4 pt-16 lg:pt-24 pb-16 grid lg:grid-cols-[1.25fr_1fr] gap-12 items-center">
        <div>
          <div className="inline-flex items-center gap-2 h-7 px-3 rounded-full border border-slate-200 dark:border-slate-800 text-xs text-slate-600 dark:text-slate-300 mb-6">
            <span className={`w-1.5 h-1.5 rounded-full ${online ? 'bg-emerald-400 animate-pulse-dot' : status ? 'bg-rose-400' : 'bg-slate-400'}`} />
            {online ? <>Devnet live <span className="text-slate-400">·</span> <span className="font-mono">#{fmt(status.height)}</span></> : status ? 'Node offline — start the devnet' : 'Connecting…'}
          </div>
          <h1 className="text-4xl sm:text-5xl xl:text-[3.6rem] font-semibold leading-[1.08] text-slate-900 dark:text-white">
            A modular blockchain<br />
            <span className="text-aurora">on your laptop.</span>
          </h1>
          <p className="mt-6 text-lg text-slate-500 dark:text-slate-400 max-w-xl leading-relaxed">
            Sub-second BFT finality, EVM and WASM contracts, and an Ethereum bridge secured by independent relayers.
            Every page here talks to your local node.
          </p>
          <div className="mt-8 flex flex-wrap gap-3">
            <Link to="/wallet" className="group inline-flex items-center gap-2 h-11 px-5 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 text-white text-sm font-semibold shadow-lg shadow-blue-600/20 hover:opacity-95 transition">
              <Wallet className="w-4 h-4" /> {hasWallet ? 'Open your wallet' : 'Create a wallet'}
              <ArrowRight className="w-4 h-4 group-hover:translate-x-0.5 transition-transform" />
            </Link>
            <Link to="/explorer" className="inline-flex items-center gap-2 h-11 px-5 rounded-xl glass text-sm font-semibold text-slate-900 dark:text-white">
              <Compass className="w-4 h-4" /> Explore the chain
            </Link>
          </div>
          <div className="mt-12 grid grid-cols-2 sm:grid-cols-4 gap-6 max-w-xl">
            <Stat label="Height" value={fmt(status?.height)} />
            <Stat label="Validators" value={fmt(validators)} />
            <Stat label="Peers" value={fmt(status?.peer_count)} />
            <Stat label="Mempool" value={fmt(status?.mempool_size)} />
          </div>
        </div>

        <div className="hidden lg:block relative h-[440px] rounded-3xl glass-strong aurora-border overflow-hidden">
          <Suspense fallback={<div className="absolute inset-0 animate-pulse bg-slate-100 dark:bg-slate-900" />}>
            <BlockchainScene className="absolute inset-0" />
          </Suspense>
          <div className="absolute bottom-0 inset-x-0 p-5 bg-gradient-to-t from-white/90 dark:from-slate-950/90 to-transparent">
            <div className="text-xs text-slate-500">Block core · validator ring · chain satellites</div>
          </div>
        </div>
      </section>

      {/* ── Get started ──────────────────────────────────────────── */}
      <section className="max-w-7xl mx-auto px-4 pb-16">
        <h2 className="text-2xl font-semibold text-slate-900 dark:text-white mb-6">Get started in three steps</h2>
        <ol className="grid md:grid-cols-3 gap-4">
          {[
            { n: 1, to: '/wallet', title: 'Create a wallet', desc: 'Keys are generated and kept in this browser.', done: hasWallet },
            { n: 2, to: '/faucet', title: 'Get test NBL', desc: '10 NBL from the faucet, credited in about a second.' },
            { n: 3, to: '/deploy', title: 'Build something', desc: 'Send a transfer, deploy a token, or bridge ETH in.' },
          ].map(({ n, to, title, desc, done }) => (
            <li key={n}>
              <Link to={to} className="group flex gap-4 p-5 rounded-2xl glass h-full">
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
            <h2 className="text-2xl font-semibold text-slate-900 dark:text-white">Latest blocks</h2>
            <p className="text-sm text-slate-500 mt-1">Each one final the moment it appears.</p>
          </div>
          <Link to="/explorer" className="text-sm font-medium text-blue-600 dark:text-blue-300 hover:underline inline-flex items-center gap-1">
            Explorer <ArrowRight className="w-3.5 h-3.5" />
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
                  <tr><td colSpan={5} className="px-5 py-10 text-center text-slate-500">{status && !online ? 'Start the devnet to see live blocks.' : 'Loading blocks…'}</td></tr>
                )}
                {blocks.map((b) => (
                  <tr key={b.header.slot} className="border-b last:border-0 border-slate-100 dark:border-slate-800/70">
                    <td className="px-5 py-3 font-mono font-medium text-slate-900 dark:text-white">#{fmt(b.header.slot)}</td>
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
        <h2 className="text-2xl font-semibold text-slate-900 dark:text-white mb-6">Everything runs on your node</h2>
        <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {PRODUCTS.map(({ to, icon: Icon, title, desc }) => (
            <Link key={to} to={to} className="group p-6 rounded-2xl glass card-hover-3d">
              <span className="w-10 h-10 rounded-xl bg-gradient-to-br from-blue-500/15 to-cyan-500/15 text-blue-600 dark:text-blue-300 flex items-center justify-center">
                <Icon className="w-5 h-5" />
              </span>
              <div className="mt-4 flex items-center gap-1.5 font-semibold text-slate-900 dark:text-white">
                {title}<ArrowRight className="w-3.5 h-3.5 text-slate-400 group-hover:translate-x-0.5 transition-transform" />
              </div>
              <p className="mt-1.5 text-sm text-slate-500 leading-relaxed">{desc}</p>
            </Link>
          ))}
        </div>
        <div className="mt-4 flex flex-wrap gap-2">
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
              <p className="text-sm text-slate-500 mt-1">Measured on the local devnet, not projected.</p>
            </div>
            <Link to="/docs" className="text-sm font-medium text-blue-600 dark:text-blue-300 hover:underline inline-flex items-center gap-1">
              Architecture <ArrowRight className="w-3.5 h-3.5" />
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
