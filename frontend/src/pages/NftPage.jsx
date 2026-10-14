import { useState, useEffect, useCallback } from 'react'
import { Link } from 'react-router-dom'
import { Image, Plus, Sparkles, RefreshCw } from 'lucide-react'
import { Page, PageHeader, Card, Field, Button, Notice, EmptyState, buttonClass, inputClass } from '../components/ui.jsx'
import {
  encodeMint, loadNftsFromChain, scanDeployedContracts,
  loadWalletKeyPair, signAndSubmit, waitForReceipt, walletAddress, shorten,
  fetchJson,
} from '../lib/chain.js'


// Deterministic artwork for a token: hues and shape picked from a hash of
// (contract, id), so every token looks distinct and always the same.
function tokenArt(contract, id) {
  let h = 2166136261
  for (const c of `${contract}:${id}`) { h ^= c.charCodeAt(0); h = Math.imul(h, 16777619) >>> 0 }
  const a = h % 360
  const b = (a + 60 + ((h >> 9) % 120)) % 360
  const shape = (h >> 17) % 3
  return { a, b, shape, rot: (h >> 5) % 360 }
}

function TokenArt({ contract, id }) {
  const { a, b, shape, rot } = tokenArt(contract, id)
  return (
    <svg viewBox="0 0 120 120" className="w-full h-full" aria-hidden="true">
      <defs>
        <linearGradient id={`g-${contract}-${id}`} x1="0" y1="0" x2="1" y2="1" gradientTransform={`rotate(${rot % 90})`}>
          <stop offset="0" stopColor={`hsl(${a} 80% 60%)`} />
          <stop offset="1" stopColor={`hsl(${b} 75% 45%)`} />
        </linearGradient>
      </defs>
      <rect width="120" height="120" fill={`url(#g-${contract}-${id})`} />
      <g transform={`rotate(${rot} 60 60)`} fill="white" fillOpacity="0.22">
        {shape === 0 && <circle cx="60" cy="60" r="32" />}
        {shape === 1 && <rect x="30" y="30" width="60" height="60" rx="12" />}
        {shape === 2 && <path d="M60 22 L96 88 L24 88 Z" />}
      </g>
      <text x="60" y="66" textAnchor="middle" fontSize="18" fontWeight="600" fill="white" fontFamily="Sora, sans-serif">#{id}</text>
    </svg>
  )
}

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

  // Contract list: saved deploys plus creations found in the wallet's history.
  const refresh = useCallback(async () => {
    const list = await scanDeployedContracts()
    const nfts = list.filter((c) => c.type === 'ERC721' || c.name?.includes('721') || c.name?.includes('NFT'))
    setContracts(nfts)
    setSelected((s) => s || nfts[0]?.address || '')
  }, [])

  // Gallery: always the selected contract's tokens.
  const loadGallery = useCallback(async (contract) => {
    if (!contract || !addr) { setGallery([]); return }
    setGallery(await loadNftsFromChain(contract, addr))
  }, [addr])

  useEffect(() => {
    setMintTo(addr)
    refresh()
  }, [addr, refresh])

  useEffect(() => { loadGallery(selected) }, [selected, loadGallery])

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
      if (est.error) throw new Error(`Mint would fail: ${est.error}`)
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
      setTokenId((id) => String(Number(id) + 1))
      loadGallery(selected)
    } catch (e) {
      setStatus(e.message || 'Mint failed')
      setStatusType('err')
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <Page width="max-w-6xl">
      <PageHeader icon={Image} eyebrow="NFTs" title="Mint and browse NFTs"
        description="Mint ERC721 tokens on your node and see who owns them, read straight from the contract."
        actions={<Link to="/deploy" className={buttonClass('secondary', 'sm')}><Sparkles className="w-4 h-4" /> Deploy a collection</Link>} />

      <div className="grid lg:grid-cols-[1fr_2fr] gap-6 items-start">
        <Card title="Mint">
          <div className="space-y-4">
            <Field label="Collection">
              <select value={selected} onChange={(e) => setSelected(e.target.value)} className={`${inputClass} font-mono text-xs`}>
                <option value="">— deploy an ERC721 first —</option>
                {contracts.map((c) => (
                  <option key={c.address} value={c.address}>{c.name || 'ERC721'} ({shorten(c.address)})</option>
                ))}
              </select>
            </Field>
            <Field label="Mint to">
              <input value={mintTo} onChange={(e) => setMintTo(e.target.value)} placeholder="0x…" className={`${inputClass} font-mono`} />
            </Field>
            <Field label="Token id">
              <input value={tokenId} onChange={(e) => setTokenId(e.target.value)} type="number" min="1" className={inputClass} />
            </Field>
            <div className="flex gap-2">
              <Button icon={Plus} loading={submitting} onClick={mint} className="flex-1">Mint on-chain</Button>
              <Button variant="secondary" icon={RefreshCw} onClick={() => { refresh(); loadGallery(selected) }} aria-label="Refresh" />
            </div>
            <Notice type={statusType === 'err' ? 'err' : 'ok'}>{status}</Notice>
          </div>
        </Card>

        <Card title="Collection" description={selected ? `${gallery.length} token${gallery.length === 1 ? '' : 's'} minted · ${shorten(selected)}` : 'Pick or deploy a collection'}>
          {gallery.length === 0 ? (
            <EmptyState icon={Image} title="No tokens yet">Mint token #1 to start the collection.</EmptyState>
          ) : (
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-4">
              {gallery.map((n) => (
                <figure key={`${n.contract}-${n.tokenId}`} className="rounded-xl overflow-hidden border border-slate-200 dark:border-slate-800">
                  <div className="aspect-square"><TokenArt contract={n.contract} id={n.tokenId} /></div>
                  <figcaption className="p-3">
                    <div className="text-sm font-semibold text-slate-900 dark:text-white">{n.name}</div>
                    <div className="mt-0.5 font-mono text-[11px] text-slate-500 truncate">Owner: {shorten(n.owner)}</div>
                  </figcaption>
                </figure>
              ))}
            </div>
          )}
        </Card>
      </div>
    </Page>
  )
}
