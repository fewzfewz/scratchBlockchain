import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { FileCode, Fuel, Send, AlertCircle, CheckCircle, RefreshCw } from 'lucide-react'
import PageShell from '../components/PageShell.jsx'
import {
  callContract, encodeBalanceOf, estimateGas, loadWalletKeyPair,
  scanDeployedContracts, signAndSubmit, waitForReceipt, walletAddress,
  decodeUint256, weiToNbl, shorten, SELECTORS,
} from '../lib/chain.js'

export default function ContractInteractPage() {
  const [contract, setContract] = useState('')
  const [from, setFrom] = useState(() => walletAddress())
  const [calldata, setCalldata] = useState('0x')
  const [value, setValue] = useState('0')
  const [estimate, setEstimate] = useState(null)
  const [readResult, setReadResult] = useState(null)
  const [contracts, setContracts] = useState([])
  const [status, setStatus] = useState('')
  const [statusType, setStatusType] = useState('')
  const [submitting, setSubmitting] = useState(false)

  useEffect(() => {
    scanDeployedContracts().then(setContracts)
  }, [])

  const presets = [
    { label: 'balanceOf', data: () => encodeBalanceOf(from) },
    { label: 'totalSupply', data: () => SELECTORS.totalSupply },
  ]

  const runEstimate = async () => {
    setStatus('')
    try {
      const d = await estimateGas(from, contract, calldata, value)
      setEstimate(d)
      setStatus(`Estimated gas: ${d.estimated_gas}`)
      setStatusType('ok')
    } catch {
      setStatus('Estimate failed — check node and fields')
      setStatusType('err')
    }
  }

  const runRead = async () => {
    setReadResult(null)
    try {
      const d = await callContract(from, contract, calldata, value)
      if (!d.success) {
        setStatus(d.error || 'Call reverted')
        setStatusType('err')
        return
      }
      setReadResult(d.result)
      const num = decodeUint256(d.result)
      setStatus(`Read OK — uint256: ${num.toString()} (${weiToNbl(num.toString())} if token)`)
      setStatusType('ok')
    } catch (e) {
      setStatus(e.message || 'Read failed')
      setStatusType('err')
    }
  }

  const sendWrite = async () => {
    if (!loadWalletKeyPair()) {
      setStatus('Create a wallet on /wallet first')
      setStatusType('err')
      return
    }
    setSubmitting(true)
    setStatus('Submitting contract call…')
    try {
      const est = estimate || await estimateGas(from, contract, calldata, value)
      const { hash } = await signAndSubmit({
        from,
        to: contract,
        valueWei: BigInt(value || '0'),
        payload: calldata.replace(/^0x/, ''),
        gasLimit: est.estimated_gas || 200000,
      })
      const receipt = await waitForReceipt(hash)
      setStatus(`Tx ${shorten(hash, 10, 8)} — ${receipt?.success !== false ? 'success' : 'failed'}`)
      setStatusType(receipt?.success === false ? 'err' : 'ok')
    } catch (e) {
      setStatus(e.message || 'Send failed')
      setStatusType('err')
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <PageShell variant="default">
      <div className="max-w-2xl mx-auto px-4 py-10 animate-fade-in">
        <header className="text-center mb-8">
          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-indigo-600 to-blue-600 flex items-center justify-center text-white shadow-lg">
              <FileCode className="w-6 h-6" />
            </div>
            <h1 className="text-3xl font-bold text-slate-900 dark:text-white">Contract Interaction</h1>
          </div>
          <p className="text-slate-500 dark:text-slate-400">Read via <code className="font-mono text-xs">POST /call_contract</code> · write via signed txs.</p>
        </header>

        {contracts.length > 0 && (
          <div className="glass rounded-2xl p-4 mb-4">
            <p className="text-xs text-slate-500 mb-2">Deployed contracts</p>
            <div className="flex flex-wrap gap-2">
              {contracts.map((c) => (
                <button key={c.address} type="button" onClick={() => setContract(c.address)}
                  className="text-xs px-3 py-1 rounded-lg bg-indigo-500/15 text-indigo-600 font-mono">
                  {c.name || shorten(c.address)}
                </button>
              ))}
            </div>
          </div>
        )}

        <div className="glass-strong rounded-2xl p-6 space-y-4">
          <div>
            <label className="text-xs text-slate-500">Contract address</label>
            <input value={contract} onChange={(e) => setContract(e.target.value)} placeholder="0x…"
              className="w-full mt-1 px-4 py-3 rounded-xl font-mono text-sm bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600" />
          </div>
          <div>
            <label className="text-xs text-slate-500">From (caller)</label>
            <input value={from} onChange={(e) => setFrom(e.target.value)} placeholder="0x…"
              className="w-full mt-1 px-4 py-3 rounded-xl font-mono text-sm bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600" />
          </div>
          <div>
            <label className="text-xs text-slate-500">Calldata (hex)</label>
            <textarea value={calldata} onChange={(e) => setCalldata(e.target.value)} rows={3}
              className="w-full mt-1 px-4 py-3 rounded-xl font-mono text-sm bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600" />
            <div className="flex flex-wrap gap-2 mt-2">
              {presets.map((p) => (
                <button key={p.label} type="button" onClick={() => setCalldata(p.data())}
                  className="text-xs px-3 py-1 rounded-lg bg-slate-200/80 dark:bg-slate-700/60">
                  {p.label}
                </button>
              ))}
            </div>
          </div>
          <div>
            <label className="text-xs text-slate-500">Value (wei)</label>
            <input value={value} onChange={(e) => setValue(e.target.value)}
              className="w-full mt-1 px-4 py-3 rounded-xl font-mono text-sm bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600" />
          </div>

          <div className="grid sm:grid-cols-3 gap-2">
            <button type="button" onClick={runRead}
              className="py-3 rounded-xl border border-indigo-500/50 text-indigo-600 font-semibold text-sm">
              Read (call)
            </button>
            <button type="button" onClick={runEstimate}
              className="py-3 rounded-xl border border-indigo-500/50 text-indigo-600 font-semibold text-sm flex items-center justify-center gap-1">
              <Fuel className="w-4 h-4" /> Estimate
            </button>
            <button type="button" disabled={submitting} onClick={sendWrite}
              className="py-3 rounded-xl bg-gradient-to-r from-indigo-600 to-blue-600 text-white font-semibold text-sm flex items-center justify-center gap-1 disabled:opacity-50">
              {submitting ? <RefreshCw className="w-4 h-4 animate-spin" /> : <Send className="w-4 h-4" />}
              Write
            </button>
          </div>

          {readResult && (
            <div className="text-xs font-mono bg-slate-100 dark:bg-slate-900/50 rounded-xl p-3 break-all">
              {readResult}
            </div>
          )}
          {estimate && (
            <div className="text-xs font-mono bg-slate-100 dark:bg-slate-900/50 rounded-xl p-3">
              gas: {estimate.estimated_gas} · cost ≈ {estimate.total_cost_estimate} wei
            </div>
          )}
          {status && (
            <p className={`text-sm flex items-center gap-2 ${statusType === 'err' ? 'text-red-500' : 'text-emerald-600'}`}>
              {statusType === 'err' ? <AlertCircle className="w-4 h-4" /> : <CheckCircle className="w-4 h-4" />}
              {status}
            </p>
          )}
        </div>

        <p className="text-center text-xs text-slate-400 mt-6">
          Deploy at <Link to="/deploy" className="text-indigo-500 underline">/deploy</Link>
        </p>
      </div>
    </PageShell>
  )
}
