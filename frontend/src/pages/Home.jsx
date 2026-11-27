import { useEffect, useState, lazy, Suspense } from 'react'
import { Link } from 'react-router-dom'
import { motion } from 'framer-motion'
import { TabGroup, TabList, Tab, TabPanels, TabPanel } from '@headlessui/react'
import {
  Compass, Wallet, Droplets, Vote, BookOpen, FileCode, ArrowRight, Shield, Zap, Layers,
  Cpu, GitBranch, Image, Rocket, Package, FileJson, CheckCircle2, Timer, Network,
  Sparkles, Terminal, Activity, Copy, Check, Bot, Coins,
} from 'lucide-react'
import PageShell from '../components/PageShell.jsx'
import { getApiUrl, walletAddress, shorten } from '../lib/chain.js'

// three.js is large; load it only for the hero panel, after first paint.
const BlockchainScene = lazy(() => import('../components/BlockchainScene.jsx'))

const PRODUCTS = [
  {
    to: '/explorer',
    icon: Compass,
    title: 'Live Explorer',
    desc: 'Inspect finalized blocks, transactions, validators, and state roots in real time.',
    tag: 'Real-time',
    chip: 'from-blue-600 to-cyan-500',
  },
  {
    to: '/validators',
    icon: Shield,
    title: 'Staking & Validators',
    desc: 'Delegate NBL or run a validator node to earn block rewards and gas fees.',
    tag: 'Earn Yield',
    chip: 'from-emerald-600 to-teal-500',
  },
  {
    to: '/defi',
    icon: Layers,
    title: 'DeFi & SimpleAMM',
    desc: 'Deploy liquidity pools, provide tokens, and swap with sub-second finality.',
    tag: 'AMM DEX',
    chip: 'from-violet-600 to-indigo-500',
  },
  {
    to: '/bridge',
    icon: GitBranch,
    title: 'Ethereum Bridge',
    desc: 'Lock ETH and mint NBL via 2-of-3 independent relayer attestations.',
    tag: 'Cross-chain',
    chip: 'from-amber-500 to-orange-500',
  },
  {
    to: '/deploy',
    icon: Rocket,
    title: 'EVM & WASM Deploy',
    desc: 'Ship Solidity ERC-20/721 contracts or high-speed Rust/WASM modules.',
    tag: 'Dual VM',
    chip: 'from-fuchsia-600 to-pink-500',
  },
  {
    to: '/governance',
    icon: Vote,
    title: 'On-Chain Governance',
    desc: 'Propose upgrades, vote with staked NBL, and execute treasury grants.',
    tag: 'DAO',
    chip: 'from-sky-600 to-blue-500',
  },
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

const fadeUp = {
  hidden: { opacity: 0, y: 18 },
  visible: (i = 0) => ({
    opacity: 1,
    y: 0,
    transition: { duration: 0.45, delay: i * 0.06, ease: [0.22, 1, 0.36, 1] },
  }),
}

function StatCard({ label, value, accent }) {
  return (
    <motion.div
      whileHover={{ y: -2 }}
      transition={{ type: 'spring', stiffness: 360, damping: 26 }}
      className="rounded-2xl glass px-4 py-3.5 border border-slate-200/70 dark:border-slate-800/80"
    >
      <div className="text-[11px] uppercase tracking-wider text-slate-500 dark:text-slate-400">{label}</div>
      <div className={`mt-1 font-display text-2xl font-semibold tabular-nums ${accent || 'text-slate-900 dark:text-white'}`}>
        {value}
      </div>
    </motion.div>
  )
}

export default function Home() {
  const [status, setStatus] = useState(null)
  const [validators, setValidators] = useState(null)
  const [baseFeeGwei, setBaseFeeGwei] = useState(null)
  const [blocks, setBlocks] = useState([])
  const [activeTab, setActiveTab] = useState(0)
  const [copiedCmd, setCopiedCmd] = useState(false)
  const hasWallet = !!walletAddress()
  const apiUrl = getApiUrl()

  useEffect(() => {
    let active = true
    const load = async () => {
      try {
        const [sr, vr, br, gr] = await Promise.all([
          window.fetch(`${apiUrl}/status`),
          window.fetch(`${apiUrl}/validators`),
          window.fetch(`${apiUrl}/block/latest`),
          window.fetch(`${apiUrl}/gas_price`).catch(() => null),
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
            .map((s) => window.fetch(`${apiUrl}/block/${s}`).then((r) => r.json()).then((d) => d.block).catch(() => null)),
        )
        if (active) setBlocks(recent.filter(Boolean))
      } catch {
        if (active) setStatus({ online: false })
      }
    }
    load()
    const t = setInterval(load, 4000)
    return () => { active = false; clearInterval(t) }
  }, [apiUrl])

  const online = status?.online

  const devTabs = [
    {
      id: 'js',
      label: 'JavaScript SDK',
      icon: Package,
      install: 'npm install nebula-chain-sdk',
      ctaTo: '/sdk',
      ctaLabel: 'Explore JS SDK',
      code: `import { NebulaClient, HttpProvider, Wallet, parseNbl } from "nebula-chain-sdk";

const client = new NebulaClient(new HttpProvider("${apiUrl}"));
const wallet = Wallet.generate();
const tx = await wallet.signTransaction({ to: "0x...", value: parseNbl("1.5") });
const receipt = await client.sendTransaction(tx);`,
    },
    {
      id: 'py',
      label: 'Python & AI Agents',
      icon: Bot,
      install: 'pip install "nebula-chain-sdk[signing]"',
      ctaTo: '/sdk',
      ctaLabel: 'Explore Python SDK',
      code: `from nebula import NebulaClient, Wallet

client = NebulaClient("${apiUrl}")
print("Height:", client.status()["height"])

agent = Wallet.generate()
client.faucet(agent.address)
agent.send_nbl(client, to="0x...", amount_nbl=0.25)`,
    },
    {
      id: 'cli',
      label: 'Developer CLI',
      icon: Terminal,
      install: 'npm install -g nebula-chain-cli',
      ctaTo: '/deploy',
      ctaLabel: 'Deploy Contracts',
      code: `# Scaffold, generate a wallet, and deploy an ERC-20 token
npx nebula-chain-cli wallet --generate
npx nebula-chain-cli init --template defi --name my-nebula-app
npx nebula-chain-cli deploy --contract ERC20 --args "NebulaToken,NBT,1000000"`,
    },
    {
      id: 'stake',
      label: 'Validator & Yield',
      icon: Coins,
      install: 'nebula-testnet-1 · BFT Consensus',
      ctaTo: '/validators/onboard',
      ctaLabel: 'Run a Validator',
      code: `# 1. Claim test NBL on /faucet and generate your validator key
# 2. Register on-chain via /validators/onboard (min 1 NBL stake)
# 3. Run your node and earn block rewards + gas fee share:
docker run -d -p 26656:26656 -p 8545:8545 \\
  -v ./validator_key.json:/data/validator_key.json \\
  ghcr.io/fewzfewz/nebula-node:main`,
    },
  ]

  const currentTab = devTabs[activeTab] || devTabs[0]

  const copyInstall = () => {
    navigator.clipboard.writeText(currentTab.install)
    setCopiedCmd(true)
    setTimeout(() => setCopiedCmd(false), 1800)
  }

  return (
    <PageShell>
      {/* ── Hero ─────────────────────────────────────────────────── */}
      <section className="relative max-w-7xl mx-auto px-4 pt-12 lg:pt-20 pb-14 grid lg:grid-cols-[1.15fr_1fr] gap-10 items-center">
        <motion.div initial="hidden" animate="visible" variants={fadeUp}>
          <motion.div
            custom={0}
            variants={fadeUp}
            className="inline-flex items-center gap-2 h-8 px-3.5 rounded-full glass border border-slate-200 dark:border-slate-800 text-xs font-medium text-slate-700 dark:text-slate-200 mb-6"
          >
            <span className={`w-2 h-2 rounded-full ${online ? 'bg-emerald-400 animate-pulse-dot' : status ? 'bg-rose-400' : 'bg-slate-400'}`} />
            {online
              ? <>Nebula Testnet Live <span className="text-slate-400">·</span> <span className="font-mono text-cyan-600 dark:text-cyan-400">Block #{fmt(status.height)}</span></>
              : status ? 'RPC Offline — reconnecting' : 'Connecting to Nebula RPC…'}
          </motion.div>

          <motion.h1
            custom={1}
            variants={fadeUp}
            className="text-4xl sm:text-5xl xl:text-[3.75rem] font-bold tracking-tight leading-[1.06] text-slate-900 dark:text-white"
          >
            The Modular Chain<br />
            <span className="text-aurora">Built for Speed &amp; Yield.</span>
          </motion.h1>

          <motion.p
            custom={2}
            variants={fadeUp}
            className="mt-5 text-base sm:text-lg text-slate-600 dark:text-slate-300 max-w-xl leading-relaxed"
          >
            Sub-second BFT finality, dual EVM + WASM smart contracts, an on-chain AMM, and published JavaScript &amp; Python SDKs — all connected live to the Nebula network.
          </motion.p>

          <motion.div custom={3} variants={fadeUp} className="mt-7 flex flex-wrap gap-3">
            <Link
              to="/wallet"
              className="group inline-flex items-center gap-2 h-11 px-5 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-500 text-white text-sm font-semibold shadow-lg shadow-blue-600/25 hover:opacity-95 transition"
            >
              <Wallet className="w-4 h-4" /> {hasWallet ? 'Open Wallet' : 'Create Wallet'}
              <ArrowRight className="w-4 h-4 group-hover:translate-x-0.5 transition-transform" />
            </Link>
            <Link
              to="/faucet"
              className="inline-flex items-center gap-2 h-11 px-5 rounded-xl glass text-sm font-semibold text-slate-900 dark:text-white hover:border-cyan-400/50 transition"
            >
              <Droplets className="w-4 h-4 text-cyan-500" /> Claim Free NBL
            </Link>
            <Link
              to="/validators/onboard"
              className="inline-flex items-center gap-2 h-11 px-5 rounded-xl glass text-sm font-semibold text-slate-900 dark:text-white hover:border-emerald-400/50 transition"
            >
              <Shield className="w-4 h-4 text-emerald-500" /> Run Validator
            </Link>
          </motion.div>

          <motion.div custom={4} variants={fadeUp} className="mt-8 grid grid-cols-2 sm:grid-cols-4 gap-3 max-w-2xl">
            <StatCard label="Block Height" value={fmt(status?.height)} accent="text-cyan-600 dark:text-cyan-400" />
            <StatCard label="Validators" value={fmt(validators)} />
            <StatCard label="Base Fee" value={baseFeeGwei != null ? `${baseFeeGwei} gwei` : '—'} />
            <StatCard label="Mempool" value={fmt(status?.mempool_size)} />
          </motion.div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, scale: 0.97 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.55, ease: [0.22, 1, 0.36, 1] }}
          className="relative h-[380px] lg:h-[470px] rounded-3xl glass-strong aurora-border overflow-hidden shadow-2xl shadow-blue-500/10"
        >
          <Suspense fallback={<div className="absolute inset-0 animate-pulse bg-slate-100 dark:bg-slate-900" />}>
            <BlockchainScene className="absolute inset-0" />
          </Suspense>

          <div className="absolute top-4 left-4 right-4 flex items-center justify-between pointer-events-none">
            <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-white/85 dark:bg-slate-950/80 backdrop-blur border border-slate-200/70 dark:border-slate-800 text-[11px] font-medium text-slate-700 dark:text-slate-200">
              <Activity className="w-3.5 h-3.5 text-emerald-500" />
              {online ? `Finalized #${fmt(status?.finalized_height ?? status?.height)}` : 'Awaiting RPC'}
            </span>
            <span className="px-3 py-1 rounded-full bg-white/85 dark:bg-slate-950/80 backdrop-blur border border-slate-200/70 dark:border-slate-800 text-[11px] font-mono text-slate-600 dark:text-slate-300">
              Chain ID: {status?.chain_id ?? 1}
            </span>
          </div>

          <div className="absolute bottom-0 inset-x-0 p-5 bg-gradient-to-t from-white/95 via-white/75 dark:from-slate-950/95 dark:via-slate-950/75 to-transparent">
            <div className="flex items-center justify-between gap-4">
              <div>
                <div className="text-xs font-semibold text-slate-900 dark:text-white">Interactive 3D Consensus Core</div>
                <div className="text-xs text-slate-500 dark:text-slate-400">Drag to orbit · BFT validator ring &amp; execution layers</div>
              </div>
              <Link to="/explorer" className="shrink-0 inline-flex items-center gap-1 text-xs font-semibold text-blue-600 dark:text-cyan-400 hover:underline">
                Explorer <ArrowRight className="w-3.5 h-3.5" />
              </Link>
            </div>
          </div>
        </motion.div>
      </section>

      {/* ── Interactive Developer & Earning Showcase ─────────────── */}
      <section className="max-w-7xl mx-auto px-4 pb-16">
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, amount: 0.2 }}
          transition={{ duration: 0.45 }}
          className="rounded-3xl glass-strong p-6 sm:p-8 border border-slate-200/80 dark:border-slate-800/90"
        >
          <TabGroup selectedIndex={activeTab} onChange={setActiveTab}>
            <div className="flex flex-wrap items-center justify-between gap-4 mb-6">
              <div>
                <div className="inline-flex items-center gap-1.5 text-xs font-semibold uppercase tracking-wider text-cyan-600 dark:text-cyan-400 mb-1">
                  <Sparkles className="w-3.5 h-3.5" /> Developer &amp; Operator Stack
                </div>
                <h2 className="text-2xl font-semibold text-slate-900 dark:text-white">Ship apps, AI agents, or validators in minutes</h2>
              </div>

              <TabList className="flex flex-wrap gap-1.5 p-1 rounded-xl bg-slate-100 dark:bg-slate-900/90 border border-slate-200 dark:border-slate-800">
                {devTabs.map(({ id, label, icon: Icon }) => (
                  <Tab
                    key={id}
                    className={({ selected }) =>
                      `relative px-3.5 py-2 rounded-lg text-xs font-semibold inline-flex items-center gap-1.5 outline-none transition-colors ${
                        selected ? 'text-white' : 'text-slate-600 dark:text-slate-300 hover:text-slate-900 dark:hover:text-white'
                      }`
                    }
                  >
                    {({ selected }) => (
                      <>
                        {selected && (
                          <motion.span
                            layoutId="homeDevTab"
                            transition={{ type: 'spring', stiffness: 420, damping: 32 }}
                            className="absolute inset-0 rounded-lg bg-gradient-to-r from-blue-600 to-cyan-500 shadow-sm"
                          />
                        )}
                        <Icon className="relative z-10 w-3.5 h-3.5" />
                        <span className="relative z-10">{label}</span>
                      </>
                    )}
                  </Tab>
                ))}
              </TabList>
            </div>

            <TabPanels>
              {devTabs.map((tab) => (
                <TabPanel key={tab.id} className="grid lg:grid-cols-[1fr_1.6fr] gap-5 items-stretch outline-none">
                  <div className="rounded-2xl bg-slate-50 dark:bg-slate-900/70 border border-slate-200 dark:border-slate-800 p-5 flex flex-col justify-between gap-4">
                    <div>
                      <div className="text-[11px] uppercase tracking-wider text-slate-500 font-semibold mb-2">Package / Command</div>
                      <div className="flex items-center justify-between gap-2 rounded-xl bg-white dark:bg-slate-950 border border-slate-200 dark:border-slate-800 px-3.5 py-2.5">
                        <code className="text-xs sm:text-sm font-mono text-slate-800 dark:text-cyan-300 truncate">{tab.install}</code>
                        <button
                          type="button"
                          onClick={copyInstall}
                          className="shrink-0 p-1.5 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-500 transition-colors"
                          aria-label="Copy command"
                        >
                          {copiedCmd ? <Check className="w-4 h-4 text-emerald-500" /> : <Copy className="w-4 h-4" />}
                        </button>
                      </div>
                    </div>

                    <div className="flex flex-wrap items-center justify-between gap-3 pt-2">
                      <span className="text-xs text-slate-500 dark:text-slate-400">Live RPC: <code className="font-mono">{apiUrl}</code></span>
                      <Link
                        to={tab.ctaTo}
                        className="inline-flex items-center gap-1.5 px-4 py-2 rounded-xl bg-slate-900 dark:bg-white text-white dark:text-slate-900 text-xs font-semibold hover:opacity-90 transition"
                      >
                        {tab.ctaLabel} <ArrowRight className="w-3.5 h-3.5" />
                      </Link>
                    </div>
                  </div>

                  <pre className="rounded-2xl bg-slate-950 text-slate-200 border border-slate-800 p-5 text-xs sm:text-sm font-mono overflow-x-auto leading-relaxed">
                    <code>{tab.code}</code>
                  </pre>
                </TabPanel>
              ))}
            </TabPanels>
          </TabGroup>
        </motion.div>
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
            <motion.li key={n} whileHover={{ y: -3 }} transition={{ type: 'spring', stiffness: 350, damping: 25 }}>
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
            </motion.li>
          ))}
        </ol>
      </section>

      {/* ── Live blocks ──────────────────────────────────────────── */}
      <section className="max-w-7xl mx-auto px-4 pb-16">
        <div className="flex items-end justify-between mb-4">
          <div>
            <h2 className="text-2xl font-semibold text-slate-900 dark:text-white">Latest finalized blocks</h2>
            <p className="text-sm text-slate-500 mt-1">Streamed directly from <span className="font-mono">{apiUrl}</span></p>
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
                  <motion.tr
                    key={b.header.slot}
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    className="border-b last:border-0 border-slate-100 dark:border-slate-800/70 hover:bg-slate-50/70 dark:hover:bg-slate-800/30 transition-colors"
                  >
                    <td className="px-5 py-3 font-mono font-medium">
                      <Link to={`/explorer?block=${b.header.slot}`} className="text-blue-600 dark:text-cyan-400 hover:underline">
                        #{fmt(b.header.slot)}
                      </Link>
                    </td>
                    <td className="px-5 py-3 font-mono text-slate-500">{shorten(hex(b.header.proposer), 8, 6) || '—'}</td>
                    <td className="px-5 py-3 text-right tabular-nums">{b.extrinsics?.length ?? 0}</td>
                    <td className="px-5 py-3 text-right tabular-nums text-slate-500">{fmt(b.header.gas_used)}</td>
                    <td className="px-5 py-3 text-right tabular-nums text-slate-500">{b.commit?.precommits?.length ?? '—'}</td>
                  </motion.tr>
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
          {PRODUCTS.map(({ to, icon: Icon, title, desc, tag, chip }) => (
            <motion.div key={to} whileHover={{ y: -4 }} transition={{ type: 'spring', stiffness: 360, damping: 24 }}>
              <Link to={to} className="group block h-full p-6 rounded-2xl glass border border-slate-200/70 dark:border-slate-800/80">
                <div className="flex items-center justify-between">
                  <span className={`w-10 h-10 rounded-xl bg-gradient-to-br ${chip} text-white shadow-md flex items-center justify-center`}>
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
            </motion.div>
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
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, amount: 0.2 }}
          transition={{ duration: 0.45 }}
          className="rounded-3xl glass-strong p-8 lg:p-10"
        >
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
        </motion.div>
      </section>
    </PageShell>
  )
}
