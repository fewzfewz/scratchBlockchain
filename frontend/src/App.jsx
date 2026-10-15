import { useState, useEffect, useRef, lazy, Suspense } from 'react'
import { Routes, Route, NavLink, Link, useLocation } from 'react-router-dom'
import {
  Wallet, Droplets, Compass, BookOpen, ChevronDown, FileCode, Vote, ExternalLink,
  Github, Sun, Moon, Menu, X, FileJson, Rocket, Shield, History, Layers, GitBranch,
  Cpu, Zap, Image, Package, Plus,
} from 'lucide-react'
import { useTheme } from './ThemeContext.jsx'
import { getApiUrl, walletAddress, weiToNbl, shorten } from './lib/chain.js'
import Home from './pages/Home.jsx'

// Route-level code splitting: only Home ships in the main bundle.
const Explorer = lazy(() => import('./pages/Explorer.jsx'))
const WalletPage = lazy(() => import('./pages/WalletPage.jsx'))
const FaucetPage = lazy(() => import('./pages/FaucetPage.jsx'))
const DocsPage = lazy(() => import('./pages/DocsPage.jsx'))
const DeveloperPortal = lazy(() => import('./pages/DeveloperPortal.jsx'))
const SdkPortal = lazy(() => import('./pages/SdkPortal.jsx'))
const Governance = lazy(() => import('./pages/Governance.jsx'))
const ContractDeployPage = lazy(() => import('./pages/ContractDeployPage.jsx'))
const ValidatorOnboardPage = lazy(() => import('./pages/ValidatorOnboardPage.jsx'))
const HistoryPage = lazy(() => import('./pages/HistoryPage.jsx'))
const DeFiPage = lazy(() => import('./pages/DeFiPage.jsx'))
const ContractInteractPage = lazy(() => import('./pages/ContractInteractPage.jsx'))
const BridgePage = lazy(() => import('./pages/BridgePage.jsx'))
const NftPage = lazy(() => import('./pages/NftPage.jsx'))
const WasmPage = lazy(() => import('./pages/WasmPage.jsx'))
const AdvancedPage = lazy(() => import('./pages/AdvancedPage.jsx'))
const ApiDocs = lazy(() => import('./pages/ApiDocs.jsx'))

const GITHUB_URL = 'https://github.com/fewzfewz/scratchBlockchain'

// Top-level links, then grouped menus (also the mobile drawer and footer).
const DIRECT_NAV = [
  { to: '/explorer', icon: Compass, label: 'Explorer' },
  { to: '/wallet', icon: Wallet, label: 'Wallet' },
]

export const NAV_GROUPS = [
  {
    title: 'Build',
    items: [
      { to: '/deploy', icon: Rocket, label: 'Deploy', desc: 'Ship an ERC20 or ERC721' },
      { to: '/contracts', icon: FileCode, label: 'Contracts', desc: 'Read and write contracts' },
      { to: '/nft', icon: Image, label: 'NFT', desc: 'Mint and browse tokens' },
      { to: '/wasm', icon: Cpu, label: 'WASM', desc: 'Deploy and call modules' },
      { to: '/advanced', icon: Zap, label: 'Advanced', desc: 'Account abstraction, MEV' },
    ],
  },
  {
    title: 'Finance',
    items: [
      { to: '/defi', icon: Layers, label: 'DeFi', desc: 'Swap and provide liquidity' },
      { to: '/bridge', icon: GitBranch, label: 'Bridge', desc: 'Move ETH to Nebula and back' },
      { to: '/faucet', icon: Droplets, label: 'Faucet', desc: 'Free test NBL' },
    ],
  },
  {
    title: 'Network',
    items: [
      { to: '/governance', icon: Vote, label: 'Governance', desc: 'Proposals, votes, treasury' },
      { to: '/validators/onboard', icon: Shield, label: 'Run a validator', desc: 'Checks and registration' },
      { to: '/history', icon: History, label: 'History', desc: 'Transactions by address' },
    ],
  },
  {
    title: 'Developers',
    items: [
      { to: '/docs', icon: BookOpen, label: 'Docs', desc: 'Guides and architecture' },
      { to: '/api-docs', icon: FileJson, label: 'API reference', desc: 'Every RPC route' },
      { to: '/sdk', icon: Package, label: 'SDK', desc: 'JavaScript SDK and CLI' },
      { to: '/developer-portal', icon: ExternalLink, label: 'Dev portal', desc: 'Starter kits' },
    ],
  },
]

