import { useEffect, useState } from 'react'
import { useSearchParams } from 'react-router-dom'
import { Compass, Search } from 'lucide-react'
import DashboardTab from './explorer/DashboardTab.jsx'
import ValidatorsTab from './explorer/ValidatorsTab.jsx'
import StakingTab from './explorer/StakingTab.jsx'
import BlocksTab from './explorer/BlocksTab.jsx'
import AddressTab from './explorer/AddressTab.jsx'
import { getApiUrl } from '../lib/chain.js'
import { Page, PageHeader, Tabs, LiveBadge, Notice, inputClass } from '../components/ui.jsx'

const TABS = [
  { id: 'dashboard', label: 'Overview' },
  { id: 'blocks', label: 'Blocks' },
  { id: 'address', label: 'Address' },
  { id: 'validators', label: 'Validators' },
  { id: 'staking', label: 'Staking' },
]

const fmt = (v) => (v == null ? '—' : Number(v).toLocaleString())

export default function Explorer() {
  const [params, setParams] = useSearchParams()
  const [activeTab, setActiveTab] = useState(params.get('tab') || 'dashboard')
  const [online, setOnline] = useState(undefined)
  const [tip, setTip] = useState(null)
  const [query, setQuery] = useState('')
  const [jumpBlock, setJumpBlock] = useState(null)
  const [jumpAddress, setJumpAddress] = useState(params.get('address') || '')
  const [searchError, setSearchError] = useState('')

  useEffect(() => {
    let active = true
    const poll = async () => {
      try {
        const res = await window.fetch(`${getApiUrl()}/status`)
        const d = await res.json()
        if (active) { setTip(d); setOnline(true) }
      } catch { if (active) setOnline(false) }
    }
    poll()
    const t = setInterval(poll, 3000)
    return () => { active = false; clearInterval(t) }
  }, [])

  const selectTab = (id) => {
    setActiveTab(id)
    setParams((p) => { p.set('tab', id); return p }, { replace: true })
  }

  // One search box: a block number or an address.
  const search = (e) => {
    e.preventDefault()
    const q = query.trim().replace(/^#/, '')
    setSearchError('')
    if (/^\d+$/.test(q)) {
      setJumpBlock(Number(q))
      selectTab('blocks')
    } else if (/^0x[a-fA-F0-9]{40}$/.test(q)) {
      setJumpAddress(q)
      selectTab('address')
    } else if (q) {
      setSearchError('Search by block number (e.g. 1200) or address (0x + 40 hex characters)')
    }
  }

  return (
    <Page>
      <PageHeader
        icon={Compass}
        eyebrow="Explorer"
        title="Chain explorer"
        description="Blocks, accounts, validators and staking — read live from your node."
        actions={<LiveBadge online={online} label={`#${fmt(tip?.height)} · ${tip?.peer_count ?? 0} peers`} />}
      />

      <form onSubmit={search} className="relative mb-6">
        <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
        <input value={query} onChange={(e) => setQuery(e.target.value)} aria-label="Search the chain"
          placeholder="Search by block number or address" className={`${inputClass} pl-10 h-12`} />
      </form>
      <Notice type="err" className="-mt-3 mb-6">{searchError}</Notice>

      <Tabs tabs={TABS} value={activeTab} onChange={selectTab} className="mb-6" />

      <div key={activeTab} className="animate-in">
        {activeTab === 'dashboard' && <DashboardTab />}
        {activeTab === 'blocks' && <BlocksTab jumpTo={jumpBlock} />}
        {activeTab === 'address' && <AddressTab initialAddress={jumpAddress} />}
        {activeTab === 'validators' && <ValidatorsTab />}
        {activeTab === 'staking' && <StakingTab />}
      </div>
    </Page>
  )
}
