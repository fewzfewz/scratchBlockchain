import { useState, useEffect, useCallback } from 'react'
import { Cpu, Play, Upload, RefreshCw, CheckCircle, AlertCircle } from 'lucide-react'
import PageShell from '../components/PageShell.jsx'
import {
  deployWasm, callWasm, listWasmContracts, getApiUrl,
} from '../lib/chain.js'

/** Sample module: `(func (export "double") (param i32) (result i32) ...)` */
const SAMPLE_WASM = 'AGFzbQEAAAABBgFgAX8BfwMCAQAHCgEGZG91YmxlAAAKCQEHACAAQQJsCw=='

export default function WasmPage() {
  const [contracts, setContracts] = useState([])
  const [name, setName] = useState('double-demo')
  const [wasmB64, setWasmB64] = useState(SAMPLE_WASM)
  const [callName, setCallName] = useState('double-demo')
  const [func, setFunc] = useState('double')
  const [arg, setArg] = useState('21')
  const [result, setResult] = useState(null)
  const [status, setStatus] = useState('')
  const [statusType, setStatusType] = useState('')
  const [busy, setBusy] = useState(false)

  const refresh = useCallback(async () => {
    try {
      const list = await listWasmContracts()
      setContracts(list)
      if (list[0] && !callName) setCallName(list[0])
    } catch {
      setContracts([])
    }
  }, [callName])

  useEffect(() => { refresh() }, [refresh])

  const show = (msg, type = 'info') => {
    setStatus(msg)
    setStatusType(type)
  }

  const handleDeploy = async () => {
    setBusy(true)
    show('Deploying WASM module…')
    try {
      const res = await deployWasm(name.trim(), wasmB64.trim())
      show(`Deployed "${res.name}" — hash ${res.code_hash?.slice(0, 12)}…`, 'ok')
      refresh()
    } catch (e) {
      show(e.message || 'Deploy failed', 'err')
    } finally {
      setBusy(false)
    }
  }

  const handleCall = async () => {
    setBusy(true)
    show('Calling WASM export…')
    try {
      const res = await callWasm(callName.trim(), func.trim(), parseInt(arg, 10) || 0)
      setResult(res.result)
      show(`Result: ${res.result}`, 'ok')
    } catch (e) {
      setResult(null)
      show(e.message || 'Call failed', 'err')
    } finally {
      setBusy(false)
    }
  }

  const inputCls =
    'w-full px-3 py-2.5 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/40'

  return (
    <PageShell>
      <div className="max-w-3xl mx-auto px-4 py-10 animate-fade-in">
        <header className="text-center mb-8">
          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-violet-600 to-purple-600 flex items-center justify-center text-white shadow-lg">
              <Cpu className="w-6 h-6" />
            </div>
            <h1 className="text-3xl font-bold text-slate-900 dark:text-white">WASM Contracts</h1>
          </div>
          <p className="text-slate-500 dark:text-slate-400">
            Deploy and call WebAssembly modules via <code className="text-xs">POST /deploy_wasm</code> and{' '}
            <code className="text-xs">POST /call_wasm</code> on {getApiUrl()}.
          </p>
        </header>

        {status && (
          <div className={`mb-4 px-4 py-3 rounded-xl text-sm flex items-center gap-2 ${
            statusType === 'ok' ? 'bg-emerald-500/15 text-emerald-400 border border-emerald-500/30'
              : statusType === 'err' ? 'bg-rose-500/15 text-rose-400 border border-rose-500/30'
                : 'bg-blue-500/15 text-blue-400 border border-blue-500/30'
          }`}>
            {statusType === 'ok' ? <CheckCircle className="w-4 h-4" /> : <AlertCircle className="w-4 h-4" />}
            {status}
          </div>
        )}

        <div className="glass-strong rounded-2xl p-6 mb-6 space-y-4">
          <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 flex items-center gap-2">
            <Upload className="w-4 h-4" /> Deploy module
          </h2>
          <div>
            <label className="text-xs text-slate-500">Contract name</label>
            <input value={name} onChange={(e) => setName(e.target.value)} className={inputCls} />
          </div>
          <div>
            <label className="text-xs text-slate-500">WASM (base64)</label>
            <textarea value={wasmB64} onChange={(e) => setWasmB64(e.target.value)} rows={3} className={inputCls} />
          </div>
          <button type="button" onClick={handleDeploy} disabled={busy}
            className="px-5 py-2.5 rounded-xl bg-violet-600 hover:bg-violet-500 text-white text-sm font-semibold disabled:opacity-50">
            Deploy
          </button>
        </div>

        <div className="glass-strong rounded-2xl p-6 mb-6 space-y-4">
          <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 flex items-center gap-2">
            <Play className="w-4 h-4" /> Call export
          </h2>
          <div className="grid sm:grid-cols-3 gap-3">
            <div>
              <label className="text-xs text-slate-500">Name</label>
              <input value={callName} onChange={(e) => setCallName(e.target.value)} className={inputCls} list="wasm-names" />
              <datalist id="wasm-names">
                {contracts.map((c) => <option key={c} value={c} />)}
              </datalist>
            </div>
            <div>
              <label className="text-xs text-slate-500">Function</label>
              <input value={func} onChange={(e) => setFunc(e.target.value)} className={inputCls} />
            </div>
            <div>
              <label className="text-xs text-slate-500">i32 arg</label>
              <input value={arg} onChange={(e) => setArg(e.target.value)} className={inputCls} />
            </div>
          </div>
          <button type="button" onClick={handleCall} disabled={busy}
            className="px-5 py-2.5 rounded-xl bg-cyan-600 hover:bg-cyan-500 text-white text-sm font-semibold disabled:opacity-50">
            Call
          </button>
          {result != null && (
            <p className="text-sm font-mono text-emerald-400">return = {String(result)}</p>
          )}
        </div>

        <div className="glass-strong rounded-2xl p-6">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">Deployed ({contracts.length})</h2>
            <button type="button" onClick={refresh} className="p-2 rounded-lg hover:bg-slate-200/50 dark:hover:bg-slate-700/40">
              <RefreshCw className="w-4 h-4 text-slate-400" />
            </button>
          </div>
          {contracts.length === 0 ? (
            <p className="text-xs text-slate-500">No WASM contracts yet — deploy the sample module above.</p>
          ) : (
            <ul className="space-y-2">
              {contracts.map((c) => (
                <li key={c} className="px-3 py-2 rounded-xl bg-slate-100/60 dark:bg-slate-700/30 font-mono text-sm">{c}</li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </PageShell>
  )
}
