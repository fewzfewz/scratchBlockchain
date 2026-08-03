import { useState, lazy, Suspense } from 'react'
import { Routes, Route, NavLink } from 'react-router-dom'
import {
  LayoutDashboard, Wallet, Droplets, Compass, BookOpen,
  FileCode, Vote, ExternalLink, Github, Sun, Moon, Menu, X, FileJson, Rocket, Shield,
  History, Layers, GitBranch,
} from 'lucide-react'
import { useTheme } from './ThemeContext.jsx'
import Home from './pages/Home.jsx'
import Explorer from './pages/Explorer.jsx'
import WalletPage from './pages/WalletPage.jsx'
import FaucetPage from './pages/FaucetPage.jsx'
import DocsPage from './pages/DocsPage.jsx'
import DeveloperPortal from './pages/DeveloperPortal.jsx'
import SdkPortal from './pages/SdkPortal.jsx'
import Governance from './pages/Governance.jsx'
import ContractDeployPage from './pages/ContractDeployPage.jsx'
import ValidatorOnboardPage from './pages/ValidatorOnboardPage.jsx'
import HistoryPage from './pages/HistoryPage.jsx'
import DeFiPage from './pages/DeFiPage.jsx'
import ContractInteractPage from './pages/ContractInteractPage.jsx'
import BridgePage from './pages/BridgePage.jsx'
import NftPage from './pages/NftPage.jsx'

const ApiDocs = lazy(() => import('./pages/ApiDocs.jsx'))

const NAV_ITEMS = [
  { to: '/', icon: LayoutDashboard, label: 'Home' },
  { to: '/explorer', icon: Compass, label: 'Explorer' },
  { to: '/wallet', icon: Wallet, label: 'Wallet' },
  { to: '/history', icon: History, label: 'History' },
  { to: '/deploy', icon: Rocket, label: 'Deploy' },
  { to: '/contracts', icon: FileCode, label: 'Contracts' },
  { to: '/defi', icon: Layers, label: 'DeFi' },
  { to: '/bridge', icon: GitBranch, label: 'Bridge' },
  { to: '/faucet', icon: Droplets, label: 'Faucet' },
  { to: '/governance', icon: Vote, label: 'Governance' },
  { to: '/validators/onboard', icon: Shield, label: 'Validators' },
  { to: '/docs', icon: BookOpen, label: 'Docs' },
  { to: '/api-docs', icon: FileJson, label: 'API' },
  { to: '/sdk', icon: FileCode, label: 'SDK' },
  { to: '/developer-portal', icon: ExternalLink, label: 'Dev Portal' },
]