const iconButton =
  'p-2 rounded-lg text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors'

const linkClass = (isActive, base) =>
  `${base} ${
    isActive
      ? 'text-slate-900 dark:text-white bg-slate-100 dark:bg-slate-800'
      : 'text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white hover:bg-slate-100 dark:hover:bg-slate-800/70'
  }`

/** Poll the node: online, height, peers. */
function useNetworkStatus() {
  const [status, setStatus] = useState(null)
  useEffect(() => {
    let active = true
    const poll = async () => {
      try {
        const r = await window.fetch(`${getApiUrl()}/status`)
        const d = await r.json()
        if (active) setStatus({ online: true, ...d })
      } catch {
        if (active) setStatus({ online: false })
      }
    }
    poll()
    const t = setInterval(poll, 5000)
    return () => { active = false; clearInterval(t) }
  }, [])
  return status
}

/** The browser wallet's address and balance (refreshes on navigation). */
function useWalletSummary() {
  const { pathname } = useLocation()
  const [summary, setSummary] = useState({ address: walletAddress(), balance: null })
  useEffect(() => {
    let active = true
    const load = async () => {
      const address = walletAddress()
      if (!address) { if (active) setSummary({ address: '', balance: null }); return }
      try {
        const r = await window.fetch(`${getApiUrl()}/balance/${address.replace(/^0x/, '')}`)
        const d = await r.json()
        if (active) setSummary({ address, balance: weiToNbl(d.balance || '0') })
      } catch {
        if (active) setSummary({ address, balance: null })
      }
    }
    load()
    const t = setInterval(load, 8000)
    return () => { active = false; clearInterval(t) }
  }, [pathname])
  return summary
}

function Logo() {
  return (
    <Link to="/" className="flex items-center gap-2.5 shrink-0" aria-label="Nebula home">
      <svg width="28" height="28" viewBox="0 0 32 32" aria-hidden="true">
        <defs>
          <linearGradient id="nebula-mark" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stopColor="#8f7ffa" />
            <stop offset="0.55" stopColor="#6847e6" />
            <stop offset="1" stopColor="#2fcfc1" />
          </linearGradient>
        </defs>
        <path d="M16 2l12.1 7v14L16 30 3.9 23V9z" fill="url(#nebula-mark)" />
        <path d="M11 21V11l10 10V11" fill="none" stroke="#fff" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
      <span className="font-display font-semibold text-[17px] tracking-tight text-slate-900 dark:text-white">Nebula</span>
    </Link>
  )
}

