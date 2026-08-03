import { useState, useEffect } from 'react'
import { Link, useSearchParams } from 'react-router-dom'
import { History, Search, RefreshCw, ArrowUpRight, ArrowDownLeft, Copy } from 'lucide-react'
import PageShell from '../components/PageShell.jsx'
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

  return (
    <PageShell variant="default">
      <div className="max-w-4xl mx-auto px-4 py-10 animate-fade-in">
        <header className="text-center mb-8">
          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-violet-600 to-fuchsia-600 flex items-center justify-center text-white shadow-lg">
              <History className="w-6 h-6" />
            </div>
            <h1 className="text-3xl md:text-4xl font-bold text-slate-900 dark:text-white">Transaction History</h1>
          </div>
          <p className="text-slate-500 dark:text-slate-400">Live on-chain activity from the node RPC.</p>
        </header>

        <form onSubmit={search} className="glass-strong rounded-2xl p-4 mb-6 flex flex-col sm:flex-row gap-3">
          <input
            className="flex-1 px-4 py-3 rounded-xl bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600 text-sm font-mono"
            placeholder="0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18"
            value={address}
            onChange={(e) => setAddress(e.target.value)}
          />
          <button type="submit" disabled={loading}
            className="px-6 py-3 rounded-xl bg-gradient-to-r from-violet-600 to-fuchsia-600 text-white font-semibold flex items-center justify-center gap-2 disabled:opacity-50">
            {loading ? <RefreshCw className="w-4 h-4 animate-spin" /> : <Search className="w-4 h-4" />}
            Search
          </button>
        </form>

        {balance && (
          <div className="glass rounded-2xl p-4 mb-4 grid sm:grid-cols-2 gap-3 text-sm">
            <div><span className="text-slate-500">Balance:</span> <span className="font-mono font-semibold">{weiToNbl(balance.balance)} NBL</span></div>
            <div><span className="text-slate-500">Nonce:</span> <span className="font-mono font-semibold">{balance.nonce}</span></div>
          </div>
        )}

        {error && <p className="text-red-500 text-sm mb-4 text-center">{error}</p>}
        {meta && (
          <p className="text-xs text-slate-500 text-center mb-4">
            Scanned {meta.scanned} recent blocks · {meta.count} transaction{meta.count !== 1 ? 's' : ''}
          </p>
        )}

        <div className="space-y-3">
          {txs.length === 0 && !loading && !error && (
            <div className="glass rounded-2xl p-8 text-center text-slate-500">
              Enter an address and search, or open from <Link to="/wallet" className="text-violet-500 underline">Wallet</Link>.
            </div>
          )}
          {txs.map((tx) => {
            const outgoing = tx.sender?.toLowerCase() === address.toLowerCase()
            return (
              <div key={tx.hash} className="glass-strong rounded-2xl p-4 hover:border-violet-500/30 border border-transparent transition-all">
                <div className="flex flex-wrap items-start justify-between gap-2 mb-2">
                  <div className="flex items-center gap-2">
                    {outgoing ? <ArrowUpRight className="w-4 h-4 text-amber-500" /> : <ArrowDownLeft className="w-4 h-4 text-emerald-500" />}
                    <span className="text-xs font-semibold uppercase text-slate-500">{outgoing ? 'Sent' : 'Received'}</span>
                    <span className={`text-xs px-2 py-0.5 rounded-full ${tx.status === 'confirmed' ? 'bg-emerald-500/15 text-emerald-600' : tx.status === 'failed' ? 'bg-red-500/15 text-red-600' : 'bg-slate-500/15 text-slate-600'}`}>
                      {tx.status || 'pending'}
                    </span>
                  </div>
                  <span className="text-xs text-slate-400">Block #{tx.block_height}</span>
                </div>
                <div className="flex items-center gap-2 font-mono text-xs text-slate-600 dark:text-slate-300 mb-2">
                  <span>{shorten(tx.hash, 10, 8)}</span>
                  <button type="button" onClick={() => copy(tx.hash)} className="p-1 hover:text-violet-500"><Copy className="w-3 h-3" /></button>
                  <Link to={`/history?address=${tx.sender}`} className="text-violet-500 ml-auto">from {shorten(tx.sender)}</Link>
                </div>
                <div className="flex flex-wrap gap-4 text-sm">
                  <span><strong className="text-slate-500">To:</strong> {tx.is_contract_creation ? 'Contract deploy' : shorten(tx.to)}</span>
                  <span><strong className="text-slate-500">Value:</strong> {weiToNbl(tx.value)} NBL</span>
                  <span><strong className="text-slate-500">Nonce:</strong> {tx.nonce}</span>
                </div>
              </div>
            )
          })}
        </div>
      </div>
    </PageShell>
  )
}
