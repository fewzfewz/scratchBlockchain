import { useState, useEffect, useCallback } from 'react'
import { Link } from 'react-router-dom'
import { Fingerprint, Search, ArrowRight } from 'lucide-react'
import { fetchBalance, fetchTxHistory, weiToNbl } from '../../lib/chain.js'
import { Card, Button, Notice, Stat, inputClass } from '../../components/ui.jsx'

const isAddress = (a) => /^0x[a-fA-F0-9]{40}$/.test(a)

export default function AddressTab({ initialAddress = '' }) {
  const [address, setAddress] = useState(initialAddress)
  const [result, setResult] = useState(null)
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  const run = useCallback(async (raw) => {
    const addr = raw.trim()
    if (!isAddress(addr)) { setError('Enter a 20-byte address: 0x followed by 40 hex characters'); return }
    setError('')
    setLoading(true)
    try {
      const [bal, hist] = await Promise.all([fetchBalance(addr), fetchTxHistory(addr, 50)])
      setResult({ address: addr, balance: bal.balance, nonce: bal.nonce, txs: (hist.transactions || []).length })
    } catch {
      setError('Lookup failed — is the node running?')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    if (initialAddress) { setAddress(initialAddress); run(initialAddress) }
  }, [initialAddress, run])

  return (
    <Card title="Address lookup" icon={Fingerprint} description="Balance, nonce and recent activity for any account or contract.">
      <form onSubmit={(e) => { e.preventDefault(); run(address) }} className="flex gap-2">
        <input value={address} onChange={(e) => setAddress(e.target.value)} placeholder="0x…" aria-label="Address" className={`${inputClass} font-mono`} />
        <Button type="submit" icon={Search} loading={loading}>Search</Button>
      </form>
      <Notice type="err" className="mt-4">{error}</Notice>
      {result && (
        <div className="mt-6">
          <div className="grid sm:grid-cols-3 gap-3">
            <Stat label="Balance" value={`${weiToNbl(result.balance)} NBL`} />
            <Stat label="Nonce" value={result.nonce} sub="transactions sent" />
            <Stat label="Recent transactions" value={result.txs} />
          </div>
          <Link to={`/history?address=${encodeURIComponent(result.address)}`}
            className="mt-4 inline-flex items-center gap-1.5 text-sm font-medium text-blue-600 dark:text-blue-300 hover:underline">
            Full transaction history <ArrowRight className="w-3.5 h-3.5" />
          </Link>
        </div>
      )}
    </Card>
  )
}
