import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { FileCode, Fuel, Send, Eye } from 'lucide-react'
import { Page, PageHeader, Card, Field, Button, Notice, buttonClass, inputClass, textareaClass } from '../components/ui.jsx'
import {
  callContract, encodeBalanceOf, estimateGas, loadWalletKeyPair,
  scanDeployedContracts, signAndSubmit, waitForReceipt, walletAddress,
  decodeUint256, weiToNbl, shorten, SELECTORS, encodeTransfer, encodeMint, encodeOwnerOf, nblToWei,
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

  // Common ERC20 / ERC721 calls: pick one, fill its arguments, and the
  // calldata is encoded for you.
  const [fn, setFn] = useState('balanceOf')
  const [argA, setArgA] = useState('')
  const [argB, setArgB] = useState('')
  const FUNCTIONS = {
    balanceOf: { args: [['Holder', 'address']], encode: () => encodeBalanceOf(argA || from), read: true },
    totalSupply: { args: [], encode: () => SELECTORS.totalSupply, read: true },
    transfer: { args: [['To', 'address'], ['Amount (tokens)', 'number']], encode: () => encodeTransfer(argA, nblToWei(argB || '0')) },
    mint: { args: [['To', 'address'], ['Token id / amount', 'number']], encode: () => encodeMint(argA || from, argB || '1') },
    ownerOf: { args: [['Token id', 'number']], encode: () => encodeOwnerOf(argA || '1'), read: true },
  }
  const presets = Object.keys(FUNCTIONS).map((label) => ({ label, data: FUNCTIONS[label].encode }))
  const chooseFn = (name) => {
    setFn(name)
    setArgA(''); setArgB('')
    setEstimate(null); setReadResult(null)
    try { setCalldata(FUNCTIONS[name].encode()) } catch { /* filled in later */ }
  }
  const applyArgs = () => { try { setCalldata(FUNCTIONS[fn].encode()) } catch (e) { setStatus(e.message); setStatusType('err') } }

  const runEstimate = async () => {
    setStatus('')
    try {
      const d = await estimateGas(from, contract, calldata, value)
      setEstimate(d)
      if (d.error) {
        setStatus(`This call would fail: ${d.error}`)
        setStatusType('err')
        return
      }
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
      if (est.error) throw new Error(`This call would fail: ${est.error}`)
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

  const spec = FUNCTIONS[fn]

  return (
    <Page width="max-w-5xl">
      <PageHeader icon={FileCode} eyebrow="Contracts" title="Interact with a contract"
        description="Read with a free simulated call, or write with a signed transaction from your wallet."
        actions={<Link to="/deploy" className={buttonClass('secondary', 'sm')}>Deploy a contract</Link>} />

      <div className="grid lg:grid-cols-[1fr_1.3fr] gap-6">
        <Card title="Target">
          <div className="space-y-4">
            {contracts.length > 0 && (
              <div>
                <div className="text-xs font-medium text-slate-600 dark:text-slate-300 mb-1.5">Your contracts</div>
                <div className="flex flex-wrap gap-2">
                  {contracts.map((c) => (
                    <button key={c.address} type="button" onClick={() => setContract(c.address)}
                      className={`h-8 px-3 rounded-lg text-xs font-medium border transition ${contract === c.address ? 'border-blue-500/50 bg-blue-500/10 text-blue-700 dark:text-blue-300' : 'border-slate-200 dark:border-slate-800 text-slate-500 hover:text-slate-900 dark:hover:text-white'}`}>
                      {c.name || shorten(c.address)}
                    </button>
                  ))}
                </div>
              </div>
            )}
            <Field label="Contract address">
              <input value={contract} onChange={(e) => setContract(e.target.value)} placeholder="0x…" className={`${inputClass} font-mono`} />
            </Field>
            <Field label="Caller" hint="Your wallet by default.">
              <input value={from} onChange={(e) => setFrom(e.target.value)} placeholder="0x…" className={`${inputClass} font-mono`} />
            </Field>
            <Field label="Value (wei)">
              <input value={value} onChange={(e) => setValue(e.target.value)} className={`${inputClass} font-mono`} />
            </Field>
          </div>
        </Card>

        <Card title="Call">
          <div className="space-y-4">
            <div>
              <div className="text-xs font-medium text-slate-600 dark:text-slate-300 mb-1.5">Function</div>
              <div className="flex flex-wrap gap-2">
                {presets.map((p) => (
                  <button key={p.label} type="button" onClick={() => chooseFn(p.label)}
                    className={`h-8 px-3 rounded-lg font-mono text-xs border transition ${fn === p.label ? 'border-blue-500/50 bg-blue-500/10 text-blue-700 dark:text-blue-300' : 'border-slate-200 dark:border-slate-800 text-slate-500 hover:text-slate-900 dark:hover:text-white'}`}>
                    {p.label}
                  </button>
                ))}
              </div>
            </div>
            {spec?.args.length > 0 && (
              <div className="grid sm:grid-cols-2 gap-3 items-end">
                {spec.args.map(([label, kind], i) => (
                  <Field key={label} label={label}>
                    <input value={i === 0 ? argA : argB} onChange={(e) => (i === 0 ? setArgA : setArgB)(e.target.value)}
                      placeholder={kind === 'address' ? '0x…' : '0'} className={`${inputClass} ${kind === 'address' ? 'font-mono' : ''}`} />
                  </Field>
                ))}
                <Button variant="secondary" onClick={applyArgs} className="sm:col-span-2">Encode arguments</Button>
              </div>
            )}
            <Field label="Calldata (hex)">
              <textarea value={calldata} onChange={(e) => setCalldata(e.target.value)} rows={3} className={`${textareaClass} font-mono text-xs`} />
            </Field>
            <div className="grid sm:grid-cols-3 gap-2">
              <Button variant="secondary" icon={Eye} onClick={runRead}>Read (call)</Button>
              <Button variant="secondary" icon={Fuel} onClick={runEstimate}>Estimate</Button>
              <Button icon={Send} loading={submitting} onClick={sendWrite}>Write</Button>
            </div>
            {readResult && <pre className="text-xs font-mono bg-slate-50 dark:bg-slate-950/50 border border-slate-100 dark:border-slate-800 rounded-xl p-3 whitespace-pre-wrap break-all">{readResult}</pre>}
            {estimate && !estimate.error && (
              <p className="text-xs text-slate-500">Gas {Number(estimate.estimated_gas).toLocaleString()} · cost ≈ {weiToNbl(String(estimate.total_cost_estimate))} NBL</p>
            )}
            <Notice type={statusType === 'err' ? 'err' : 'ok'}>{status}</Notice>
          </div>
        </Card>
      </div>
    </Page>
  )
}
