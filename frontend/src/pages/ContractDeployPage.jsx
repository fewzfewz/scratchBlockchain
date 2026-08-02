import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import nacl from 'tweetnacl'
import { Rocket, Wallet, Copy, RefreshCw, FileCode, Fuel, CheckCircle, AlertCircle } from 'lucide-react'

const toHex = (buf) => Array.from(new Uint8Array(buf)).map(b => b.toString(16).padStart(2, '0')).join('')
const fromHex = (hex) => {
  const clean = hex.replace(/^0x/, '')
  const b = new Uint8Array(clean.length / 2)
  for (let i = 0; i < clean.length; i += 2) b[i / 2] = parseInt(clean.substr(i, 2), 16)
  return b
}

const CONTRACT_PRESETS = {
  ERC20: {
    label: 'ERC20 Token',
    bytecode:
      '608060405260405180604001604052806007815260200166455243323056360bc1b815250604051806040016040528060038152602001624554360ea1b815250601260006101000a81548160ff021916908360ff160217905550336000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff160217905550',
  },
  ERC721: {
    label: 'ERC721 NFT',
    bytecode:
      '6080604052604051806040016040528060058152602001644552433732360d81b815250604051806040016040528060038152602001624e465460ea1b815250816000908051906020019061005c92919061008c565b50806001908051906020019061007392919061008c565b505061010b565b828054610086906100da565b6000825580601f1061009857506100b7565b601f0160209004906000526020600020908101906100b791906100ba565b50565b5b808211156100d357600081556001016100bb565b5090565b600060028204905060005b600660040b8281049050600081526020016001815182026020019150505b92915050565b6101cd8061011a6000396000f3fe',
  },
}

const inputCls =
  'w-full px-3 py-2.5 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-800 dark:text-slate-200 placeholder-slate-400 dark:placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500/40 font-mono'

