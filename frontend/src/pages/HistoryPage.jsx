import { useState, useEffect } from 'react'
import { Link, useSearchParams } from 'react-router-dom'
import { History, Search, ArrowUpRight, ArrowDownLeft, Copy, FileCode, RefreshCw, Download, X } from 'lucide-react'
import { Page, PageHeader, Card, Button, Notice, Stat, Badge, EmptyState, Tabs, KeyValue, inputClass } from '../components/ui.jsx'
import {
  fetchTxHistory, fetchBalance, fetchJson, shorten, weiToNbl, walletAddress,
} from '../lib/chain.js'

const isAddr = (v) => /^0x[a-fA-F0-9]{40}$/.test(v)
const isTxHash = (v) => /^0x[a-fA-F0-9]{64}$/.test(v)

export default function HistoryPage() {
  const [params] = useSearchParams()
  const [address, setAddress] = useState(params.get('address') || walletAddress() || '')
  const [txs, setTxs] = useState([])
  const [balance, setBalance] = useState(null)
  const [meta, setMeta] = useState(null)
  const [filter, setFilter] = useState('all')
  const [selectedTx, setSelectedTx] = useState(null)
  const [receipt, setReceipt] = useState(null)
  const [loadingReceipt, setLoadingReceipt] = useState(false)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  const inspectTx = async (tx) => {
    setSelectedTx(tx)
    setReceipt(null)
    setLoadingReceipt(true)
    try {
      const clean = String(tx.hash).replace(/^0x/, '')
      const d = await fetchJson(`/tx/${clean}`)
      setReceipt(d.receipt || d)
    } catch {
      setReceipt(null)
    } finally {
      setLoadingReceipt(false)
    }
  }

  const search = async (e) => {
    e?.preventDefault()
    const q = address.trim()
    setSelectedTx(null)
    setReceipt(null)

    if (isTxHash(q)) {
      setLoading(true)
      setError('')
      setBalance(null)
      try {
        const d = await fetchJson(`/tx/${q.replace(/^0x/, '')}`)
        const rec = d.receipt || d
        const tx = d.transaction || { hash: q, status: rec?.success === false ? 'failed' : 'confirmed', block_height: d.block_height ?? 0 }
        setTxs([tx])
        setSelectedTx(tx)
        setReceipt(rec)
        setMeta({ scanned: 1, count: 1 })
      } catch {
        setError('Transaction not found')
        setTxs([])
      } finally {
        setLoading(false)
      }
      return
    }

    if (!isAddr(q)) {
      setError('Enter a valid address (0x + 40 hex) or tx hash (0x + 64 hex)')
      return
    }
    setLoading(true)
    setError('')
    try {
      const [hist, bal] = await Promise.all([
        fetchTxHistory(q, 50),
        fetchBalance(q),
      ])
      setTxs(hist.transactions || [])
      setBalance(bal)
      setMeta({ scanned: hist.scanned_blocks, count: (hist.transactions || []).length })
    } catch {
      setError('Could not load history — is the node running?')
      setTxs([])
      setBalance(null)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    if (params.get('address') || walletAddress()) search()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const copy = (t) => navigator.clipboard?.writeText(t)
  const me = address.trim().toLowerCase()

  const filtered = txs.filter((tx) => {
    if (filter === 'all') return true
    const outgoing = tx.sender?.toLowerCase() === me
    if (filter === 'contracts') return Boolean(tx.is_contract_creation)
    if (filter === 'sent') return outgoing && !tx.is_contract_creation
    if (filter === 'received') return !outgoing && !tx.is_contract_creation
    return true
  })

  const exportCsv = () => {
    const header = 'hash,block_height,sender,to,value_wei,status,is_contract_creation\n'
    const rows = filtered.map((t) =>
      [t.hash, t.block_height, t.sender, t.to || '', t.value || 0, t.status || '', Boolean(t.is_contract_creation)].join(','),
    ).join('\n')
    const blob = new Blob([header + rows], { type: 'text/csv' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `nebula-history-${shorten(address.trim(), 6, 4)}.csv`
    a.click()
    URL.revokeObjectURL(url)
  }

  return (
    <Page width="max-w-5xl">
      <PageHeader icon={History} eyebrow="History" title="Transaction history"
        description="Search any address or transaction hash, inspect receipts, and export CSV."
        actions={
          txs.length > 0 && (
            <div className="flex gap-2">
              <Button variant="secondary" size="sm" icon={RefreshCw} loading={loading} onClick={() => search()}>Refresh</Button>
              <Button variant="secondary" size="sm" icon={Download} onClick={exportCsv}>Export CSV</Button>
            </div>
          )
        } />

      <form onSubmit={search} className="flex gap-2 mb-6">
        <div className="relative flex-1">
          <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
          <input value={address} onChange={(e) => setAddress(e.target.value)} aria-label="Address or transaction hash"
            placeholder="0x… address (40 hex) or tx hash (64 hex)" className={`${inputClass} pl-10 font-mono`} />
        </div>
        <Button type="submit" loading={loading}>Search</Button>
      </form>
      <Notice type="err" className="mb-6">{error}</Notice>

      {balance && (
        <div className="grid grid-cols-2 sm:grid-cols-3 gap-3 mb-6">
          <Stat label="Balance" value={`${weiToNbl(balance.balance)} NBL`} />
          <Stat label="Nonce" value={balance.nonce} sub="transactions sent" />
          <Stat label="Shown" value={filtered.length} sub={`${meta?.count ?? 0} total`} />
        </div>
      )}

      {txs.length > 0 && (
        <Tabs value={filter} onChange={setFilter} className="mb-4" tabs={[
          { id: 'all', label: `All (${txs.length})` },
          { id: 'sent', label: 'Sent' },
          { id: 'received', label: 'Received' },
          { id: 'contracts', label: 'Contracts' },
        ]} />
      )}

      {selectedTx && (
        <Card title={`Transaction ${shorten(selectedTx.hash, 10, 8)}`} className="mb-6"
          actions={<Button variant="ghost" size="sm" icon={X} onClick={() => setSelectedTx(null)} aria-label="Close" />}>
          {loadingReceipt ? (
            <p className="text-sm text-slate-500">Loading receipt…</p>
          ) : (
            <KeyValue rows={[
              ['Hash', selectedTx.hash, true],
              ['Block', selectedTx.block_height != null ? `#${Number(selectedTx.block_height).toLocaleString()}` : '—'],
              ['Status', receipt?.success === false ? 'Failed' : 'Confirmed'],
              ['Gas used', receipt?.gas_used != null ? Number(receipt.gas_used).toLocaleString() : '—'],
              ...(receipt?.created_address || receipt?.contract_address ? [['Created contract', receipt.created_address || receipt.contract_address, true]] : []),
              ...(receipt?.revert_reason ? [['Revert reason', receipt.revert_reason]] : []),
              ...(receipt?.logs?.length ? [['Logs', `${receipt.logs.length} event(s)`]] : []),
            ]} />
          )}
        </Card>
      )}

      <Card padded={false}>
        {filtered.length === 0 ? (
          <EmptyState icon={History} title={balance ? 'No matching transactions' : 'Search an address or tx hash'}>
            {balance ? 'Nothing matches this filter for the address.' : <>Enter an address or tx hash above, or open from the <Link to="/wallet" className="text-blue-600 dark:text-blue-300 hover:underline">wallet</Link>.</>}
          </EmptyState>
        ) : (
          <ul className="divide-y divide-slate-100 dark:divide-slate-800">
            {filtered.map((tx) => {
              const outgoing = tx.sender?.toLowerCase() === me
              const counterparty = outgoing ? tx.to : tx.sender
              return (
                <li key={tx.hash} className="flex items-center gap-4 px-5 sm:px-6 py-3.5 hover:bg-slate-50 dark:hover:bg-slate-800/40 transition-colors">
                  <span className={`w-9 h-9 shrink-0 rounded-xl flex items-center justify-center ${tx.is_contract_creation ? 'bg-blue-500/10 text-blue-600 dark:text-blue-300' : outgoing ? 'bg-rose-500/10 text-rose-500' : 'bg-emerald-500/10 text-emerald-500'}`}>
                    {tx.is_contract_creation ? <FileCode className="w-4 h-4" /> : outgoing ? <ArrowUpRight className="w-4 h-4" /> : <ArrowDownLeft className="w-4 h-4" />}
                  </span>
                  <button type="button" onClick={() => inspectTx(tx)} className="min-w-0 flex-1 text-left">
                    <div className="flex items-center gap-2 text-sm">
                      <span className="font-medium text-slate-900 dark:text-white">{tx.is_contract_creation ? 'Contract / action' : outgoing ? 'Sent' : 'Received'}</span>
                      <Badge tone={tx.status === 'confirmed' ? 'green' : tx.status === 'failed' ? 'red' : 'amber'}>{tx.status || 'pending'}</Badge>
                    </div>
                    <div className="mt-0.5 flex items-center gap-2 text-xs text-slate-500">
                      <span className="font-mono">{counterparty ? `${outgoing ? 'to' : 'from'} ${shorten(counterparty)}` : '—'}</span>
                      <span>· block #{Number(tx.block_height).toLocaleString()}</span>
                      <span className="font-mono">{shorten(tx.hash, 6, 4)}</span>
                    </div>
                  </button>
                  <button type="button" onClick={() => copy(tx.hash)} className="text-slate-400 hover:text-slate-900 dark:hover:text-white" title="Copy transaction hash">
                    <Copy className="w-3.5 h-3.5" />
                  </button>
                  <span className={`text-sm font-semibold tabular-nums ${outgoing ? 'text-slate-900 dark:text-white' : 'text-emerald-600 dark:text-emerald-400'}`}>
                    {outgoing ? '−' : '+'}{weiToNbl(String(tx.value || 0))}
                  </span>
                </li>
              )
            })}
          </ul>
        )}
      </Card>
    </Page>
  )
}
