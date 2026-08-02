import { useEffect, useState } from 'react'
import { Blocks, ArrowLeft, ChevronLeft, ChevronRight, Cpu, Fuel, ShieldCheck, Coins, Layers, Fingerprint, FileSignature, RefreshCw } from 'lucide-react'

const API_URL = 'http://localhost:8545'
const PAGE_SIZE = 10

const fmt = (v) => v == null || isNaN(Number(v)) ? '--' : Number(v).toLocaleString()
const toHex = (arr) => {
  if (!arr) return '--'
  const bytes = Array.isArray(arr) ? arr : []
  if (bytes.length === 0) return '0x00'
  return '0x' + bytes.map(b => String(b & 0xff).padStart(2, '0')).join('')
}
const shorten = (s, n = 16) => !s || s === '0x00' ? '--' : `${s.slice(0, n)}...${s.slice(-8)}`

export default function BlocksTab() {
  const [blocks, setBlocks] = useState([])
  const [tip, setTip] = useState(null)
  const [cursor, setCursor] = useState(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [selected, setSelected] = useState(null)
  const [loadingMore, setLoadingMore] = useState(false)

  useEffect(() => {
    let active = true
    const fetchPage = async (from) => {
      setLoading(true)
      try {
        const res = await window.fetch(`${API_URL}/block/latest`)
        if (!res.ok) throw new Error('Failed to fetch')
        const { block } = await res.json()
        if (!block) { if (active) setBlocks([]); return }
        const latest = block.header.slot
        const to = from == null ? latest : from
        const fromSlot = Math.max(to - PAGE_SIZE + 1, 0)
        const reqs = []
        for (let s = to; s >= fromSlot; s--) {
          reqs.push(
            window.fetch(`${API_URL}/block/${s}`)
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
          window.fetch(`${API_URL}/block/${s}`)
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
          window.fetch(`${API_URL}/block/${s}`)
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
    const rows = [
      { icon: Layers, label: 'Parent hash', value: shorten(toHex(h.parent_hash), 20), mono: true },
      { icon: Coins, label: 'Base fee', value: fmt(h.base_fee), mono: true },
      { icon: Fuel, label: 'Gas used', value: fmt(h.gas_used), mono: true },
      { icon: Fingerprint, label: 'State root', value: shorten(toHex(h.state_root), 20), mono: true },
      { icon: Layers, label: 'Extrinsics root', value: shorten(toHex(h.extrinsics_root), 20), mono: true },
      { icon: Cpu, label: 'Epoch', value: fmt(h.epoch), mono: true },
      { icon: ShieldCheck, label: 'Validator set id', value: fmt(h.validator_set_id), mono: true },
      { icon: FileSignature, label: 'Signature', value: shorten(toHex(h.signature), 20), mono: true },
    ]
    return (
      <div className="rounded-2xl glass-strong overflow-hidden">
        <div className="bg-gradient-to-r from-blue-600/10 via-cyan-600/10 to-violet-600/10 border-b border-slate-200/50 dark:border-slate-700/40 px-5 py-4">
          <button onClick={() => setSelected(null)}
            className="inline-flex items-center gap-1.5 text-xs font-medium text-blue-600 dark:text-cyan-400 hover:underline mb-3">
            <ArrowLeft className="w-3.5 h-3.5" /> All blocks
          </button>
          <div className="flex items-center gap-3">
            <div className="w-11 h-11 rounded-xl bg-gradient-to-br from-blue-500 to-cyan-600 flex items-center justify-center text-white shadow-lg">
              <Blocks className="w-5 h-5" />
            </div>
            <div>
              <h3 className="text-lg font-bold text-slate-900 dark:text-white">Block #{fmt(h.slot)}</h3>
              <p className="text-xs text-slate-400 dark:text-slate-500 mt-0.5">{b.extrinsics?.length ?? 0} transactions</p>
            </div>
          </div>
        </div>
        <div className="p-5 grid sm:grid-cols-2 gap-2">
          {rows.map(({ icon: Icon, label, value, mono }) => (
            <div key={label} className="flex justify-between items-center gap-3 p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
              <span className="flex items-center gap-2 text-sm text-slate-500 dark:text-slate-400">
                <Icon className="w-3.5 h-3.5 shrink-0" /> {label}
              </span>
              <strong className={`text-sm text-slate-900 dark:text-white tabular-nums truncate ${mono ? 'font-mono' : ''}`}>{value}</strong>
            </div>
          ))}
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between px-1">
        <div>
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300">Blocks</h3>
          <p className="text-xs text-slate-400 dark:text-slate-500 mt-0.5">{tip != null ? `Live from tip #{fmt(tip)}` : 'Fetching…'}</p>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={goOlder} disabled={loadingMore || !cursor || cursor <= 1}
            className="inline-flex items-center gap-1 px-3 py-1.5 rounded-xl glass text-xs font-medium text-slate-600 dark:text-slate-300 hover:-translate-y-0.5 transition-all disabled:opacity-40 disabled:hover:translate-y-0">
            <ChevronLeft className="w-3.5 h-3.5" /> Older
          </button>
          <button onClick={goNewer} disabled={loadingMore || !cursor || !tip || cursor >= tip}
            className="inline-flex items-center gap-1 px-3 py-1.5 rounded-xl glass text-xs font-medium text-slate-600 dark:text-slate-300 hover:-translate-y-0.5 transition-all disabled:opacity-40 disabled:hover:translate-y-0">
            Newer <ChevronRight className="w-3.5 h-3.5" />
          </button>
          {loadingMore && <RefreshCw className="w-3.5 h-3.5 text-slate-400 animate-spin" />}
        </div>
      </div>

      {error ? (
        <div className="p-8 rounded-2xl glass-strong text-center">
          <p className="text-sm text-red-400">{error}</p>
        </div>
      ) : loading && blocks.length === 0 ? (
        <div className="space-y-2">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="h-16 rounded-2xl glass-strong animate-pulse" />
          ))}
        </div>
      ) : (
        <div className="space-y-2">
          {blocks.map((b) => {
            const h = b.header
            return (
              <button key={h.slot} onClick={() => setSelected(b)}
                className="w-full text-left p-3 rounded-2xl glass hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all">
                <div className="flex items-center gap-3 flex-wrap">
                  <div className="w-9 h-9 shrink-0 rounded-xl bg-gradient-to-br from-blue-500/20 to-cyan-600/20 border border-blue-500/20 flex items-center justify-center text-blue-500 dark:text-cyan-400">
                    <Blocks className="w-4 h-4" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="font-mono text-sm font-bold text-slate-900 dark:text-white">#{fmt(h.slot)}</div>
                    <div className="text-xs text-slate-400 dark:text-slate-500 truncate">parent {shorten(toHex(h.parent_hash))}</div>
                  </div>
                  <div className="hidden sm:flex items-center gap-4 text-xs text-slate-500 dark:text-slate-400">
                    <span className="flex items-center gap-1"><Coins className="w-3 h-3" /> {fmt(h.base_fee)}</span>
                    <span className="flex items-center gap-1"><Fuel className="w-3 h-3" /> {fmt(h.gas_used)}</span>
                    <span className="flex items-center gap-1"><Layers className="w-3 h-3" /> {b.extrinsics?.length ?? 0} tx</span>
                  </div>
                </div>
              </button>
            )
          })}
        </div>
      )}
    </div>
  )
}