export default function ContractDeployPage() {
  const [apiUrl] = useState(() => localStorage.getItem('nebula_rpc_url') || 'http://localhost:8545')
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

  const sha256 = async (data) => new Uint8Array(await crypto.subtle.digest('SHA-256', data))

  const txHash = async (tx) => {
    let h = new Uint8Array(0)
    const ap = (b) => {
      const a = new Uint8Array(h.length + b.length)
      a.set(h)
      a.set(b, h.length)
      h = a
    }
    ap(new Uint8Array(tx.sender.slice(0, 20)))
    const nb = new Uint8Array(8)
    new DataView(nb.buffer).setBigUint64(0, BigInt(tx.nonce), true)
    ap(nb)
    ap(new Uint8Array(tx.payload))
    const gb = new Uint8Array(8)
    new DataView(gb.buffer).setBigUint64(0, BigInt(tx.gas_limit), true)
    ap(gb)
    const fb = new Uint8Array(8)
    new DataView(fb.buffer).setBigUint64(0, BigInt(tx.max_fee_per_gas), true)
    ap(fb)
    const pb = new Uint8Array(8)
    new DataView(pb.buffer).setBigUint64(0, BigInt(tx.max_priority_fee_per_gas), true)
    ap(pb)
    if (tx.chain_id) {
      const cb = new Uint8Array(8)
      new DataView(cb.buffer).setBigUint64(0, BigInt(tx.chain_id), true)
      ap(cb)
    }
    if (tx.to && tx.to.length) ap(new Uint8Array(tx.to.slice(0, 20)))
    const vb = new Uint8Array(8)
    new DataView(vb.buffer).setBigUint64(0, BigInt(tx.value), true)
    ap(vb)
    return await sha256(await sha256(h))
  }

  const fetchNonce = async () => {
    const r = await window.fetch(`${apiUrl}/balance/${address}`)
    if (r.ok) {
      const d = await r.json()
      return d.nonce || 0
    }
    return 0
  }

  const estimateGas = async () => {
    if (!address || !bytecode) return
    try {
      const payload = Array.from(fromHex(bytecode))
      const r = await window.fetch(`${apiUrl}/estimate_gas`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          sender: Array.from(fromHex(address.replace('0x', ''))),
          payload,
          gas_limit: gasLimit,
          max_fee_per_gas: 1e9,
          max_priority_fee_per_gas: 1e8,
          chain_id: 1,
          value: 0,
        }),
      })
      if (r.ok) {
        const d = await r.json()
        const est = d.gas_estimate || d.gas || gasLimit
        setEstimatedGas(est)
        setGasLimit(est)
        showMsg(`Estimated gas: ${est.toLocaleString()}`, 'success')
      } else {
        showMsg('Gas estimation unavailable — using manual limit', 'info')
      }
    } catch {
      showMsg('Could not reach node for gas estimate', 'error')
    }
  }

  const deploy = async (e) => {
    e.preventDefault()
    if (!keyPair) {
      showMsg('Create or import a wallet on the Wallet page first', 'error')
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
      showMsg('Preparing deployment transaction...', 'info')
      const nonce = await fetchNonce()
      const payload = Array.from(fromHex(clean))
      const tx = {
        sender: Array.from(fromHex(address.replace('0x', ''))),
        nonce,
        value: 0,
        gas_limit: gasLimit,
        max_fee_per_gas: 1e9,
        max_priority_fee_per_gas: 1e8,
        payload,
        chain_id: 1,
        signature: [],
      }
      const msg = await txHash(tx)
      tx.signature = Array.from(nacl.sign.detached(msg, keyPair.secretKey))

      showMsg('Submitting contract creation...', 'info')
      const r = await window.fetch(`${apiUrl}/submit_tx`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(tx),
      })
      const text = await r.text()
      if (!r.ok) {
        showMsg('Deploy failed: ' + text, 'error')
        return
      }

      const hash = text.replace(/^"|"$/g, '')
      showMsg(`Deploy tx sent: ${hash.slice(0, 10)}...`, 'success')

      setTimeout(async () => {
        try {
          const rr = await window.fetch(`${apiUrl}/tx/${hash}`)
          if (rr.ok) {
            const d = await rr.json()
            const receipt = d.receipt
            setDeployResult({ hash, receipt })
            if (receipt?.contract_address || receipt?.created_address) {
              showMsg('Contract deployed successfully!', 'success')
            }
          }
        } catch {
          /* receipt may not be indexed yet */
        }
      }, 5000)
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

  return (
    <div className="relative min-h-[70vh] overflow-hidden animate-fade-in">
      <div className="absolute inset-0 bg-grid pointer-events-none" />
      <div className="relative z-10 max-w-3xl mx-auto px-4 py-10">
        <div className="text-center mb-8">
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full glass text-xs text-blue-600 dark:text-cyan-400 mb-4">
            <Rocket className="w-3.5 h-3.5" />
            EVM contract deployment
          </div>
          <h1 className="text-3xl font-bold text-slate-900 dark:text-white mb-2">Deploy Contract</h1>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Submit a contract-creation transaction via the node RPC
          </p>
        </div>

        {!keyPair ? (
          <div className="p-6 rounded-2xl glass-strong text-center">
            <Wallet className="w-10 h-10 mx-auto text-slate-400 mb-3" />
            <p className="text-sm text-slate-600 dark:text-slate-300 mb-4">
              You need a funded wallet before deploying contracts.
            </p>
            <Link
              to="/wallet"
              className="inline-flex items-center gap-2 px-4 py-2 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 text-white text-sm font-medium"
            >
              <Wallet className="w-4 h-4" />
              Open Wallet
            </Link>
          </div>
        ) : (
          <form onSubmit={deploy} className="p-6 rounded-2xl glass-strong space-y-5">
            <div>
              <label className="text-xs uppercase tracking-wider text-slate-500 dark:text-slate-400">Deployer</label>
              <div className="mt-1 flex items-center gap-2">
                <code className="flex-1 text-xs font-mono text-slate-700 dark:text-slate-200 truncate">{address}</code>
                <button
                  type="button"
                  onClick={() => navigator.clipboard.writeText(address)}
                  className="p-2 rounded-lg hover:bg-slate-200/50 dark:hover:bg-slate-700/50"
                >
                  <Copy className="w-4 h-4 text-slate-400" />
                </button>
              </div>
            </div>

            <div>
              <label className="text-xs uppercase tracking-wider text-slate-500 dark:text-slate-400">Template</label>
              <select
                value={preset}
                onChange={(e) => setPreset(e.target.value)}
                className={`mt-1 ${inputCls} font-sans`}
              >
                {Object.entries(CONTRACT_PRESETS).map(([key, { label }]) => (
                  <option key={key} value={key}>
                    {label}
                  </option>
                ))}
                <option value="Custom">Custom bytecode</option>
              </select>
            </div>

            <div>
              <label className="text-xs uppercase tracking-wider text-slate-500 dark:text-slate-400 flex items-center gap-1">
                <FileCode className="w-3.5 h-3.5" />
                Init bytecode (hex)
              </label>
              <textarea
                value={bytecode}
                onChange={(e) => {
                  setPreset('Custom')
                  setBytecode(e.target.value.replace(/^0x/, ''))
                }}
                rows={4}
                className={`mt-1 ${inputCls} resize-y`}
                placeholder="6080604052..."
              />
            </div>

            <div className="flex gap-3 items-end">
              <div className="flex-1">
                <label className="text-xs uppercase tracking-wider text-slate-500 dark:text-slate-400 flex items-center gap-1">
                  <Fuel className="w-3.5 h-3.5" />
                  Gas limit
                </label>
                <input
                  type="number"
                  value={gasLimit}
                  onChange={(e) => setGasLimit(Number(e.target.value))}
                  className={`mt-1 ${inputCls} font-sans`}
                />
              </div>
              <button
                type="button"
                onClick={estimateGas}
                className="px-4 py-2.5 rounded-xl border border-slate-300 dark:border-slate-600 text-sm text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-700/50"
              >
                <RefreshCw className="w-4 h-4 inline mr-1" />
                Estimate
              </button>
            </div>
            {estimatedGas != null && (
              <p className="text-xs text-emerald-600 dark:text-emerald-400">Estimated: {estimatedGas.toLocaleString()} gas</p>
            )}

            <button
              type="submit"
              disabled={loading}
              className="w-full py-3 rounded-xl bg-gradient-to-r from-violet-600 to-fuchsia-600 hover:opacity-90 text-white font-medium transition-all disabled:opacity-50"
            >
              {loading ? 'Deploying...' : 'Deploy Contract'}
            </button>

            {status && (
              <div
                className={`flex items-start gap-2 p-3 rounded-xl text-sm ${
                  statusType === 'error'
                    ? 'bg-red-500/10 text-red-600 dark:text-red-400'
                    : statusType === 'success'
                      ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
                      : 'bg-blue-500/10 text-blue-600 dark:text-blue-400'
                }`}
              >
                {statusType === 'success' ? (
                  <CheckCircle className="w-4 h-4 shrink-0 mt-0.5" />
                ) : statusType === 'error' ? (
                  <AlertCircle className="w-4 h-4 shrink-0 mt-0.5" />
                ) : null}
                <span>{status}</span>
              </div>
            )}

            {deployResult && (
              <div className="p-4 rounded-xl bg-slate-100/60 dark:bg-slate-700/40 space-y-2">
                <p className="text-xs text-slate-500">Transaction</p>
                <code className="text-xs font-mono break-all">{deployResult.hash}</code>
                {contractAddress && (
                  <>
                    <p className="text-xs text-slate-500 mt-2">Contract address</p>
                    <code className="text-xs font-mono break-all text-emerald-600 dark:text-emerald-400">
                      {contractAddress}
                    </code>
                  </>
                )}
              </div>
            )}
          </form>
        )}
      </div>
    </div>
  )
}
