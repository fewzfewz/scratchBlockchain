import { useState, useEffect, useCallback } from 'react'
import { Link } from 'react-router-dom'
import nacl from 'tweetnacl'
import { Rocket, Wallet, Copy, RefreshCw, FileCode, Coins, Image, Code2, ArrowRight, BookOpen, ChevronDown } from 'lucide-react'
import { Page, PageHeader, Card, Field, Button, Notice, KeyValue, inputClass, textareaClass, buttonClass } from '../components/ui.jsx'
import {
  saveContract,
  buildSignedDeployTx,
  submitDeployTx,
  waitForReceipt,
  receiptStatus,
  getApiUrl,
  txHash,
} from '../lib/chain.js'
import demoContracts from '../lib/demoContracts.json'

const fromHex = (hex) => {
  const clean = hex.replace(/^0x/, '')
  const b = new Uint8Array(clean.length / 2)
  for (let i = 0; i < clean.length; i += 2) b[i / 2] = parseInt(clean.substr(i, 2), 16)
  return b
}

// Real compiled contracts (interop/contracts/demo, exported by
// interop/scripts/export-demo-contracts.js).
const CONTRACT_PRESETS = Object.fromEntries(
  Object.entries(demoContracts).map(([k, v]) => [k, { label: v.label, bytecode: v.bytecode.replace(/^0x/, '') }]),
)