function Navbar() {
  const { theme, toggleTheme } = useTheme()
  const [sidebarOpen, setSidebarOpen] = useState(false)

  return (
    <>
      <nav className="glass-nav fixed top-0 left-0 right-0 z-50 h-14 transition-all duration-300">
        <div className="max-w-7xl mx-auto px-4 h-full flex items-center justify-between">
          <NavLink to="/" className="flex items-center gap-2 font-bold">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-blue-500 to-cyan-600 flex items-center justify-center text-xs text-white shadow-lg shadow-blue-500/25">
              N
            </div>
            <span className="text-slate-900 dark:text-white">Nebula</span>
          </NavLink>

          <div className="hidden md:flex items-center gap-0.5">
            {NAV_ITEMS.map(({ to, icon: Icon, label }) => (
              <NavLink
                key={to}
                to={to}
                end={to === '/'}
                className={({ isActive }) =>
                  `flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all duration-200 nav-glow ${
                    isActive
                      ? 'bg-blue-500/15 text-blue-600 dark:text-blue-400'
                      : 'text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200 hover:bg-slate-200/50 dark:hover:bg-slate-700/40'
                  }`
                }
              >
                <Icon className="w-3.5 h-3.5" />
                {label}
              </NavLink>
            ))}
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={toggleTheme}
              className="p-2 rounded-lg text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200 hover:bg-slate-200/50 dark:hover:bg-slate-700/40 transition-all"
              aria-label="Toggle theme"
            >
              {theme === 'dark' ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
            </button>
            <a href="https://github.com/fewzfewz/scratchBlockchain" target="_blank" rel="noopener noreferrer"
              className="hidden sm:flex p-2 rounded-lg text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200 hover:bg-slate-200/50 dark:hover:bg-slate-700/40 transition-all">
              <Github className="w-4 h-4" />
            </a>
            <button onClick={() => setSidebarOpen(true)}
              className="md:hidden p-2 rounded-lg text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200 hover:bg-slate-200/50 dark:hover:bg-slate-700/40 transition-all">
              <Menu className="w-5 h-5" />
            </button>
          </div>
        </div>
      </nav>

      {sidebarOpen && (
        <div className="sidebar-mobile md:hidden">
          <div className="flex items-center justify-between mb-8">
            <span className="text-lg font-bold text-slate-900 dark:text-white">Navigation</span>
            <button onClick={() => setSidebarOpen(false)}
              className="p-2 rounded-lg text-slate-500 dark:text-slate-400 hover:bg-slate-200/50 dark:hover:bg-slate-700/40 transition-all">
              <X className="w-5 h-5" />
            </button>
          </div>
          <div className="flex flex-col gap-1">
            {NAV_ITEMS.map(({ to, icon: Icon, label }) => (
              <NavLink
                key={to}
                to={to}
                end={to === '/'}
                onClick={() => setSidebarOpen(false)}
                className={({ isActive }) =>
                  `flex items-center gap-3 px-4 py-3 rounded-xl text-sm font-medium transition-all ${
                    isActive
                      ? 'bg-blue-500/15 text-blue-600 dark:text-blue-400'
                      : 'text-slate-600 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200 hover:bg-slate-200/50 dark:hover:bg-slate-700/40'
                  }`
                }
              >
                <Icon className="w-4 h-4" />
                {label}
              </NavLink>
            ))}
          </div>
          <div className="mt-8 pt-6 border-t border-slate-200 dark:border-slate-700/50">
            <a href="https://github.com/fewzfewz/scratchBlockchain" target="_blank" rel="noopener noreferrer"
              className="flex items-center gap-3 px-4 py-3 rounded-xl text-sm text-slate-600 dark:text-slate-400 hover:bg-slate-200/50 dark:hover:bg-slate-700/40 transition-all">
              <Github className="w-4 h-4" />
              GitHub
              <ExternalLink className="w-3 h-3 ml-auto" />
            </a>
          </div>
        </div>
      )}
    </>
  )
}

export default function App() {
  return (
    <div className="min-h-screen pt-14">
      <Navbar />
      <main className="animate-in">
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/explorer" element={<Explorer />} />
          <Route path="/wallet" element={<WalletPage />} />
          <Route path="/history" element={<HistoryPage />} />
          <Route path="/deploy" element={<ContractDeployPage />} />
          <Route path="/contracts" element={<ContractInteractPage />} />
          <Route path="/defi" element={<DeFiPage />} />
          <Route path="/bridge" element={<BridgePage />} />
          <Route path="/nft" element={<NftPage />} />
          <Route path="/faucet" element={<FaucetPage />} />
          <Route path="/docs" element={<DocsPage />} />
          <Route path="/developer-portal" element={<DeveloperPortal />} />
          <Route path="/sdk" element={<SdkPortal />} />
          <Route path="/governance/*" element={<Governance />} />
          <Route path="/validators/onboard" element={<ValidatorOnboardPage />} />
          <Route path="/api-docs" element={
            <Suspense fallback={
              <div className="relative min-h-[70vh] overflow-hidden animate-fade-in">
                <div className="absolute inset-0 bg-grid pointer-events-none" />
                <div className="relative z-10 max-w-7xl mx-auto px-4 py-10">
                  <div className="text-center mb-8">
                    <div className="mx-auto mb-5 h-8 w-56 rounded-full glass animate-pulse" />
                    <div className="flex items-center justify-center gap-3 mb-2">
                      <div className="w-12 h-12 rounded-2xl bg-slate-200 dark:bg-slate-700/60 animate-pulse" />
                      <div className="h-12 w-64 rounded-xl bg-slate-200 dark:bg-slate-700/60 animate-pulse" />
                    </div>
                    <div className="mx-auto h-4 w-96 max-w-full rounded-lg bg-slate-200 dark:bg-slate-700/60 animate-pulse" />
                  </div>
                  <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-8">
                    {[0, 1, 2, 3].map(i => <div key={i} className="h-24 rounded-2xl glass-strong animate-pulse" />)}
                  </div>
                  <div className="h-96 rounded-2xl glass-strong animate-pulse" />
                </div>
              </div>
            }>
              <ApiDocs />
            </Suspense>
          } />
        </Routes>
      </main>
    </div>
  )
}
