import { useState } from 'react'
import { Compass } from 'lucide-react'
import DashboardTab from './explorer/DashboardTab.jsx'
import ValidatorsTab from './explorer/ValidatorsTab.jsx'
import StakingTab from './explorer/StakingTab.jsx'

const TABS = [
  { id: 'dashboard', label: 'Dashboard' },
  { id: 'validators', label: 'Validators' },
  { id: 'staking', label: 'Staking' },
]

export default function Explorer() {
  const [activeTab, setActiveTab] = useState('dashboard')

  return (
    <div className="max-w-5xl mx-auto px-4 py-8 animate-fade-in">
      <div className="mb-6">
        <div className="flex items-center gap-3 mb-1">
          <Compass className="w-5 h-5 text-blue-500 dark:text-blue-400" />
          <p className="text-xs uppercase tracking-widest text-blue-500 dark:text-blue-400 font-medium">Scratch Blockchain</p>
        </div>
        <h1 className="text-4xl font-bold text-slate-900 dark:text-white">Nebula Explorer</h1>
        <p className="text-slate-500 dark:text-slate-400 mt-1">A live control room for network activity, validators, and staking.</p>
      </div>

      <div className="flex gap-1 p-1 rounded-xl glass-strong mb-6">
        {TABS.map(({ id, label }) => (
          <button
            key={id}
            onClick={() => setActiveTab(id)}
            className={`flex-1 px-4 py-2.5 rounded-lg text-sm font-medium transition-all ${
              activeTab === id
                ? 'bg-blue-500/20 dark:bg-blue-600/30 text-blue-600 dark:text-blue-300 shadow-sm'
                : 'text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 hover:bg-slate-200/50 dark:hover:bg-slate-700/40'
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      {activeTab === 'dashboard' && <DashboardTab />}
      {activeTab === 'validators' && <ValidatorsTab />}
      {activeTab === 'staking' && <StakingTab />}
    </div>
  )
}
