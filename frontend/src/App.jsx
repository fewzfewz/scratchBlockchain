import { useState, useEffect, useRef, lazy, Suspense } from 'react'
import { Routes, Route, NavLink, useLocation } from 'react-router-dom'
import {
  LayoutDashboard, Wallet, Droplets, Compass, BookOpen, ChevronDown,
  FileCode, Vote, ExternalLink, Github, Sun, Moon, Menu, X, FileJson, Rocket, Shield,
  History, Layers, GitBranch, Cpu, Zap, Image, Package,
} from 'lucide-react'
import { useTheme } from './ThemeContext.jsx'
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

// Shown directly in the desktop bar.
const PRIMARY_NAV = [
  { to: '/', icon: LayoutDashboard, label: 'Home' },
  { to: '/explorer', icon: Compass, label: 'Explorer' },
  { to: '/wallet', icon: Wallet, label: 'Wallet' },
  { to: '/deploy', icon: Rocket, label: 'Deploy' },
  { to: '/defi', icon: Layers, label: 'DeFi' },
  { to: '/bridge', icon: GitBranch, label: 'Bridge' },
  { to: '/governance', icon: Vote, label: 'Governance' },
]

// Behind "More" on desktop; listed in full in the mobile drawer.
const MORE_NAV = [
  {
    title: 'Build',
    items: [
      { to: '/contracts', icon: FileCode, label: 'Contracts' },
      { to: '/nft', icon: Image, label: 'NFT' },
      { to: '/wasm', icon: Cpu, label: 'WASM' },
      { to: '/advanced', icon: Zap, label: 'Advanced' },
    ],
  },
  {
    title: 'Network',
    items: [
      { to: '/history', icon: History, label: 'History' },
      { to: '/faucet', icon: Droplets, label: 'Faucet' },
      { to: '/validators/onboard', icon: Shield, label: 'Validators' },
    ],
  },
  {
    title: 'Developers',
    items: [
      { to: '/docs', icon: BookOpen, label: 'Docs' },
      { to: '/api-docs', icon: FileJson, label: 'API' },
      { to: '/sdk', icon: Package, label: 'SDK' },
      { to: '/developer-portal', icon: ExternalLink, label: 'Dev Portal' },
    ],
  },
]

const MORE_PATHS = MORE_NAV.flatMap((g) => g.items.map((i) => i.to))

const iconButton =
  'p-2 rounded-lg text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200 hover:bg-slate-200/50 dark:hover:bg-slate-700/40 transition-all'

const linkClass = (isActive, base) =>
  `${base} ${
    isActive
      ? 'bg-blue-500/15 text-blue-600 dark:text-blue-400'
      : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100 hover:bg-slate-200/50 dark:hover:bg-slate-700/40'
  }`

