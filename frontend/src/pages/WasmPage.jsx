import { useState, useEffect, useCallback } from 'react'
import { Cpu, Play, Upload, RefreshCw } from 'lucide-react'
import { Page, PageHeader, Card, Field, Button, Notice, EmptyState, Badge, inputClass, textareaClass } from '../components/ui.jsx'
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
      // Registered only when the deploy transaction is in a block.
      show(`Deploy submitted — waiting for "${res.name}" to be included…`)
      for (let i = 0; i < 30; i++) {
        const list = await listWasmContracts().catch(() => [])
        if (list.includes(res.name)) {
          setContracts(list)
          setCallName(res.name)
          show(`Deployed "${res.name}" — code hash ${res.code_hash?.slice(0, 12)}…`, 'ok')
          return
        }
        await new Promise((r) => setTimeout(r, 1000))
      }
      show(`"${res.name}" was submitted but is not on chain yet — refresh in a moment`, 'err')
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

  return (
    <Page width="max-w-5xl">
      <PageHeader icon={Cpu} eyebrow="WASM" title="WebAssembly contracts"
        description="Deploy a WASM module by name and call its exported i32 functions. Execution is fuel-metered by wasmtime."
        actions={<Badge tone="amber">Dev mode</Badge>} />

      <div className="grid lg:grid-cols-2 gap-6 items-start">
        <Card title="Deploy a module" icon={Upload} description="Registered on chain when the deploy transaction is included.">
          <div className="space-y-4">
            <Field label="Contract name"><input value={name} onChange={(e) => setName(e.target.value)} className={`${inputClass} font-mono`} /></Field>
            <Field label="Module (base64)" hint="The sample exports double(i32) → i32.">
              <textarea value={wasmB64} onChange={(e) => setWasmB64(e.target.value)} rows={3} className={`${textareaClass} font-mono text-xs`} />
            </Field>
            <Button icon={Upload} loading={busy} onClick={handleDeploy}>Deploy</Button>
          </div>
        </Card>

        <div className="space-y-6">
          <Card title="Call an export" icon={Play}>
            <div className="space-y-4">
              <Field label="Contract">
                <input value={callName} onChange={(e) => setCallName(e.target.value)} className={`${inputClass} font-mono`} list="wasm-names" />
                <datalist id="wasm-names">{contracts.map((c) => <option key={c} value={c} />)}</datalist>
              </Field>
              <div className="grid grid-cols-2 gap-3">
                <Field label="Function"><input value={func} onChange={(e) => setFunc(e.target.value)} className={`${inputClass} font-mono`} /></Field>
                <Field label="Argument (i32)"><input value={arg} onChange={(e) => setArg(e.target.value)} className={`${inputClass} font-mono`} /></Field>
              </div>
              <Button variant="secondary" icon={Play} loading={busy} onClick={handleCall} disabled={contracts.length === 0} title={contracts.length === 0 ? 'Deploy a module first' : undefined}>Call</Button>
              {result != null && (
                <div className="rounded-xl bg-slate-50 dark:bg-slate-950/40 border border-slate-100 dark:border-slate-800 p-4">
                  <div className="text-xs text-slate-500">{func}({arg})</div>
                  <div className="font-display text-2xl font-semibold tabular-nums text-slate-900 dark:text-white">{String(result)}</div>
                </div>
              )}
            </div>
          </Card>

          <Card title={`Deployed (${contracts.length})`} actions={<Button variant="ghost" size="sm" icon={RefreshCw} onClick={refresh} aria-label="Refresh" />}>
            {contracts.length === 0 ? (
              <EmptyState icon={Cpu} title="No modules yet">Deploy the sample module to try a call.</EmptyState>
            ) : (
              <div className="flex flex-wrap gap-2">
                {contracts.map((c) => (
                  <button key={c} type="button" onClick={() => setCallName(c)} className="h-8 px-3 rounded-lg border border-slate-200 dark:border-slate-800 font-mono text-xs text-slate-600 dark:text-slate-300 hover:border-blue-500/50">{c}</button>
                ))}
              </div>
            )}
          </Card>
        </div>
      </div>
      <Notice type={statusType === 'err' ? 'err' : statusType === 'ok' ? 'ok' : 'info'} className="mt-6">{status}</Notice>
    </Page>
  )
}
