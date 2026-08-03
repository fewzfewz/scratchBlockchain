import { useState } from 'react'
import { Link } from 'react-router-dom'
import { Fingerprint, Search, ExternalLink } from 'lucide-react'
import { fetchBalance, fetchTxHistory, shorten, weiToNbl } from '../../lib/chain.js'

export default function AddressTab() {
  const [address, setAddress] = useState('')
  const [balance, setBalance] = useState(null)
  const [nonce, setNonce] = useState(null)
  const [txCount, setTxCount] = useState(null)
  const [error, setError] = useState('')

  const lookup = async (e) => {
    e.preventDefault()
    const addr = address.trim()
    if (!/^0x[a-fA-F0-9]{40}$/.test(addr)) {
      setError('Invalid address')
      return
    }
    setError('')
    try {
      const [bal, hist] = await Promise.all([
        fetchBalance(addr),
        fetchTxHistory(addr, 50),
      ])
      setBalance(bal.balance)
      setNonce(bal.nonce)
      setTxCount((hist.transactions || []).length)
    } catch {
      setError('Lookup failed')
    }
  }

  return (
    <div className="glass-strong rounded-2xl p-6 animate-in">
      <h2 className="font-semibold mb-4 flex items-center gap-2">
        <Fingerprint className="w-5 h-5 text-violet-500" /> Address lookup
      </h2>
      <form onSubmit={lookup} className="flex gap-2 mb-6">
        <input value={address} onChange={(e) => setAddress(e.target.value)} placeholder="0x…"
          className="flex-1 px-4 py-2.5 rounded-xl font-mono text-sm bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600" />
        <button type="submit" className="px-4 py-2.5 rounded-xl bg-violet-600 text-white font-semibold flex items-center gap-1">
          <Search className="w-4 h-4" /> Lookup
        </button>
      </form>
      {error && <p className="text-red-500 text-sm mb-4">{error}</p>}
      {balance != null && (
        <div className="grid sm:grid-cols-3 gap-3 mb-4">
          <div className="rounded-xl bg-slate-100 dark:bg-slate-900/40 p-4">
            <p className="text-xs text-slate-500">Balance</p>
            <p className="font-mono font-bold">{weiToNbl(balance)}</p>
          </div>
          <div className="rounded-xl bg-slate-100 dark:bg-slate-900/40 p-4">
            <p className="text-xs text-slate-500">Nonce</p>
            <p className="font-mono font-bold">{nonce}</p>
          </div>
          <div className="rounded-xl bg-slate-100 dark:bg-slate-900/40 p-4">
            <p className="text-xs text-slate-500">Transactions</p>
            <p className="font-mono font-bold">{txCount ?? '—'}</p>
          </div>
        </div>
      )}
      {address && /^0x[a-fA-F0-9]{40}$/.test(address.trim()) && (
        <Link to={`/history?address=${encodeURIComponent(address.trim())}`}
          className="inline-flex items-center gap-2 text-sm text-violet-600 dark:text-violet-400 font-semibold">
          View full transaction history <ExternalLink className="w-3.5 h-3.5" />
        </Link>
      )}
    </div>
  )
}
