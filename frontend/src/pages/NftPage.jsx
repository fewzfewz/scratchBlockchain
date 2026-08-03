import { useState, useEffect, useCallback } from 'react'
import { Link } from 'react-router-dom'
import { Image, Plus, Sparkles, RefreshCw, CheckCircle, AlertCircle } from 'lucide-react'
import PageShell from '../components/PageShell.jsx'
import {
  encodeMint, loadNftsFromChain, scanDeployedContracts,
  loadWalletKeyPair, signAndSubmit, waitForReceipt, walletAddress, shorten,
  fetchJson,
} from '../lib/chain.js'

export default function NftPage() {
  const [contracts, setContracts] = useState([])
  const [selected, setSelected] = useState('')
  const [tokenId, setTokenId] = useState('1')
  const [gallery, setGallery] = useState([])
  const [mintTo, setMintTo] = useState('')
  const [status, setStatus] = useState('')
  const [statusType, setStatusType] = useState('')
  const [submitting, setSubmitting] = useState(false)

  const addr = walletAddress()

  const refresh = useCallback(async () => {
    const list = await scanDeployedContracts()
    const nfts = list.filter((c) => c.type === 'ERC721' || c.name?.includes('721') || c.name?.includes('NFT'))
    setContracts(nfts)
    const pick = nfts[0]?.address || list.find((c) => c.type === 'ERC721')?.address || ''
    setSelected((s) => s || pick)
    if (pick && addr) {
      const items = await loadNftsFromChain(pick, addr)
      setGallery(items)
    } else {
      setGallery([])
    }
  }, [addr])

  useEffect(() => {
    setMintTo(addr)
    refresh()
  }, [addr, refresh])

  const mint = async () => {
    if (!loadWalletKeyPair()) {
      setStatus('Create a wallet on /wallet first')
      setStatusType('err')
      return
    }
    if (!selected) {
      setStatus('Deploy ERC721 at /deploy first')
      setStatusType('err')
      return
    }
    setSubmitting(true)
    setStatus('Minting on-chain…')
    try {
      const to = mintTo.trim() || addr
      const data = encodeMint(to, tokenId)
      const est = await fetchJson('/estimate_gas', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ from: addr, to: selected, data, value: '0' }),
      })
      const { hash } = await signAndSubmit({
        from: addr,
        to: selected,
        valueWei: 0n,
        payload: data.replace(/^0x/, ''),
        gasLimit: est.estimated_gas || 120000,
      })
      await waitForReceipt(hash)
      setStatus(`Minted token #${tokenId} — tx ${shorten(hash, 10, 8)}`)
      setStatusType('ok')
      refresh()
    } catch (e) {
      setStatus(e.message || 'Mint failed')
      setStatusType('err')
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <PageShell variant="default">
      <div className="max-w-4xl mx-auto px-4 py-10 animate-fade-in">
        <header className="text-center mb-8">
          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-pink-600 to-rose-600 flex items-center justify-center text-white shadow-lg">
              <Image className="w-6 h-6" />
            </div>
            <h1 className="text-3xl font-bold text-slate-900 dark:text-white">NFT Marketplace</h1>
          </div>
          <p className="text-slate-500 dark:text-slate-400">Mint ERC721 tokens and browse on-chain ownership.</p>
        </header>

        <div className="glass-strong rounded-2xl p-6 mb-8 space-y-4">
          <div>
            <label className="text-xs text-slate-500">NFT contract</label>
            <select value={selected} onChange={(e) => setSelected(e.target.value)}
              className="w-full mt-1 px-4 py-3 rounded-xl font-mono text-sm bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600">
              <option value="">— deploy ERC721 first —</option>
              {contracts.map((c) => (
                <option key={c.address} value={c.address}>{c.name || 'ERC721'} ({shorten(c.address)})</option>
              ))}
            </select>
          </div>
          <div className="grid sm:grid-cols-2 gap-3">
            <div>
              <label className="text-xs text-slate-500">Mint to</label>
              <input value={mintTo} onChange={(e) => setMintTo(e.target.value)} placeholder="0x…"
                className="w-full mt-1 px-4 py-3 rounded-xl font-mono text-sm bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600" />
            </div>
            <div>
              <label className="text-xs text-slate-500">Token ID</label>
              <input value={tokenId} onChange={(e) => setTokenId(e.target.value)} type="number" min="1"
                className="w-full mt-1 px-4 py-3 rounded-xl font-mono bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600" />
            </div>
          </div>
          <div className="flex flex-wrap gap-3">
            <button type="button" disabled={submitting} onClick={mint}
              className="px-6 py-3 rounded-xl bg-gradient-to-r from-pink-600 to-rose-600 text-white font-semibold flex items-center gap-2 disabled:opacity-50">
              {submitting ? <RefreshCw className="w-4 h-4 animate-spin" /> : <Plus className="w-4 h-4" />}
              Mint on-chain
            </button>
            <button type="button" onClick={refresh} className="px-4 py-3 rounded-xl border border-pink-500/40 text-pink-600">
              <RefreshCw className="w-4 h-4" />
            </button>
            <Link to="/deploy"
              className="px-6 py-3 rounded-xl border border-pink-500/50 text-pink-600 font-semibold flex items-center gap-2">
              <Sparkles className="w-4 h-4" /> Deploy ERC721
            </Link>
          </div>
          {status && (
            <p className={`text-sm flex items-center gap-2 ${statusType === 'err' ? 'text-red-500' : 'text-emerald-600'}`}>
              {statusType === 'err' ? <AlertCircle className="w-4 h-4" /> : <CheckCircle className="w-4 h-4" />}
              {status}
            </p>
          )}
        </div>

        <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {gallery.length === 0 && (
            <div className="col-span-full glass rounded-2xl p-10 text-center text-slate-500">
              No tokens found via <code className="font-mono text-xs">ownerOf</code> — mint or deploy at{' '}
              <Link to="/deploy" className="text-pink-500 underline">/deploy</Link>
            </div>
          )}
          {gallery.map((n) => (
            <div key={`${n.contract}-${n.tokenId}`} className="glass-strong rounded-2xl overflow-hidden">
              <div className="h-32 bg-gradient-to-br from-pink-500/30 via-violet-500/20 to-cyan-500/30 flex items-center justify-center">
                <Image className="w-12 h-12 text-white/60" />
              </div>
              <div className="p-4">
                <p className="font-semibold">{n.name}</p>
                <p className="text-xs text-slate-500 font-mono">#{n.tokenId}</p>
                <p className="text-xs text-slate-400 mt-1 truncate">Owner: {shorten(n.owner)}</p>
                <p className="text-xs text-slate-400 truncate">{shorten(n.contract)}</p>
              </div>
            </div>
          ))}
        </div>
      </div>
    </PageShell>
  )
}