export default function ContractDeployPage() {
  const [apiUrl] = useState(getApiUrl)
  const [keyPair, setKeyPair] = useState(null)
  const [address, setAddress] = useState('')
  const [preset, setPreset] = useState('ERC20')
  const [bytecode, setBytecode] = useState(CONTRACT_PRESETS.ERC20.bytecode)
  const [gasLimit, setGasLimit] = useState(500000)
  const [estimatedGas, setEstimatedGas] = useState(null)
  const [status, setStatus] = useState('')
  const [statusType, setStatusType] = useState('')
  const [deployResult, setDeployResult] = useState(null)
  const [loading, setLoading] = useState(false)
  const [pendingHash, setPendingHash] = useState(null)
  const [showBytecode, setShowBytecode] = useState(false)

  useEffect(() => {
    const priv = localStorage.getItem('nebula_wallet_priv')
    const addr = localStorage.getItem('nebula_wallet_addr')
    if (priv && addr) {
      try {
        const secret = fromHex(priv)
        const kp = nacl.sign.keyPair.fromSecretKey(secret)
        setKeyPair(kp)
        setAddress(addr)
      } catch {
        /* ignore invalid saved key */
      }
    }
  }, [])

  useEffect(() => {
    if (preset !== 'Custom') {
      setBytecode(CONTRACT_PRESETS[preset]?.bytecode || '')
    }
  }, [preset])

  const showMsg = (msg, type = 'info') => {
    setStatus(msg)
    setStatusType(type)
  }

  const fetchNonce = async () => {
    const r = await window.fetch(`${apiUrl}/balance/${address.replace(/^0x/, '')}`)
    if (r.ok) {
      const d = await r.json()
      return d.nonce || 0
    }
    return 0
  }

  const pollReceipt = useCallback(async (hash) => {
    setPendingHash(hash)
    showMsg(`Deploy submitted — waiting for confirmation (${hash.slice(0, 10)}...)`, 'info')
    const receipt = await waitForReceipt(hash, 20, 2500)
    setPendingHash(null)
    if (!receipt) {
      showMsg('Deploy submitted but receipt not found yet — check History or retry later', 'info')
      setDeployResult({ hash, receipt: null })
      return
    }
    setDeployResult({ hash, receipt })
    const addr = receipt?.contract_address || receipt?.created_address
    if (receiptStatus(receipt) === 'confirmed' && addr) {
      const full = String(addr).startsWith('0x') ? String(addr) : `0x${addr}`
      saveContract({
        address: full,
        type: preset,
        name: CONTRACT_PRESETS[preset]?.label || preset,
        txHash: hash,
      })
      showMsg(`Contract deployed at ${full.slice(0, 10)}...`, 'success')
    } else if (receiptStatus(receipt) === 'failed') {
      showMsg(`Deploy failed on-chain: ${receipt.revert_reason || 'execution reverted'}`, 'error')
    } else {
      showMsg('Deploy transaction included but no contract address returned', 'info')
    }
  }, [preset])

  // Simulated by the node against current state (POST /estimate_gas).
  const requestEstimate = async () => {
    const r = await window.fetch(`${apiUrl}/estimate_gas`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ from: address, to: '', data: '0x' + bytecode.replace(/^0x/, '').trim(), value: '0' }),
    })
    if (!r.ok) throw new Error(`estimate failed (${r.status})`)
    const d = await r.json()
    if (d.error) throw new Error(`deployment would fail: ${d.error}`)
    return d.estimated_gas
  }

  const estimateGas = async () => {
    if (!address || !bytecode) return
    try {
      const est = await requestEstimate()
      setEstimatedGas(est)
      setGasLimit(est)
      showMsg(`Estimated gas: ${est.toLocaleString()}`, 'success')
    } catch (e) {
      showMsg(e.message.startsWith('deployment would fail') ? e.message : 'Gas estimation unavailable — using manual limit', 'error')
    }
  }

  const deploy = async (e) => {
    e.preventDefault()
    if (!keyPair) {
      showMsg('Create or import a wallet on the Wallet page first', 'error')
      return
    }
    if (loading || pendingHash) {
      showMsg('A deploy is already in progress — wait for confirmation', 'info')
      return
    }
    const clean = bytecode.replace(/^0x/, '').trim()
    if (!clean || clean.length < 4) {
      showMsg('Enter valid contract bytecode', 'error')
      return
    }
    setLoading(true)
    setDeployResult(null)
    try {
      showMsg('Estimating gas...', 'info')
      let limit = gasLimit
      try {
        limit = await requestEstimate()
        setEstimatedGas(limit)
        setGasLimit(limit)
      } catch (err) {
        if (String(err.message).startsWith('deployment would fail')) throw err
      }
      showMsg('Preparing deployment transaction...', 'info')
      const nonce = await fetchNonce()
      const tx = await buildSignedDeployTx({
        keyPair,
        from: address,
        bytecodeHex: clean,
        nonce,
        gasLimit: limit,
      })

      showMsg('Submitting contract creation...', 'info')
      const data = await submitDeployTx(tx).catch(async (err) => {
        if (String(err.message).includes('already in mempool')) {
          const hashHex = Array.from(await txHash(tx)).map(b => b.toString(16).padStart(2, '0')).join('')
          return { status: err.message, hash: hashHex, alreadyPending: true }
        }
        throw err
      })

      if (data.alreadyPending) {
        showMsg('Deploy already pending in mempool — tracking existing transaction', 'info')
        await pollReceipt(data.hash)
        return
      }

      const hash = data.hash
      if (!hash) {
        showMsg('Deploy failed: no hash returned', 'error')
        return
      }
      await pollReceipt(hash)
    } catch (err) {
      showMsg('Error: ' + err.message, 'error')
    } finally {
      setLoading(false)
    }
  }

  const contractAddress =
    deployResult?.receipt?.contract_address ||
    deployResult?.receipt?.created_address ||
    null

  const TEMPLATES = [
    { id: 'ERC20', icon: Coins, title: 'Token (ERC20)', desc: '1,000,000 NDT minted to you; transfer and mint.' },
    { id: 'ERC721', icon: Image, title: 'NFT collection (ERC721)', desc: 'Mint numbered tokens to any address.' },
    { id: 'Custom', icon: Code2, title: 'Custom bytecode', desc: 'Paste the init code of your own contract.' },
  ]
  const pickTemplate = (id) => {
    setPreset(id)
    if (id === 'Custom') { setBytecode(''); setShowBytecode(true) }
  }

  return (
    <Page width="max-w-5xl">
      <PageHeader icon={Rocket} eyebrow="Deploy" title="Deploy a contract"
        description="Send a contract-creation transaction to the EVM. Gas is estimated by simulating it on your node."
        actions={<Link to="/docs" className={buttonClass('secondary', 'sm')}><BookOpen className="w-4 h-4" /> How it works</Link>} />

      {!keyPair ? (
        <Card className="max-w-xl">
          <div className="text-center py-6">
            <Wallet className="w-8 h-8 mx-auto text-slate-400" />
            <h2 className="mt-3 font-semibold text-slate-900 dark:text-white">You need a funded wallet</h2>
            <p className="mt-1 text-sm text-slate-500">Deploying costs gas. Create a wallet and get test NBL first.</p>
            <Link to="/wallet" className={buttonClass('primary', 'md', 'mt-5')}><Wallet className="w-4 h-4" /> Open wallet</Link>
          </div>
        </Card>
      ) : (
        <form onSubmit={deploy} className="grid lg:grid-cols-[1.5fr_1fr] gap-6">
          <div className="space-y-6">
            <Card title="1. Choose a template">
              <div role="radiogroup" className="grid sm:grid-cols-3 gap-3">
                {TEMPLATES.map(({ id, icon: Icon, title, desc }) => (
                  <button key={id} type="button" role="radio" aria-checked={preset === id} onClick={() => pickTemplate(id)}
                    className={`text-left p-4 rounded-xl border transition ${preset === id ? 'border-blue-500 bg-blue-500/5 ring-2 ring-blue-500/20' : 'border-slate-200 dark:border-slate-800 hover:border-slate-300 dark:hover:border-slate-700'}`}>
                    <Icon className={`w-5 h-5 ${preset === id ? 'text-blue-600 dark:text-blue-300' : 'text-slate-400'}`} />
                    <div className="mt-2 text-sm font-semibold text-slate-900 dark:text-white">{title}</div>
                    <div className="mt-1 text-xs text-slate-500">{desc}</div>
                  </button>
                ))}
              </div>
              <button type="button" onClick={() => setShowBytecode((v) => !v)} aria-expanded={showBytecode}
                className="mt-4 inline-flex items-center gap-1.5 text-xs font-medium text-slate-500 hover:text-slate-900 dark:hover:text-white">
                <FileCode className="w-3.5 h-3.5" /> {showBytecode ? 'Hide' : 'Show'} init bytecode
                <ChevronDown className={`w-3.5 h-3.5 transition-transform ${showBytecode ? 'rotate-180' : ''}`} />
              </button>
              {showBytecode && (
                <textarea value={bytecode} rows={5} aria-label="Init bytecode (hex)" placeholder="6080604052..."
                  onChange={(e) => { setPreset('Custom'); setBytecode(e.target.value.replace(/^0x/, '')) }}
                  className={`${textareaClass} mt-3 font-mono text-xs resize-y`} />
              )}
            </Card>

            <Card title="2. Gas">
              <div className="flex gap-2 items-end">
                <Field label="Gas limit" hint={estimatedGas != null ? `Simulated: ${estimatedGas.toLocaleString()} gas (with headroom)` : 'Estimated automatically when you deploy.'} className="flex-1">
                  <input type="number" value={gasLimit} onChange={(e) => setGasLimit(Number(e.target.value))} className={inputClass} />
                </Field>
                <Button variant="secondary" icon={RefreshCw} onClick={estimateGas} className="mb-[22px]">Estimate</Button>
              </div>
            </Card>
          </div>

          <div className="space-y-6">
            <Card title="3. Deploy">
              <KeyValue rows={[
                ['From', `${address.slice(0, 10)}…${address.slice(-6)}`, true],
                ['Template', TEMPLATES.find((t) => t.id === preset)?.title || preset],
                ['Code size', `${Math.floor((bytecode || '').length / 2).toLocaleString()} bytes`],
              ]} />
              <Button type="submit" icon={Rocket} loading={loading || !!pendingHash} className="w-full mt-5">
                {loading || pendingHash ? 'Deploying…' : 'Deploy contract'}
              </Button>
              <Notice type={statusType === 'error' ? 'err' : statusType === 'success' ? 'ok' : 'info'} className="mt-4">{status}</Notice>
            </Card>

            {deployResult && (
              <Card title={contractAddress ? 'Deployed' : 'Transaction'}>
                <div className="space-y-3 text-sm">
                  <div>
                    <div className="text-xs text-slate-500">Transaction</div>
                    <code className="block font-mono text-xs break-all text-slate-700 dark:text-slate-300">{deployResult.hash}</code>
                  </div>
                  {contractAddress && (
                    <div>
                      <div className="text-xs text-slate-500">Contract address</div>
                      <div className="flex items-center gap-2">
                        <code className="font-mono text-xs break-all text-emerald-600 dark:text-emerald-400">{contractAddress}</code>
                        <button type="button" onClick={() => navigator.clipboard.writeText(contractAddress)} aria-label="Copy contract address" className="text-slate-400 hover:text-slate-900 dark:hover:text-white"><Copy className="w-3.5 h-3.5" /></button>
                      </div>
                    </div>
                  )}
                  {contractAddress && (
                    <Link to={preset === 'ERC721' ? '/nft' : '/contracts'} className="inline-flex items-center gap-1.5 font-medium text-blue-600 dark:text-blue-300 hover:underline">
                      {preset === 'ERC721' ? 'Mint an NFT' : 'Interact with it'} <ArrowRight className="w-3.5 h-3.5" />
                    </Link>
                  )}
                </div>
              </Card>
            )}
          </div>
        </form>
      )}
    </Page>
  )
}
