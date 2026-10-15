import { useEffect, useState } from 'react'
import { Blocks, ArrowLeft, ChevronLeft, ChevronRight, RefreshCw } from 'lucide-react'
import { getApiUrl, weiToNbl } from '../../lib/chain.js'
import { Card, Button, KeyValue, Badge, EmptyState, Notice } from '../../components/ui.jsx'

const PAGE_SIZE = 10

const fmt = (v) => v == null || isNaN(Number(v)) ? '--' : Number(v).toLocaleString()
const toHex = (arr) => {
  if (!arr) return '--'
  const bytes = Array.isArray(arr) ? arr : []
  if (bytes.length === 0) return '0x00'
  return '0x' + bytes.map(b => (b & 0xff).toString(16).padStart(2, '0')).join('')
}
const shorten = (s, n = 16) => !s || s === '0x00' ? '--' : `${s.slice(0, n)}...${s.slice(-8)}`

export default function BlocksTab({ jumpTo }) {
  const [blocks, setBlocks] = useState([])
  const [tip, setTip] = useState(null)
  const [cursor, setCursor] = useState(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [selected, setSelected] = useState(null)
  const [loadingMore, setLoadingMore] = useState(false)
  const [jumpError, setJumpError] = useState('')

  // Open a specific block (from the explorer search).
  useEffect(() => {
    if (jumpTo == null) return
    setJumpError('')
    window.fetch(`${getApiUrl()}/block/${jumpTo}`)
      .then((r) => r.json())
      .then((d) => (d.block ? setSelected(d.block) : setJumpError(`Block #${jumpTo} not found`)))
      .catch(() => setJumpError(`Block #${jumpTo} not found`))
  }, [jumpTo])

  useEffect(() => {
    let active = true
    const fetchPage = async (from) => {
      setLoading(true)
      try {
        const res = await window.fetch(`${getApiUrl()}/block/latest`)
        if (!res.ok) throw new Error('Failed to fetch')
        const { block } = await res.json()
        if (!block) { if (active) setBlocks([]); return }
        const latest = block.header.slot
        const to = from == null ? latest : from
        const fromSlot = Math.max(to - PAGE_SIZE + 1, 0)
        const reqs = []
        for (let s = to; s >= fromSlot; s--) {
          reqs.push(
            window.fetch(`${getApiUrl()}/block/${s}`)
              .then(r => r.json())
              .then(d => d.block)
              .catch(() => null),
          )
        }
        const found = (await Promise.all(reqs)).filter(Boolean)
        if (!active) return
        if (from == null) setCursor(to)
        setBlocks(found)
        setTip(latest)
        setError('')
      } catch { if (active) setError('Unable to reach the local node.') }
      finally { if (active) setLoading(false) }
    }
    fetchPage()
    const interval = setInterval(() => { if (!selected) fetchPage() }, 5000)
    return () => { active = false; clearInterval(interval) }
  }, [selected])

  const goOlder = async () => {
    if (!cursor || cursor <= 1) return
    const to = cursor - PAGE_SIZE
    setLoadingMore(true)
    try {
      const fromSlot = Math.max(to - PAGE_SIZE + 1, 0)
      const reqs = []
      for (let s = to; s >= fromSlot; s--) {
        reqs.push(
          window.fetch(`${getApiUrl()}/block/${s}`)
            .then(r => r.json())
            .then(d => d.block)
            .catch(() => null),
        )
      }
      const found = (await Promise.all(reqs)).filter(Boolean)
      if (found.length) { setBlocks(found); setCursor(to) }
    } catch { /* ignore */ }
    finally { setLoadingMore(false) }
  }

  const goNewer = async () => {
    if (!cursor || !tip || cursor >= tip) return
    const to = Math.min(cursor + PAGE_SIZE, tip)
    const fromSlot = Math.max(to - PAGE_SIZE + 1, 0)
    setLoadingMore(true)
    try {
      const reqs = []
      for (let s = to; s >= fromSlot; s--) {
        reqs.push(
          window.fetch(`${getApiUrl()}/block/${s}`)
            .then(r => r.json())
            .then(d => d.block)
            .catch(() => null),
        )
      }
      const found = (await Promise.all(reqs)).filter(Boolean)
      if (found.length) { setBlocks(found); setCursor(to) }
    } catch { /* ignore */ }
    finally { setLoadingMore(false) }
  }

  if (selected) {
    const b = selected
    const h = b.header
    const txs = b.extrinsics || []
    return (
      <Card padded={false}>
        <div className="p-5 sm:p-6 border-b border-slate-100 dark:border-slate-800">
          <button type="button" onClick={() => setSelected(null)}
            className="inline-flex items-center gap-1.5 text-xs font-medium text-blue-600 dark:text-blue-300 hover:underline mb-3">
            <ArrowLeft className="w-3.5 h-3.5" /> All blocks
          </button>
          <div className="flex flex-wrap items-center gap-3">
            <h2 className="font-display text-2xl font-semibold text-slate-900 dark:text-white">Block #{fmt(h.slot)}</h2>
            <Badge tone="green">Final</Badge>
            <Badge>{txs.length} transaction{txs.length === 1 ? '' : 's'}</Badge>
          </div>
        </div>
        <div className="p-5 sm:p-6 grid lg:grid-cols-2 gap-x-10">
          <KeyValue rows={[
            ['Proposer', shorten(toHex(h.proposer), 12), true],
            ['Signed by', b.commit?.precommits ? `${b.commit.precommits.length} validators (round ${b.commit.round})` : '—'],
            ['Parent', shorten(toHex(h.parent_hash), 12), true],
            ['State root', shorten(toHex(h.state_root), 12), true],
          ]} />
          <KeyValue rows={[
            ['Gas used', fmt(h.gas_used)],
            ['Base fee', `${fmt(h.base_fee)} wei`],
            ['Transactions root', shorten(toHex(h.extrinsics_root), 12), true],
            ['Previous block signers', b.last_commit?.precommits ? String(b.last_commit.precommits.length) : '—'],
          ]} />
        </div>
        {txs.length > 0 && (
          <div className="border-t border-slate-100 dark:border-slate-800">
            <div className="px-5 sm:px-6 pt-4 pb-2 text-xs font-medium uppercase tracking-wider text-slate-500">Transactions</div>
            <ul className="divide-y divide-slate-100 dark:divide-slate-800">
              {txs.slice(0, 50).map((t, i) => (
                <li key={i} className="px-5 sm:px-6 py-2.5 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm">
                  <span className="font-mono text-xs text-slate-500 w-28 truncate">{shorten(toHex(t.sender), 8)}</span>
                  <span className="text-slate-400">→</span>
                  <span className="font-mono text-xs text-slate-500 w-28 truncate">{t.to ? shorten(toHex(t.to), 8) : t.payload?.length ? 'contract / action' : '—'}</span>
                  <span className="ml-auto tabular-nums text-slate-900 dark:text-white">{weiToNbl(String(t.value || 0))} NBL</span>
                </li>
              ))}
            </ul>
          </div>
        )}
      </Card>
    )
  }

  return (
    <Card
      title="Blocks"
      description={tip != null ? `Newest first · tip #${fmt(tip)}` : 'Fetching…'}
      padded={false}
      actions={
        <>
          {loadingMore && <RefreshCw className="w-4 h-4 text-slate-400 animate-spin" />}
          <Button variant="secondary" size="sm" icon={ChevronLeft} onClick={goOlder} disabled={loadingMore || !cursor || cursor <= 1}>Older</Button>
          <Button variant="secondary" size="sm" onClick={goNewer} disabled={loadingMore || !cursor || !tip || cursor >= tip}>Newer <ChevronRight className="w-4 h-4" /></Button>
        </>
      }
    >
      {jumpError && <div className="px-5 sm:px-6 pb-3"><Notice type="err">{jumpError}</Notice></div>}
      {error ? (
        <EmptyState icon={Blocks} title="Node unreachable">{error}</EmptyState>
      ) : loading && blocks.length === 0 ? (
        <div className="p-5 space-y-2">{[...Array(5)].map((_, i) => <div key={i} className="h-11 rounded-xl bg-slate-100 dark:bg-slate-800/60 animate-pulse" />)}</div>
      ) : (
        <div>
          <div className="hidden sm:grid grid-cols-[1fr_1.4fr_70px_110px_80px] gap-4 px-5 sm:px-6 py-2.5 text-[11px] uppercase tracking-wider text-slate-500 border-y border-slate-100 dark:border-slate-800">
            <span>Block</span><span>Proposer</span><span className="text-right">Txs</span><span className="text-right">Gas used</span><span className="text-right">Signers</span>
          </div>
          {blocks.map((b) => {
            const h = b.header
            return (
              <button key={h.slot} type="button" onClick={() => setSelected(b)}
                className="w-full grid grid-cols-[1fr_auto] sm:grid-cols-[1fr_1.4fr_70px_110px_80px] gap-4 px-5 sm:px-6 py-3 text-left text-sm border-b last:border-0 border-slate-100 dark:border-slate-800 hover:bg-slate-50 dark:hover:bg-slate-800/40 transition-colors">
                <span className="font-mono font-medium text-slate-900 dark:text-white">#{fmt(h.slot)}</span>
                <span className="hidden sm:block font-mono text-xs text-slate-500 truncate self-center">{shorten(toHex(h.proposer), 10)}</span>
                <span className="text-right tabular-nums">{b.extrinsics?.length ?? 0}<span className="sm:hidden text-slate-400"> tx</span></span>
                <span className="hidden sm:block text-right tabular-nums text-slate-500">{fmt(h.gas_used)}</span>
                <span className="hidden sm:block text-right tabular-nums text-slate-500">{b.commit?.precommits?.length ?? '—'}</span>
              </button>
            )
          })}
        </div>
      )}
    </Card>
  )
}
