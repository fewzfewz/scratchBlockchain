import { Link } from 'react-router-dom'
import { Compass, Wallet, Droplets, Vote, BookOpen, FileCode, ExternalLink, ArrowRight } from 'lucide-react'

const APPS = [
  { to: '/explorer', icon: Compass, title: 'Explorer', desc: 'Live blocks, transactions, validators & staking', color: 'from-blue-500/20 to-cyan-600/10 border-blue-500/20' },
  { to: '/wallet', icon: Wallet, title: 'Wallet', desc: 'Generate Ed25519 keys, check balance, send transactions', color: 'from-emerald-500/20 to-emerald-600/10 border-emerald-500/20' },
  { to: '/faucet', icon: Droplets, title: 'Faucet', desc: 'Request test tokens for local development', color: 'from-amber-500/20 to-amber-600/10 border-amber-500/20' },
  { to: '/governance', icon: Vote, title: 'Governance', desc: 'Proposals, voting, treasury & analytics', color: 'from-violet-500/20 to-violet-600/10 border-violet-500/20' },
  { to: '/docs', icon: BookOpen, title: 'Docs', desc: 'Architecture, API reference, quick start guides', color: 'from-rose-500/20 to-rose-600/10 border-rose-500/20' },
  { to: '/sdk', icon: FileCode, title: 'SDK Portal', desc: 'JavaScript SDK, CLI tools & contract templates', color: 'from-indigo-500/20 to-indigo-600/10 border-indigo-500/20' },
  { to: '/developer-portal', icon: ExternalLink, title: 'Dev Portal', desc: 'Onboarding, starter kits & resources', color: 'from-teal-500/20 to-teal-600/10 border-teal-500/20' },
]

export default function Home() {
  return (
    <div className="relative min-h-screen">
      <div className="fixed w-[40rem] h-[40rem] rounded-full opacity-20 dark:opacity-20 opacity-10 pointer-events-none"
        style={{ top: '-12rem', left: '-12rem', background: 'radial-gradient(circle, rgba(14,165,233,0.4), transparent 70%)' }} />
      <div className="fixed w-[40rem] h-[40rem] rounded-full opacity-20 dark:opacity-20 opacity-10 pointer-events-none"
        style={{ right: '-12rem', bottom: '-12rem', background: 'radial-gradient(circle, rgba(251,146,60,0.3), transparent 70%)' }} />

      <div className="relative z-10 max-w-5xl mx-auto px-4 py-12 animate-fade-in">
        <div className="text-center mb-12">
          <p className="text-xs uppercase tracking-widest text-blue-500 dark:text-blue-400 font-medium mb-3">Scratch Blockchain</p>
          <h1 className="text-5xl md:text-7xl font-bold text-slate-900 dark:text-white" style={{ lineHeight: 0.92 }}>Nebula</h1>
          <p className="text-slate-500 dark:text-slate-400 mt-4 text-lg max-w-xl mx-auto">High-performance modular blockchain — all tools in one place.</p>
        </div>

        <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-4">
          {APPS.map(({ to, icon: Icon, title, desc, color }) => (
            <Link
              key={to}
              to={to}
              className={`group p-5 rounded-2xl bg-gradient-to-br ${color} border backdrop-blur-sm hover:scale-[1.02] hover:shadow-lg hover:shadow-blue-500/5 transition-all`}
            >
              <Icon className="w-8 h-8 mb-3 text-slate-600 dark:text-slate-200" />
              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-1">{title}</h3>
              <p className="text-sm text-slate-500 dark:text-slate-400">{desc}</p>
              <ArrowRight className="w-4 h-4 text-slate-400 dark:text-slate-500 mt-3 opacity-0 group-hover:opacity-100 transition-opacity" />
            </Link>
          ))}
        </div>
      </div>
    </div>
  )
}
