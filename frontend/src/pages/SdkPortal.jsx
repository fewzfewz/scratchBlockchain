import { useState, useEffect } from 'react'
import {
  FileCode, Terminal, BookOpen, Activity, Gauge, ShieldCheck,
  WifiOff, Copy, Check, Package, Rocket, Zap, Shield, Layers, ArrowRight,
  Wrench, Boxes, PlusCircle, Wallet,
} from 'lucide-react'
import { Link } from 'react-router-dom'

const RPC_URL = 'http://localhost:8545'
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

export default function SdkPortal() {
  const [loading, setLoading] = useState(true)
  const [backendOnline, setBackendOnline] = useState(true)
  const [network, setNetwork] = useState(null)

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

  useEffect(() => {
    const t = setTimeout(() => setLoading(false), 350)
    return () => clearTimeout(t)
  }, [])

  return (
    <div className="relative min-h-screen overflow-hidden">
      {/* Aurora blobs + grid backdrop */}
      <div className="absolute -top-40 -left-40 w-[38rem] h-[38rem] rounded-full opacity-25 animate-float pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(14,165,233,0.45), transparent 70%)' }} />
      <div className="absolute top-40 -right-40 w-[34rem] h-[34rem] rounded-full opacity-20 animate-float-alt pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(139,92,246,0.4), transparent 70%)' }} />
      <div className="absolute inset-0 bg-grid pointer-events-none" />

      <div className="relative z-10 max-w-5xl mx-auto px-4 py-8 md:py-10 animate-fade-in">
        {/* ── Header ─────────────────────────────────────────────── */}
        <header className="text-center mb-8">
          <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full glass text-xs font-medium text-slate-600 dark:text-slate-300 mb-5">
            <span className={`w-2 h-2 rounded-full ${backendOnline ? 'bg-emerald-400 animate-pulse-dot' : 'bg-red-400'}`} />
            {backendOnline ? 'Local testnet live' : 'Node offline'}
            <span className="text-slate-400 dark:text-slate-500">· TypeScript SDK · CLI</span>
          </div>

          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-indigo-600 to-violet-600 flex items-center justify-center text-white shadow-lg shadow-indigo-600/25">
              <FileCode className="w-6 h-6" />
            </div>
            <h1 className="text-4xl md:text-5xl font-bold tracking-tight text-slate-900 dark:text-white">
              Nebula
              <span className="bg-gradient-to-r from-indigo-500 via-blue-500 to-cyan-500 bg-clip-text text-transparent"> SDK</span>
            </h1>
          </div>
          <p className="text-slate-500 dark:text-slate-400 max-w-xl mx-auto">
            Build decentralized applications on a high-performance, modular blockchain with Ed25519 security and EIP-1559 gas mechanics.
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
            <span>Node not reachable at {RPC_URL}. The SDK code samples still apply — start the testnet to run them.</span>
          </div>
        )}

        {loading ? <Skeleton /> : (
          <div className="space-y-10">
            {/* ── Quick stats ────────────────────────────────────── */}
            <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
              {[
                { icon: Zap, label: 'Block time', value: '1s', sub: 'Target block interval', chip: 'from-amber-500 to-orange-600' },
                { icon: Gauge, label: 'Throughput', value: '10k+', sub: 'Transactions / second', chip: 'from-emerald-600 to-teal-600' },
                { icon: Shield, label: 'Cryptography', value: 'Ed25519', sub: 'Signature scheme', chip: 'from-blue-600 to-cyan-600' },
                { icon: Layers, label: 'Fee market', value: 'EIP-1559', sub: 'Base + priority fees', chip: 'from-violet-600 to-purple-600' },
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

            {/* ── Quick links ────────────────────────────────────── */}
            <div className="flex flex-wrap justify-center gap-3">
              {[
                { to: '/docs', label: 'Quick Start', icon: BookOpen },
                { to: '/api-docs', label: 'API Reference', icon: Terminal },
                { to: '/governance', label: 'Governance', icon: Wrench },
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

            {/* ── Quick Start ────────────────────────────────────── */}
            <section>
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-2">Quick Start</h2>
              <p className="text-slate-500 dark:text-slate-400 mb-6">Get connected and send your first transaction in 30 seconds.</p>
              <div className="space-y-6">
                {[
                  { num: '1', title: 'Install the SDK', code: 'npm install @modular-blockchain/sdk' },
                  { num: '2', title: 'Connect to a node', code: `import { ModularClient, HttpProvider } from "@modular-blockchain/sdk";

const provider = new HttpProvider("http://localhost:8545");
const client = new ModularClient(provider);
const status = await client.getNodeStatus();
console.log("Height:", status.height);` },
                  { num: '3', title: 'Create a wallet', code: `import { Wallet } from "@modular-blockchain/sdk";

const wallet = Wallet.generate();
console.log("Address:", wallet.address); // 0x...` },
                  { num: '4', title: 'Send a transaction', code: `const tx = await wallet.signTransaction({
  to: "0x...",
  value: "1000000000000000000", // 1 NBL
});
const receipt = await client.sendTransaction(tx);
console.log("Tx hash:", receipt.hash);` },
                ].map(({ num, title, code }) => (
                  <div key={num} className="flex gap-4 md:gap-5 pb-6 border-b border-slate-200 dark:border-slate-700/50 last:border-0 last:pb-0">
                    <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-blue-600 to-cyan-600 flex items-center justify-center font-bold text-white shadow-md shadow-blue-600/20 shrink-0">
                      {num}
                    </div>
                    <div className="min-w-0 flex-1">
                      <h3 className="text-slate-800 dark:text-white font-semibold mb-3">{title}</h3>
                      <CodeBlock code={code} lang={num === '1' ? 'bash' : 'ts'} />
                    </div>
                  </div>
                ))}
              </div>
            </section>

            {/* ── SDK Packages ───────────────────────────────────── */}
            <section>
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-2">SDK Packages</h2>
              <p className="text-slate-500 dark:text-slate-400 mb-6">Everything you need to build on Nebula.</p>
              <div className="grid md:grid-cols-3 gap-4">
                {[
                  { icon: Package, title: '@modular-blockchain/sdk', desc: 'TypeScript SDK with wallet management, transaction building, and providers.', tags: ['TypeScript', 'Node.js'], chip: 'from-blue-600 to-cyan-600', install: 'npm install @modular-blockchain/sdk' },
                  { icon: Terminal, title: '@modular-blockchain/sdk-cli', desc: 'CLI for scaffolding projects, deploying contracts, and managing wallets.', tags: ['CLI', 'Scaffold'], chip: 'from-emerald-600 to-teal-600', install: 'npm install -g @modular-blockchain/sdk-cli' },
                  { icon: Boxes, title: 'Smart Contract Templates', desc: 'Battle-tested templates for ERC-20, ERC-721, and DAO contracts.', tags: ['Solidity', 'Open Source'], chip: 'from-violet-600 to-purple-600', install: 'npm i @modular-blockchain/templates' },
                ].map(({ icon: Icon, title, desc, tags, chip, install }) => (
                  <div key={title} className="p-5 rounded-2xl glass-strong hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all">
                    <div className="flex items-center gap-3 mb-4">
                      <div className={`w-9 h-9 rounded-xl bg-gradient-to-br ${chip} flex items-center justify-center text-white shadow-md`}>
                        <Icon className="w-4 h-4" />
                      </div>
                      <h3 className="text-slate-800 dark:text-white font-semibold text-sm leading-snug">{title}</h3>
                    </div>
                    <p className="text-sm text-slate-500 dark:text-slate-400 mb-4">{desc}</p>
                    <div className="flex flex-wrap gap-2 mb-4">
                      {tags.map(t => (
                        <span key={t} className="px-2 py-0.5 rounded text-xs font-medium bg-blue-500/15 text-blue-600 dark:text-blue-400">{t}</span>
                      ))}
                    </div>
                    <div className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg bg-slate-100 dark:bg-slate-700/40 border border-slate-200 dark:border-slate-600/40">
                      <code className="text-[11px] font-mono text-slate-600 dark:text-slate-300 flex-1 truncate">{install}</code>
                      <CopyBtn text={install} />
                    </div>
                  </div>
                ))}
              </div>
            </section>

            {/* ── CLI Tool ───────────────────────────────────────── */}
            <section>
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-2">CLI Tool</h2>
              <p className="text-slate-500 dark:text-slate-400 mb-6">Scaffold, deploy, and manage your dApps from the terminal.</p>
              <CodeBlock code={`# Install
npm install -g @modular-blockchain/sdk-cli

# Generate wallet
modular wallet --generate

# Scaffold a DeFi project
modular init --template defi --name my-amm

# Deploy a contract
modular deploy --contract ERC20 --args "MyToken,MTK,1000000"`} lang="bash" />
              <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-3">
                {[
                  { icon: Rocket, title: 'modular init', desc: 'Scaffold projects from templates.' },
                  { icon: Terminal, title: 'modular deploy', desc: 'Deploy bytecode or templates.' },
                  { icon: PlusCircle, title: 'modular create', desc: 'Generate contract files.' },
                  { icon: Wallet, title: 'modular wallet', desc: 'Generate and manage wallets.' },
                ].map(({ icon: Icon, title, desc }) => (
                  <div key={title} className="p-4 rounded-xl glass-strong hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all">
                    <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-slate-500 to-slate-600 flex items-center justify-center text-white shadow-md mb-3">
                      <Icon className="w-4 h-4" />
                    </div>
                    <h3 className="text-slate-800 dark:text-white font-semibold text-sm mb-1 font-mono">{title}</h3>
                    <p className="text-xs text-slate-500 dark:text-slate-400">{desc}</p>
                  </div>
                ))}
              </div>
            </section>

            {/* ── RPC API ────────────────────────────────────────── */}
            <section>
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-2">RPC API</h2>
              <p className="text-slate-500 dark:text-slate-400 mb-6">The Nebula node exposes these REST endpoints — explore them interactively.</p>
              <CodeBlock code={`# Node status
GET /status

# Submit transaction
POST /submit_tx  body: Transaction JSON

# Query
GET /block/{height}     GET /balance/{address}
GET /tx/{hash}          GET /gas_price
GET /fee_history/{count}

# Mempool
GET /mempool

# Network
GET /peers              POST /connect_peer`} lang="text" />
              <Link to="/api-docs" className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 text-white text-sm font-semibold hover:-translate-y-0.5 transition-all shadow-lg shadow-blue-600/20">
                <Terminal className="w-4 h-4" />
                Try the interactive API docs
                <ArrowRight className="w-3.5 h-3.5" />
              </Link>
            </section>
          </div>
        )}

        <footer className="mt-12 pt-6 border-t border-slate-200 dark:border-slate-800 text-center text-xs text-slate-500 dark:text-slate-600">
          Nebula SDK v0.1.0 · Scratch Blockchain · Open source
        </footer>
      </div>
    </div>
  )
}

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
