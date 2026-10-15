import { useState, useEffect } from 'react'
import { Link, useSearchParams } from 'react-router-dom'
import { History, Search, ArrowUpRight, ArrowDownLeft, Copy, FileCode } from 'lucide-react'
import { Page, PageHeader, Card, Button, Notice, Stat, Badge, EmptyState, inputClass } from '../components/ui.jsx'
import {
  fetchTxHistory, fetchBalance, shorten, weiToNbl, walletAddress,
} from '../lib/chain.js'

export default function HistoryPage() {
  const [params] = useSearchParams()
  const [address, setAddress] = useState(params.get('address') || walletAddress() || '')
  const [txs, setTxs] = useState([])
  const [balance, setBalance] = useState(null)
  const [meta, setMeta] = useState(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  const search = async (e) => {
    e?.preventDefault()
    const addr = address.trim()
    if (!/^0x[a-fA-F0-9]{40}$/.test(addr)) {
      setError('Enter a valid 0x… address (40 hex chars)')
      return
    }
    setLoading(true)
    setError('')
    try {
      const [hist, bal] = await Promise.all([
        fetchTxHistory(addr, 50),
        fetchBalance(addr),
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

  return (
    <Page width="max-w-5xl">
      <PageHeader icon={History} eyebrow="History" title="Transaction history"
        description="Every transaction sent from or to an address, newest first." />

      <form onSubmit={search} className="flex gap-2 mb-6">
        <div className="relative flex-1">
          <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
          <input value={address} onChange={(e) => setAddress(e.target.value)} aria-label="Address"
            placeholder="0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18" className={`${inputClass} pl-10 font-mono`} />
        </div>
        <Button type="submit" loading={loading}>Search</Button>
      </form>
      <Notice type="err" className="mb-6">{error}</Notice>

      {balance && (
        <div className="grid grid-cols-2 sm:grid-cols-3 gap-3 mb-6">
          <Stat label="Balance" value={`${weiToNbl(balance.balance)} NBL`} />
          <Stat label="Nonce" value={balance.nonce} sub="transactions sent" />
          <Stat label="Shown" value={meta?.count ?? 0} sub="most recent first" />
        </div>
      )}

      <Card padded={false}>
        {txs.length === 0 ? (
          <EmptyState icon={History} title={balance ? 'No transactions yet' : 'Search an address'}>
            {balance ? 'Nothing has been sent from or to this address.' : <>Enter an address above, or open it from the <Link to="/wallet" className="text-blue-600 dark:text-blue-300 hover:underline">wallet</Link>.</>}
          </EmptyState>
        ) : (
          <ul className="divide-y divide-slate-100 dark:divide-slate-800">
            {txs.map((tx) => {
              const outgoing = tx.sender?.toLowerCase() === me
              const counterparty = outgoing ? tx.to : tx.sender
              return (
                <li key={tx.hash} className="flex items-center gap-4 px-5 sm:px-6 py-3.5">
                  <span className={`w-9 h-9 shrink-0 rounded-xl flex items-center justify-center ${tx.is_contract_creation ? 'bg-blue-500/10 text-blue-600 dark:text-blue-300' : outgoing ? 'bg-rose-500/10 text-rose-500' : 'bg-emerald-500/10 text-emerald-500'}`}>
                    {tx.is_contract_creation ? <FileCode className="w-4 h-4" /> : outgoing ? <ArrowUpRight className="w-4 h-4" /> : <ArrowDownLeft className="w-4 h-4" />}
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2 text-sm">
                      <span className="font-medium text-slate-900 dark:text-white">{tx.is_contract_creation ? 'Contract / action' : outgoing ? 'Sent' : 'Received'}</span>
                      <Badge tone={tx.status === 'confirmed' ? 'green' : tx.status === 'failed' ? 'red' : 'amber'}>{tx.status || 'pending'}</Badge>
                    </div>
                    <div className="mt-0.5 flex items-center gap-2 text-xs text-slate-500">
                      <span className="font-mono">{counterparty ? `${outgoing ? 'to' : 'from'} ${shorten(counterparty)}` : '—'}</span>
                      <span>· block #{Number(tx.block_height).toLocaleString()}</span>
                      <button type="button" onClick={() => copy(tx.hash)} className="inline-flex items-center gap-1 hover:text-slate-900 dark:hover:text-white" title="Copy transaction hash">
                        <Copy className="w-3 h-3" /> {shorten(tx.hash, 6, 4)}
                      </button>
                    </div>
                  </div>
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
