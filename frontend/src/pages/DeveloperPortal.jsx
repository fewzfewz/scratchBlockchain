import { useState, useEffect } from 'react'
import {
  Code2, Terminal, Github, Activity, Gauge, ShieldCheck, WifiOff,
  Copy, Check, Blocks, Coins, Rocket, Wand2, ScrollText, PlusCircle,
  Wallet, Layers, ArrowRight, Package, FileCode, Boxes,
} from 'lucide-react'
import { Link } from 'react-router-dom'
import { getApiUrl } from '../lib/chain.js'
import { PageHeader, LiveBadge } from '../components/ui.jsx'

const fmt = (v) => v == null || isNaN(Number(v)) ? '--' : Number(v).toLocaleString()

function CodeBlock({ code, lang = 'bash' }) {
  const [copied, setCopied] = useState(false)
  return (
    <div className="rounded-xl border border-slate-200 dark:border-slate-700/50 bg-slate-50 dark:bg-slate-800/80 overflow-hidden mb-4 hover:border-slate-300 dark:hover:border-slate-600/60 transition-colors">
      <div className="flex items-center justify-between px-4 py-1.5 bg-slate-100 dark:bg-black/30 border-b border-slate-200 dark:border-slate-700/50">
        <span className="flex items-center gap-2">
          <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
          <span className="text-[10px] uppercase tracking-wider font-semibold px-1.5 py-0.5 rounded bg-emerald-500/15 text-emerald-600 dark:text-emerald-400">{lang}</span>
        </span>
        <button
          onClick={() => { navigator.clipboard.writeText(code); setCopied(true); setTimeout(() => setCopied(false), 2000) }}
          className="text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 transition-colors"
        >
          {copied ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
        </button>
      </div>
      <pre className="p-4 overflow-x-auto text-sm text-slate-700 dark:text-slate-300 font-mono leading-relaxed scrollbar-thin"><code>{code}</code></pre>
    </div>
  )
}

function Skeleton() {
  return (
    <div className="space-y-6 animate-fade-in">
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
        {[0, 1, 2, 3].map(i => <div key={i} className="h-24 rounded-2xl glass-strong animate-pulse" />)}
      </div>
      {[0, 1, 2].map(i => <div key={i} className="h-64 rounded-2xl glass-strong animate-pulse" />)}
    </div>
  )
}

export default function DeveloperPortal() {
  const [loading, setLoading] = useState(true)
  const [backendOnline, setBackendOnline] = useState(true)
  const [network, setNetwork] = useState(null)

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

  useEffect(() => {
    const t = setTimeout(() => setLoading(false), 350)
    return () => clearTimeout(t)
  }, [])

  return (
    <div className="relative min-h-screen overflow-hidden">

      <div className="relative z-10 py-12 max-w-5xl mx-auto px-4 animate-fade-in">
        {/* ── Header ─────────────────────────────────────────────── */}
        <PageHeader icon={Blocks} eyebrow="Developers" title="Build on Nebula"
          description="Starter kits, tooling and examples for shipping against your local node."
          actions={<LiveBadge online={backendOnline} label={network ? `#${Number(network.height).toLocaleString()}` : 'Online'} />} />

        {!backendOnline && (
          <div className="mb-8 px-4 py-3 rounded-xl bg-amber-500/15 border border-amber-500/30 text-sm text-amber-400 flex items-center gap-2">
            <WifiOff className="w-4 h-4 shrink-0" />
            <span>Node not reachable at {getApiUrl()}. Starter kits and SDK samples still work — start the testnet to deploy against it.</span>
          </div>
        )}

        {loading ? <Skeleton /> : (
          <div className="space-y-10">
            {/* ── Quick stats ────────────────────────────────────── */}
            <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
              {[
                { icon: Package, label: 'SDK packages', value: '3', sub: 'Runtime, CLI, templates', chip: 'from-blue-600 to-cyan-600' },
                { icon: Boxes, label: 'Starter kits', value: '4', sub: 'Token, DeFi, NFT, DAO', chip: 'from-emerald-600 to-teal-600' },
                { icon: Terminal, label: 'CLI commands', value: '12+', sub: 'Init, deploy, wallet', chip: 'from-amber-500 to-orange-600' },
                { icon: Layers, label: 'RPC endpoints', value: '17', sub: 'REST + JSON-RPC', chip: 'from-violet-600 to-purple-600' },
              ].map(({ icon: Icon, label, value, sub, chip }) => (
                <div key={label} className="p-4 rounded-2xl glass-strong hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all">
                  <div className="flex items-center gap-2 mb-3">
                    <div className={`w-8 h-8 rounded-lg bg-gradient-to-br ${chip} flex items-center justify-center text-white shadow-md`}>
                      <Icon className="w-4 h-4" />
                    </div>
                    <span className="text-[11px] uppercase tracking-wider text-slate-500 dark:text-slate-400">{label}</span>
                  </div>
                  <div className="text-2xl font-bold text-slate-900 dark:text-white tabular-nums">{value}</div>
                  <p className="text-xs text-slate-400 dark:text-slate-500 mt-1">{sub}</p>
                </div>
              ))}
            </div>

            {/* ── Hero buttons ───────────────────────────────────── */}
            <div className="flex flex-wrap justify-center gap-3">
              {[
                { to: '/docs', label: 'Get Started', icon: ScrollText },
                { to: '/sdk', label: 'JavaScript SDK', icon: Code2 },
                { to: '/api-docs', label: 'RPC Reference', icon: Terminal },
              ].map(({ to, label, icon: Icon }) => (
                <Link
                  key={to}
                  to={to}
                  className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl glass text-sm font-semibold text-slate-700 dark:text-slate-200 hover:-translate-y-0.5 transition-all"
                >
                  <Icon className="w-4 h-4 text-blue-500 dark:text-cyan-400" />
                  {label}
                  <ArrowRight className="w-3.5 h-3.5 text-slate-400" />
                </Link>
              ))}
            </div>

            {/* ── JavaScript SDK ─────────────────────────────────── */}
            <section>
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-2">JavaScript SDK</h2>
              <p className="text-slate-500 dark:text-slate-400 mb-6">Connect, transact, and build with the Nebula SDK.</p>
              <div className="space-y-4 mb-6">
                <CodeBlock lang="bash" code={'npm install @modular-blockchain/sdk'} />
                <CodeBlock lang="ts" code={`import { ModularClient, HttpProvider, Wallet } from '@modular-blockchain/sdk';

const client = new ModularClient(
  new HttpProvider('http://localhost:8545')
);

// Generate a wallet
const wallet = Wallet.generate();

// Check balance
const balance = await client.getBalance(wallet.address);

// Send a transaction
const tx = await wallet.signTransaction({
  to: '0x...', value: '1000000000000000000',
});
const result = await client.sendTransaction(tx);`} />
              </div>
              <div className="grid md:grid-cols-3 gap-4">
                {[
                  { icon: Wallet, title: 'Wallet', desc: 'Generate, import, and sign transactions with Ed25519 keys.', chip: 'from-blue-600 to-cyan-600' },
                  { icon: Terminal, title: 'Client', desc: 'Full RPC coverage: blocks, transactions, and accounts.', chip: 'from-emerald-600 to-teal-600' },
                  { icon: FileCode, title: 'Governance', desc: 'Proposals, voting, and treasury via SDK methods.', chip: 'from-violet-600 to-purple-600' },
                ].map(({ icon: Icon, title, desc, chip }) => (
                  <div key={title} className="p-5 rounded-2xl glass-strong hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all">
                    <div className={`w-10 h-10 rounded-xl bg-gradient-to-br ${chip} flex items-center justify-center text-white shadow-md mb-3`}>
                      <Icon className="w-5 h-5" />
                    </div>
                    <h3 className="text-slate-800 dark:text-white font-semibold mb-1">{title}</h3>
                    <p className="text-sm text-slate-500 dark:text-slate-400">{desc}</p>
                  </div>
                ))}
              </div>
            </section>

            {/* ── Starter Kits ───────────────────────────────────── */}
            <section>
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-2">Starter Kits</h2>
              <p className="text-slate-500 dark:text-slate-400 mb-6">Ready-to-deploy templates for common dApp patterns.</p>
              <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-4">
                {[
                  { icon: Coins, title: 'Token', desc: 'ERC-20 and ERC-721 token deployment.', chip: 'from-amber-500 to-orange-600' },
                  { icon: Layers, title: 'DeFi', desc: 'AMM DEX with liquidity pools.', chip: 'from-blue-600 to-cyan-600' },
                  { icon: Rocket, title: 'NFT Marketplace', desc: 'Mint, list, buy, and sell NFTs.', chip: 'from-violet-600 to-purple-600' },
                  { icon: Wand2, title: 'DAO', desc: 'On-chain governance with voting.', chip: 'from-emerald-600 to-teal-600' },
                ].map(({ icon: Icon, title, desc, chip }) => (
                  <Link key={title} to="/sdk" className="p-5 rounded-2xl glass-strong hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all group">
                    <div className={`w-10 h-10 rounded-xl bg-gradient-to-br ${chip} flex items-center justify-center text-white shadow-md mb-3`}>
                      <Icon className="w-5 h-5" />
                    </div>
                    <h3 className="text-slate-800 dark:text-white font-semibold mb-1">{title}</h3>
                    <p className="text-sm text-slate-500 dark:text-slate-400 mb-2">{desc}</p>
                    <span className="text-xs text-blue-500 dark:text-cyan-400 group-hover:text-blue-400 dark:group-hover:text-cyan-300 inline-flex items-center gap-1">
                      View Kit <ArrowRight className="w-3 h-3 group-hover:translate-x-0.5 transition-transform" />
                    </span>
                  </Link>
                ))}
              </div>
            </section>

            {/* ── CLI Tool ───────────────────────────────────────── */}
            <section>
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-2">CLI Tool</h2>
              <p className="text-slate-500 dark:text-slate-400 mb-6">Command-line tools for development and deployment.</p>
              <CodeBlock lang="bash" code={'npm install -g @modular-blockchain/cli'} />
              <div className="grid md:grid-cols-3 gap-3">
                {[
                  { cmd: 'modular init my-dapp --kit token', desc: 'Scaffold a new project from a template.' },
                  { cmd: 'modular deploy ERC20 --args "MyToken,MTK,1000000"', desc: 'Deploy smart contracts from the CLI.' },
                  { cmd: 'modular wallet generate', desc: 'Generate wallets and check balances.' },
                ].map(({ cmd, desc }) => (
                  <div key={cmd} className="p-4 rounded-xl glass-strong hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all">
                    <code className="text-xs text-blue-600 dark:text-cyan-400 bg-blue-500/10 dark:bg-blue-500/10 px-2 py-1 rounded block mb-2 font-mono">{cmd}</code>
                    <p className="text-sm text-slate-500 dark:text-slate-400">{desc}</p>
                  </div>
                ))}
              </div>
            </section>

            {/* ── CTA ────────────────────────────────────────────── */}
            <section className="rounded-3xl p-8 md:p-10 text-center glass-strong relative overflow-hidden">
              <div className="absolute -top-24 -right-24 w-72 h-72 rounded-full opacity-30 pointer-events-none"
                style={{ background: 'radial-gradient(circle, rgba(14,165,233,0.5), transparent 70%)' }} />
              <div className="relative">
                <div className="w-12 h-12 mx-auto rounded-2xl bg-gradient-to-br from-blue-600 to-cyan-600 flex items-center justify-center text-white shadow-lg shadow-blue-600/25 mb-4">
                  <PlusCircle className="w-6 h-6" />
                </div>
                <h2 className="text-2xl md:text-3xl font-bold text-slate-900 dark:text-white mb-2">Ready to start building?</h2>
                <p className="text-slate-500 dark:text-slate-400 max-w-lg mx-auto mb-6">
                  Pick a starter kit, generate a wallet with the CLI, and deploy your first contract to the local testnet in minutes.
                </p>
                <div className="flex flex-wrap justify-center gap-3">
                  <Link to="/docs" className="inline-flex items-center gap-2 px-6 py-3 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 text-white text-sm font-semibold hover:-translate-y-0.5 transition-all shadow-lg shadow-blue-600/25">
                    <ScrollText className="w-4 h-4" />
                    Read the docs
                  </Link>
                  <a href="https://github.com/fewzfewz/scratchBlockchain" target="_blank" rel="noopener noreferrer"
                    className="inline-flex items-center gap-2 px-6 py-3 rounded-xl glass text-sm font-semibold text-slate-700 dark:text-slate-200 hover:-translate-y-0.5 transition-all">
                    <Github className="w-4 h-4" />
                    View on GitHub
                  </a>
                </div>
              </div>
            </section>
          </div>
        )}

        <footer className="mt-12 pt-6 border-t border-slate-200 dark:border-slate-800 text-center text-xs text-slate-500 dark:text-slate-600">
          Nebula Developer Portal v0.1.0 · Scratch Blockchain · Open source
        </footer>
      </div>
    </div>
  )
}
