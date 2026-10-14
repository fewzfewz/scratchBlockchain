import { useEffect, useState } from 'react'
import { Activity, Inbox, Users, Coins, Blocks } from 'lucide-react'
import { getApiUrl } from '../../lib/chain.js'
import { Card, Stat, KeyValue, EmptyState, Badge } from '../../components/ui.jsx'


const fmt = (v) => v == null || isNaN(Number(v)) ? '--' : Number(v).toLocaleString()
const shorten = (v, s = 10, e = 8) => !v ? '--' : v.length <= s + e + 3 ? v : `${v.slice(0, s)}...${v.slice(-e)}`

export default function DashboardTab() {
  const [status, setStatus] = useState(null)
  const [mempool, setMempool] = useState(null)
  const [blocks, setBlocks] = useState([])
  const [loading, setLoading] = useState(true)
  const [lastUpdated, setLastUpdated] = useState(null)
  const [error, setError] = useState('')

  useEffect(() => {
    let active = true
    const fetch = async () => {
      try {
        const [sr, mr] = await Promise.all([
          window.fetch(`${getApiUrl()}/status`),
          window.fetch(`${getApiUrl()}/mempool`),
        ])
        if (!sr.ok || !mr.ok) throw new Error('Node returned unexpected response')
        const [sd, md] = await Promise.all([sr.json(), mr.json()])
        if (!active) return
        setStatus(sd); setMempool(md); setLastUpdated(new Date()); setError('')
      } catch { if (active) setError(`Unable to reach the node at ${getApiUrl()}.`) }
      finally { if (active) setLoading(false) }
    }
    const fetchBlocks = async () => {
      try {
        const res = await window.fetch(`${getApiUrl()}/block/latest`)
        if (!res.ok) throw new Error('offline')
        const { block } = await res.json()
        if (!block) return
        const latest = block.header.slot
        const from = Math.max(latest - 5, 0)
        const reqs = []
        for (let s = latest; s >= from; s--) {
          reqs.push(
            window.fetch(`${getApiUrl()}/block/${s}`)
              .then((r) => r.json())
              .then((d) => d.block)
              .catch(() => null),
          )
        }
        const found = (await Promise.all(reqs)).filter(Boolean)
        if (active && found.length) setBlocks(found)
      } catch { /* keep last known blocks */ }
    }
    fetch()
    fetchBlocks()
    const interval = setInterval(() => { fetch(); fetchBlocks() }, 3000)
    return () => { active = false; clearInterval(interval) }
  }, [])

  const txs = mempool?.transactions ?? []
  const finalized = status?.finalized_height ?? 0
  const current = status?.height ?? 0
  const gap = Math.max(current - finalized, 0)

  const latest = blocks[0]

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
        <Stat icon={Activity} label="Height" value={fmt(current)} sub="every block is final" />
        <Stat icon={Users} label="Peers" value={fmt(status?.peer_count)} sub="connected to this node" />
        <Stat icon={Inbox} label="Mempool" value={fmt(status?.mempool_size ?? txs.length)} sub="pending transactions" />
        <Stat icon={Coins} label="Base fee" value={latest ? `${fmt(latest.header.base_fee)} wei` : '—'} sub="per gas, EIP-1559" />
      </div>

      <div className="grid lg:grid-cols-2 gap-6">
        <Card title="Node" actions={<Badge tone={error ? 'red' : 'green'}>{error ? 'Offline' : 'Synced'}</Badge>}>
          <KeyValue rows={[
            ['RPC endpoint', getApiUrl(), true],
            ['Latest block', `#${fmt(current)}`],
            ['Consensus', 'BFT, >2/3 of stake per block'],
            ['Updated', lastUpdated ? lastUpdated.toLocaleTimeString() : '—'],
          ]} />
        </Card>

        <Card title="Pending transactions" description="Waiting in this node's mempool">
          {loading ? (
            <div className="space-y-2">{[0, 1, 2].map((i) => <div key={i} className="h-11 rounded-xl bg-slate-100 dark:bg-slate-800/60 animate-pulse" />)}</div>
          ) : txs.length > 0 ? (
            <ul className="divide-y divide-slate-100 dark:divide-slate-800 max-h-64 overflow-y-auto scrollbar-thin -my-2">
              {txs.slice(0, 8).map((tx, i) => (
                <li key={i} className="flex items-center justify-between gap-3 py-2.5 text-sm">
                  <span className="font-mono text-xs text-slate-500 truncate">{shorten(tx.to || tx.recipient || tx.hash)}</span>
                  <span className="text-xs text-slate-500 whitespace-nowrap">nonce {fmt(tx.nonce)} · {fmt(tx.gas_limit ?? tx.gas ?? 0)} gas</span>
                </li>
              ))}
            </ul>
          ) : (
            <EmptyState icon={Inbox} title="Mempool is empty">Send a transaction from the wallet to see it here.</EmptyState>
          )}
        </Card>
      </div>

      <Card title="Recent blocks" icon={Blocks} description="Live from the tip">
        {blocks.length > 0 ? (
          <ul className="divide-y divide-slate-100 dark:divide-slate-800 -my-2">
            {blocks.map((b) => (
              <li key={b.header.slot} className="flex items-center justify-between py-2.5 text-sm">
                <span className="font-mono font-medium text-slate-900 dark:text-white">#{fmt(b.header.slot)}</span>
                <span className="text-slate-500 tabular-nums">{b.extrinsics?.length ?? 0} txs · {fmt(b.header.gas_used)} gas</span>
              </li>
            ))}
          </ul>
        ) : (
          <EmptyState icon={Blocks}>{error ? 'Node unreachable — start the devnet to see live blocks.' : 'Fetching blocks…'}</EmptyState>
        )}
      </Card>
    </div>
  )
}