function GroupMenu({ group }) {
  const [open, setOpen] = useState(false)
  const ref = useRef(null)
  const { pathname } = useLocation()
  const active = group.items.some((i) => pathname.startsWith(i.to))

  useEffect(() => setOpen(false), [pathname])
  useEffect(() => {
    if (!open) return
    const onClick = (e) => { if (ref.current && !ref.current.contains(e.target)) setOpen(false) }
    const onKey = (e) => { if (e.key === 'Escape') setOpen(false) }
    document.addEventListener('mousedown', onClick)
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('mousedown', onClick)
      document.removeEventListener('keydown', onKey)
    }
  }, [open])

  return (
    <div className="relative" ref={ref}>
      <button
        onClick={() => setOpen((o) => !o)}
        aria-expanded={open}
        aria-haspopup="menu"
        className={linkClass(active, 'flex items-center gap-1 px-3 py-1.5 rounded-lg text-sm font-medium transition-colors')}
      >
        {group.title}
        <ChevronDown className={`w-3.5 h-3.5 transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>
      {open && (
        <div role="menu" className="absolute left-0 mt-2 w-72 p-1.5 rounded-xl glass-strong shadow-xl animate-fade-in">
          {group.items.map(({ to, icon: Icon, label, desc }) => (
            <NavLink key={to} to={to} role="menuitem"
              className={({ isActive }) => `flex items-start gap-3 p-2.5 rounded-lg transition-colors ${isActive ? 'bg-slate-100 dark:bg-slate-800' : 'hover:bg-slate-100 dark:hover:bg-slate-800/70'}`}>
              <span className="mt-0.5 w-8 h-8 rounded-lg bg-blue-500/10 text-blue-600 dark:text-blue-300 flex items-center justify-center shrink-0">
                <Icon className="w-4 h-4" />
              </span>
              <span>
                <span className="block text-sm font-medium text-slate-900 dark:text-white">{label}</span>
                <span className="block text-xs text-slate-500">{desc}</span>
              </span>
            </NavLink>
          ))}
        </div>
      )}
    </div>
  )
}

function NetworkPill({ status }) {
  const online = status?.online
  return (
    <Link to="/explorer" title={online ? `${status.peer_count ?? 0} peers` : 'Node unreachable'}
      className="hidden md:flex items-center gap-2 h-8 px-3 rounded-lg border border-slate-200 dark:border-slate-800 text-xs font-medium text-slate-600 dark:text-slate-300 hover:border-slate-300 dark:hover:border-slate-700 transition-colors">
      <span className={`w-1.5 h-1.5 rounded-full ${online ? 'bg-emerald-400 animate-pulse-dot' : status ? 'bg-rose-400' : 'bg-slate-400'}`} />
      {online ? <span className="font-mono tabular-nums">#{Number(status.height).toLocaleString()}</span> : status ? 'Offline' : '…'}
    </Link>
  )
}

function WalletPill({ summary }) {
  if (!summary.address) {
    return (
      <Link to="/wallet" className="flex items-center gap-1.5 h-8 px-3 rounded-lg bg-gradient-to-r from-blue-600 to-cyan-600 text-white text-xs font-semibold hover:opacity-90 transition-opacity">
        <Plus className="w-3.5 h-3.5" /> Create wallet
      </Link>
    )
  }
  return (
    <Link to="/wallet" title={summary.address}
      className="flex items-center gap-2 h-8 pl-1 pr-3 rounded-lg border border-slate-200 dark:border-slate-800 hover:border-slate-300 dark:hover:border-slate-700 transition-colors">
      <span className="w-6 h-6 rounded-md bg-gradient-to-br from-blue-500 to-cyan-500" aria-hidden="true" />
      <span className="text-xs font-semibold tabular-nums text-slate-900 dark:text-white">{summary.balance ?? '—'} <span className="text-slate-400 font-normal">NBL</span></span>
      <span className="hidden sm:inline font-mono text-[11px] text-slate-500">{shorten(summary.address, 6, 4)}</span>
    </Link>
  )
}

function MobileDrawer({ onClose }) {
  useEffect(() => {
    const onKey = (e) => { if (e.key === 'Escape') onClose() }
    document.addEventListener('keydown', onKey)
    const prevOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    return () => {
      document.removeEventListener('keydown', onKey)
      document.body.style.overflow = prevOverflow
    }
  }, [onClose])

  const item = ({ to, icon: Icon, label }) => (
    <NavLink key={to} to={to} end={to === '/'} onClick={onClose}
      className={({ isActive }) => linkClass(isActive, 'flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors')}>
      <Icon className="w-4 h-4" />
      {label}
    </NavLink>
  )

  return (
    <div className="lg:hidden fixed inset-0 z-[60]" role="dialog" aria-modal="true" aria-label="Navigation">
      <div className="absolute inset-0 bg-slate-950/60 animate-fade-in" onClick={onClose} />
      <aside className="absolute right-0 top-0 h-full w-80 max-w-[85vw] overflow-y-auto scrollbar-thin bg-white dark:bg-slate-900 border-l border-slate-200 dark:border-slate-800 p-5 animate-slide-up">
        <div className="flex items-center justify-between mb-6">
          <Logo />
          <button onClick={onClose} className={iconButton} aria-label="Close navigation"><X className="w-5 h-5" /></button>
        </div>
        <div className="flex flex-col gap-1 mb-5">{DIRECT_NAV.map(item)}</div>
        {NAV_GROUPS.map(({ title, items }) => (
          <div key={title} className="mb-5">
            <div className="px-3 mb-1.5 text-[11px] font-semibold uppercase tracking-wider text-slate-400">{title}</div>
            <div className="flex flex-col gap-0.5">{items.map(item)}</div>
          </div>
        ))}
      </aside>
    </div>
  )
}

function Navbar() {
  const { theme, toggleTheme } = useTheme()
  const [drawerOpen, setDrawerOpen] = useState(false)
  const status = useNetworkStatus()
  const wallet = useWalletSummary()

  return (
    <>
      <nav className="glass-nav fixed top-0 left-0 right-0 z-50 h-14">
        <div className="max-w-7xl mx-auto px-4 h-full flex items-center gap-6">
          <Logo />
          <div className="hidden lg:flex items-center gap-0.5 min-w-0">
            {DIRECT_NAV.map(({ to, label }) => (
              <NavLink key={to} to={to}
                className={({ isActive }) => linkClass(isActive, 'px-3 py-1.5 rounded-lg text-sm font-medium transition-colors whitespace-nowrap')}>
                {label}
              </NavLink>
            ))}
            {NAV_GROUPS.map((g) => <GroupMenu key={g.title} group={g} />)}
          </div>
          <div className="ml-auto flex items-center gap-1.5 shrink-0">
            <NetworkPill status={status} />
            <WalletPill summary={wallet} />
            <button onClick={toggleTheme} className={iconButton} aria-label="Toggle theme">
              {theme === 'dark' ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
            </button>
            <button onClick={() => setDrawerOpen(true)} className={`lg:hidden ${iconButton}`} aria-label="Open navigation">
              <Menu className="w-5 h-5" />
            </button>
          </div>
        </div>
      </nav>
      {drawerOpen && <MobileDrawer onClose={() => setDrawerOpen(false)} />}
    </>
  )
}

function Footer() {
  return (
    <footer className="relative z-10 border-t border-slate-200 dark:border-slate-800 mt-16">
      <div className="max-w-7xl mx-auto px-4 py-12 grid gap-10 sm:grid-cols-2 lg:grid-cols-6">
        <div className="lg:col-span-2">
          <Logo />
          <p className="mt-3 text-sm text-slate-500 max-w-xs">
            A modular Rust blockchain: BFT consensus, EVM and WASM execution, and a relayer-secured Ethereum bridge.
          </p>
          <a href={GITHUB_URL} target="_blank" rel="noopener noreferrer"
            className="mt-4 inline-flex items-center gap-2 text-sm text-slate-500 hover:text-slate-900 dark:hover:text-white transition-colors">
            <Github className="w-4 h-4" /> Source on GitHub
          </a>
        </div>
        {NAV_GROUPS.map(({ title, items }) => (
          <div key={title}>
            <div className="text-xs font-semibold uppercase tracking-wider text-slate-400 mb-3">{title}</div>
            <ul className="space-y-2">
              {items.map(({ to, label }) => (
                <li key={to}><Link to={to} className="text-sm text-slate-500 hover:text-slate-900 dark:hover:text-white transition-colors">{label}</Link></li>
              ))}
            </ul>
          </div>
        ))}
      </div>
      <div className="border-t border-slate-200 dark:border-slate-800">
        <div className="max-w-7xl mx-auto px-4 py-4 text-xs text-slate-400 flex flex-wrap justify-between gap-2">
          <span>Nebula devnet · not a production network</span>
          <span className="font-mono">RPC {getApiUrl()}</span>
        </div>
      </div>
    </footer>
  )
}

function PageFallback() {
  return (
    <div className="max-w-7xl mx-auto px-4 py-16 animate-fade-in" aria-busy="true">
      <div className="mx-auto mb-6 h-10 w-72 max-w-full rounded-xl bg-slate-200 dark:bg-slate-800 animate-pulse" />
      <div className="mx-auto mb-10 h-4 w-96 max-w-full rounded-lg bg-slate-200 dark:bg-slate-800 animate-pulse" />
      <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {[0, 1, 2].map((i) => <div key={i} className="h-40 rounded-2xl glass-strong animate-pulse" />)}
      </div>
    </div>
  )
}

export default function App() {
  return (
    <div className="min-h-screen pt-14 flex flex-col">
      <Navbar />
      <main className="animate-in flex-1">
        <Suspense fallback={<PageFallback />}>
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/explorer" element={<Explorer />} />
            <Route path="/wallet" element={<WalletPage />} />
            <Route path="/history" element={<HistoryPage />} />
            <Route path="/deploy" element={<ContractDeployPage />} />
            <Route path="/contracts" element={<ContractInteractPage />} />
            <Route path="/defi" element={<DeFiPage />} />
            <Route path="/bridge" element={<BridgePage />} />
            <Route path="/wasm" element={<WasmPage />} />
            <Route path="/advanced" element={<AdvancedPage />} />
            <Route path="/nft" element={<NftPage />} />
            <Route path="/faucet" element={<FaucetPage />} />
            <Route path="/docs" element={<DocsPage />} />
            <Route path="/developer-portal" element={<DeveloperPortal />} />
            <Route path="/sdk" element={<SdkPortal />} />
            <Route path="/governance/*" element={<Governance />} />
            <Route path="/validators/onboard" element={<ValidatorOnboardPage />} />
            <Route path="/api-docs" element={<ApiDocs />} />
          </Routes>
        </Suspense>
      </main>
      <Footer />
    </div>
  )
}
