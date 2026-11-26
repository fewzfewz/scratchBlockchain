import { useState, useEffect, useCallback } from 'react'
import { Cpu, Play, Upload, RefreshCw, Send } from 'lucide-react'
import { Page, PageHeader, Card, Field, Button, Notice, EmptyState, Badge, inputClass, textareaClass } from '../components/ui.jsx'
import {
  deployWasm, callWasm, listWasmContracts,
  loadWalletKeyPair, signAndSubmit, waitForReceipt, walletAddress, shorten,
} from '../lib/chain.js'

/** Sample modules: stateless double(i32) and stateful counter inc(i32)/get(i32). */
const SAMPLE_WASM = 'AGFzbQEAAAABBgFgAX8BfwMCAQAHCgEGZG91YmxlAAAKCQEHACAAQQJsCw=='
const COUNTER_WASM = 'AGFzbQEAAAABCwJgAX8Bf2ACf38AAiUCA2VudgtzdG9yYWdlX2dldAAAA2VudgtzdG9yYWdlX3NldAABAwMCAAAHDQIDaW5jAAIDZ2V0AAMKHwIWAQF/QQAQACAAaiIBGkEAIAEQASABCwYAIAAQAAs='

const PRESETS = {
  double: { name: 'double-demo', wasm: SAMPLE_WASM, func: 'double', arg: '21' },
  counter: { name: 'counter-demo', wasm: COUNTER_WASM, func: 'inc', arg: '1' },
}

function b64ToHex(b64) {
  const bin = atob(b64)
  return Array.from(bin, (ch) => ch.charCodeAt(0).toString(16).padStart(2, '0')).join('')
}

function strToHex(str) {
  return Array.from(new TextEncoder().encode(str), (b) => b.toString(16).padStart(2, '0')).join('')
}

function decodeReceiptI32(output) {
  if (!Array.isArray(output) || output.length < 4) return null
  const view = new DataView(new Uint8Array(output.slice(0, 4)).buffer)
  return view.getInt32(0, true)
}

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

  const hasWallet = Boolean(loadWalletKeyPair())
  const addr = walletAddress()

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
    const cleanName = name.trim()
    const cleanB64 = wasmB64.trim()
    if (!cleanName || cleanName.includes(':')) {
      show('Enter a valid contract name (no ":")', 'err')
      return
    }
    setBusy(true)
    show('Deploying WASM module…')
    try {
      if (hasWallet) {
        const payload = strToHex(`WASMDEPLOY:${cleanName}:`) + b64ToHex(cleanB64)
        const { hash } = await signAndSubmit({
          from: addr,
          to: null,
          valueWei: 0n,
          payload,
          gasLimit: 500000,
        })
        show(`Signed deploy submitted (${shorten(hash, 10, 8)}) — waiting for block…`)
        const receipt = await waitForReceipt(hash)
        if (receipt?.success === false) {
          throw new Error(receipt.revert_reason || 'WASM deploy failed on-chain')
        }
        await refresh()
        setCallName(cleanName)
        show(`Deployed "${cleanName}" via wallet tx ${shorten(hash, 10, 8)}`, 'ok')
        return
      }

      const res = await deployWasm(cleanName, cleanB64)
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

  const handleReadCall = async () => {
    setBusy(true)
    show('Simulating WASM call (read-only)…')
    try {
      const res = await callWasm(callName.trim(), func.trim(), parseInt(arg, 10) || 0)
      setResult(res.result)
      show(`Read result: ${res.result} (${res.gas_used ?? 0} fuel)`, 'ok')
    } catch (e) {
      setResult(null)
      show(e.message || 'Call failed', 'err')
    } finally {
      setBusy(false)
    }
  }

  const handleWriteCall = async () => {
    if (!hasWallet) {
      show('Create a wallet on /wallet to submit on-chain WASM transactions', 'err')
      return
    }
    setBusy(true)
    show('Submitting on-chain WASM transaction…')
    try {
      const intArg = parseInt(arg, 10) || 0
      const payload = strToHex(`WASM:${callName.trim()}:${func.trim()}:${intArg}`)
      const { hash } = await signAndSubmit({
        from: addr,
        to: null,
        valueWei: 0n,
        payload,
        gasLimit: 500000,
      })
      const receipt = await waitForReceipt(hash)
      if (receipt?.success === false) {
        throw new Error(receipt.revert_reason || 'WASM execution failed on-chain')
      }
      const out = decodeReceiptI32(receipt?.output)
      if (out != null) setResult(out)
      show(
        `On-chain tx ${shorten(hash, 10, 8)} confirmed${out != null ? ` — result: ${out}` : ''}`,
        'ok',
      )
    } catch (e) {
      show(e.message || 'On-chain call failed', 'err')
    } finally {
      setBusy(false)
    }
  }

  return (
    <Page width="max-w-5xl">
      <PageHeader icon={Cpu} eyebrow="WASM" title="WebAssembly contracts"
        description="Deploy WASM modules and run fuel-metered i32 functions with persistent env.storage_get / env.storage_set."
        actions={<Badge tone={hasWallet ? 'green' : 'amber'}>{hasWallet ? 'Wallet connected' : 'Dev mode'}</Badge>} />

      <div className="grid lg:grid-cols-2 gap-6 items-start">
        <Card title="Deploy a module" icon={Upload} description={hasWallet ? 'Signed with your wallet and stored on-chain.' : 'Uses node dev signer when no wallet is connected.'}>
          <div className="space-y-4">
            <div className="flex flex-wrap gap-2">
              <Button type="button" variant="secondary" size="sm" onClick={() => {
                setName(PRESETS.double.name); setWasmB64(PRESETS.double.wasm); setFunc(PRESETS.double.func); setArg(PRESETS.double.arg)
              }}>Preset: Double</Button>
              <Button type="button" variant="secondary" size="sm" onClick={() => {
                setName(PRESETS.counter.name); setWasmB64(PRESETS.counter.wasm); setFunc(PRESETS.counter.func); setArg(PRESETS.counter.arg)
              }}>Preset: Stateful Counter</Button>
            </div>
            <Field label="Contract name"><input value={name} onChange={(e) => setName(e.target.value)} className={`${inputClass} font-mono`} /></Field>
            <Field label="Module (base64)" hint="Supports env.storage_get(i32)->i32 and env.storage_set(i32,i32).">
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
              <div className="grid grid-cols-2 gap-2">
                <Button variant="secondary" icon={Play} loading={busy} onClick={handleReadCall} disabled={contracts.length === 0} title={contracts.length === 0 ? 'Deploy a module first' : undefined}>Read (RPC)</Button>
                <Button icon={Send} loading={busy} onClick={handleWriteCall} disabled={contracts.length === 0} title={contracts.length === 0 ? 'Deploy a module first' : undefined}>Write (On-chain)</Button>
              </div>
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