function MoreMenu() {
  const [open, setOpen] = useState(false)
  const ref = useRef(null)
  const { pathname } = useLocation()
  const activeInMore = MORE_PATHS.some((p) => pathname.startsWith(p))

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
        className={linkClass(activeInMore, 'flex items-center gap-1 px-3 py-1.5 rounded-lg text-sm font-medium transition-colors')}
      >
        More
        <ChevronDown className={`w-3.5 h-3.5 transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>

      {open && (
        <div
          role="menu"
          className="absolute right-0 mt-2 w-[34rem] p-4 rounded-2xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700/60 shadow-2xl grid grid-cols-3 gap-4 animate-fade-in"
        >
          {MORE_NAV.map(({ title, items }) => (
            <div key={title}>
              <div className="px-2 mb-2 text-[11px] font-semibold uppercase tracking-wider text-slate-400">{title}</div>
              <div className="flex flex-col gap-0.5">
                {items.map(({ to, icon: Icon, label }) => (
                  <NavLink
                    key={to}
                    to={to}
                    role="menuitem"
                    className={({ isActive }) => linkClass(isActive, 'flex items-center gap-2 px-2 py-1.5 rounded-lg text-sm transition-colors')}
                  >
                    <Icon className="w-4 h-4 shrink-0" />
                    {label}
                  </NavLink>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
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
    <NavLink
      key={to}
      to={to}
      end={to === '/'}
      onClick={onClose}
      className={({ isActive }) => linkClass(isActive, 'flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm font-medium transition-colors')}
    >
      <Icon className="w-4 h-4" />
      {label}
    </NavLink>
  )

  return (
    <div className="lg:hidden fixed inset-0 z-[60]" role="dialog" aria-modal="true" aria-label="Navigation">
      <div className="absolute inset-0 bg-slate-950/50 animate-fade-in" onClick={onClose} />
      <aside className="absolute right-0 top-0 h-full w-80 max-w-[85vw] overflow-y-auto scrollbar-thin bg-white dark:bg-slate-900 border-l border-slate-200 dark:border-slate-700/60 p-5 animate-slide-up">
        <div className="flex items-center justify-between mb-6">
          <span className="text-lg font-bold text-slate-900 dark:text-white">Navigation</span>
          <button onClick={onClose} className={iconButton} aria-label="Close navigation">
            <X className="w-5 h-5" />
          </button>
        </div>
        <div className="flex flex-col gap-1 mb-5">{PRIMARY_NAV.map(item)}</div>
        {MORE_NAV.map(({ title, items }) => (
          <div key={title} className="mb-5">
            <div className="px-3 mb-1.5 text-[11px] font-semibold uppercase tracking-wider text-slate-400">{title}</div>
            <div className="flex flex-col gap-1">{items.map(item)}</div>
          </div>
        ))}
        <div className="pt-4 border-t border-slate-200 dark:border-slate-700/50">
          <a href={GITHUB_URL} target="_blank" rel="noopener noreferrer"
            className="flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm text-slate-600 dark:text-slate-400 hover:bg-slate-200/50 dark:hover:bg-slate-700/40 transition-colors">
            <Github className="w-4 h-4" />
            GitHub
            <ExternalLink className="w-3 h-3 ml-auto" />
          </a>
        </div>
      </aside>
    </div>
  )
}

function Navbar() {
  const { theme, toggleTheme } = useTheme()
  const [drawerOpen, setDrawerOpen] = useState(false)

  return (
    <>
      <nav className="glass-nav fixed top-0 left-0 right-0 z-50 h-14">
        <div className="max-w-7xl mx-auto px-4 h-full flex items-center justify-between gap-4">
          <NavLink to="/" className="flex items-center gap-2 font-bold shrink-0">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-blue-500 to-cyan-600 flex items-center justify-center text-xs text-white shadow-lg shadow-blue-500/25">
              N
            </div>
            <span className="text-slate-900 dark:text-white">Nebula</span>
          </NavLink>

          <div className="hidden lg:flex items-center gap-0.5 min-w-0">
            {PRIMARY_NAV.map(({ to, label }) => (
              <NavLink
                key={to}
                to={to}
                end={to === '/'}
                className={({ isActive }) => linkClass(isActive, 'px-3 py-1.5 rounded-lg text-sm font-medium transition-colors whitespace-nowrap')}
              >
                {label}
              </NavLink>
            ))}
            <MoreMenu />
          </div>

          <div className="flex items-center gap-1 shrink-0">
            <button onClick={toggleTheme} className={iconButton} aria-label="Toggle theme">
              {theme === 'dark' ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
            </button>
            <a href={GITHUB_URL} target="_blank" rel="noopener noreferrer" className={`hidden sm:flex ${iconButton}`} aria-label="GitHub">
              <Github className="w-4 h-4" />
            </a>
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

function PageFallback() {
  return (
    <div className="max-w-7xl mx-auto px-4 py-16 animate-fade-in" aria-busy="true">
      <div className="mx-auto mb-6 h-10 w-72 max-w-full rounded-xl bg-slate-200 dark:bg-slate-700/60 animate-pulse" />
      <div className="mx-auto mb-10 h-4 w-96 max-w-full rounded-lg bg-slate-200 dark:bg-slate-700/60 animate-pulse" />
      <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {[0, 1, 2].map((i) => <div key={i} className="h-40 rounded-2xl glass-strong animate-pulse" />)}
      </div>
    </div>
  )
}

export default function App() {
  return (
    <div className="min-h-screen pt-14">
      <Navbar />
      <main className="animate-in">
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
    </div>
  )
}
