import { useState, useEffect } from 'react'
import nacl from 'tweetnacl'
import { Shield, Zap, Send, RefreshCw, Unlock } from 'lucide-react'
import { Page, PageHeader, Card, Field, Button, Notice, Tabs, Stat, inputClass } from '../components/ui.jsx'
import {
  getApiUrl, walletAddress, loadWalletKeyPair, fromHex, toHex,
  submitUserOperation, mevCommit, mevReveal, fetchPendingUserOps,
  fetchBalance, signTx, txHash, nblToWei, shorten,
} from '../lib/chain.js'

const TABS = [
  { id: 'aa', label: 'Account Abstraction', icon: Zap },
  { id: 'mev', label: 'MEV Commit-Reveal', icon: Shield },
]

function randomHex32() {
  const buf = new Uint8Array(32)
  crypto.getRandomValues(buf)
  return toHex(buf)
}

const isAddress = (a) => /^0x[a-fA-F0-9]{40}$/.test((a || '').trim())

export default function AdvancedPage() {
  const [tab, setTab] = useState('aa')
  const [pendingOps, setPendingOps] = useState(0)
  const [status, setStatus] = useState('')
  const [statusType, setStatusType] = useState('')
  const [busy, setBusy] = useState(false)

  // AA fields
  const [aaNonce, setAaNonce] = useState('0')
  const [aaCallData, setAaCallData] = useState('0x010203')
  const [aaGasLimit, setAaGasLimit] = useState('200000')
  const [aaPaymaster, setAaPaymaster] = useState('')

  // MEV commit-reveal fields
  const addr = walletAddress()
  const [mevTo, setMevTo] = useState(() => addr || '')
  const [mevAmount, setMevAmount] = useState('0.001')
  const [mevSecret, setMevSecret] = useState('')
  const [mevCommitment, setMevCommitment] = useState('')
  const [mevPendingTx, setMevPendingTx] = useState(null)

  useEffect(() => {
    fetchPendingUserOps().then((n) => setPendingOps(n)).catch(() => setPendingOps(0))
  }, [status])

  useEffect(() => {
    if (!addr) return
    setMevTo((cur) => cur || addr)
    fetchBalance(addr)
      .then((b) => setAaNonce(String(b.nonce ?? 0)))
      .catch(() => {})
  }, [addr])

  const show = (msg, type = 'info') => {
    setStatus(msg)
    setStatusType(type)
  }

  const submitAa = async (e) => {
    e.preventDefault()
    const kp = loadWalletKeyPair()
    if (!kp || !addr) {
      show('Create a wallet on /wallet first', 'err')
      return
    }
    const cleanData = aaCallData.trim().replace(/^0x/, '')
    if (cleanData.length % 2 !== 0 || (cleanData && !/^[0-9a-fA-F]+$/.test(cleanData))) {
      show('Invalid hex call_data', 'err')
      return
    }
    const paymasterTrimmed = aaPaymaster.trim()
    if (paymasterTrimmed && !isAddress(paymasterTrimmed)) {
      show('Invalid paymaster address (expected 0x + 40 hex chars)', 'err')
      return
    }
    setBusy(true)
    show('Signing and submitting UserOperation…')
    try {
      const senderBytes = Array.from(fromHex(addr.replace(/^0x/, '')))
      const callDataBytes = cleanData ? Array.from(fromHex(cleanData)) : []
      const paymasterBytes = paymasterTrimmed ? Array.from(fromHex(paymasterTrimmed.replace(/^0x/, ''))) : null
      const nonceNum = Number(aaNonce) || 0
      const gasNum = Number(aaGasLimit) || 200000
      const msg = new TextEncoder().encode(JSON.stringify({
        sender: senderBytes,
        nonce: nonceNum,
        call_data: callDataBytes,
        call_gas_limit: gasNum,
        paymaster: paymasterBytes,
      }))
      const signature = Array.from(nacl.sign.detached(msg, kp.secretKey))
      const res = await submitUserOperation({
        sender: senderBytes,
        nonce: nonceNum,
        init_code: [],
        call_data: callDataBytes,
        verification_gas_limit: 100000,
        call_gas_limit: gasNum,
        max_fee_per_gas: 1_000_000_000,
        max_priority_fee_per_gas: 100_000_000,
        paymaster: paymasterBytes,
        paymaster_data: [],
        signature,
      })
      if (res.status?.startsWith('error')) throw new Error(res.status)
      show(`Signed UserOperation accepted — hash ${res.hash?.slice(0, 14)}…`, 'ok')
    } catch (err) {
      show(err.message || 'Submit failed', 'err')
    } finally {
      setBusy(false)
    }
  }

  const doMevCommit = async (e) => {
    e.preventDefault()
    const kp = loadWalletKeyPair()
    if (!kp || !addr) {
      show('Create a wallet on /wallet first', 'err')
      return
    }
    const recipient = mevTo.trim()
    if (!isAddress(recipient)) {
      show('Enter a valid recipient address (0x + 40 hex chars)', 'err')
      return
    }
    setBusy(true)
    show('Signing transaction and submitting MEV commitment…')
    try {
      const [{ nonce }, statusRes] = await Promise.all([
        fetchBalance(addr),
        window.fetch(`${getApiUrl()}/status`).then((r) => r.json()).catch(() => ({ chain_id: 1 })),
      ])
      const tx = await signTx({
        keyPair: kp,
        to: recipient,
        valueWei: nblToWei(mevAmount || '0'),
        nonce: nonce || 0,
        chainId: statusRes.chain_id || 1,
      })
      const hash = toHex(await txHash(tx))
      const secret = randomHex32()
      const res = await mevCommit({
        txHash: hash,
        secret,
        sender: addr,
        nonce: nonce || 0,
      })
      if (res.error) throw new Error(res.error)
      setMevSecret(secret)
      setMevCommitment(res.commitment)
      setMevPendingTx(tx)
      show('Commitment stored — click Reveal & execute after the block delay', 'ok')
    } catch (err) {
      show(err.message || 'Commit failed', 'err')
    } finally {
      setBusy(false)
    }
  }

  const doMevReveal = async () => {
    if (!mevPendingTx || !mevSecret || !mevCommitment) return
    setBusy(true)
    show('Revealing committed transaction…')
    try {
      const res = await mevReveal({
        transaction: mevPendingTx,
        secret: mevSecret,
        commitment: mevCommitment,
      })
      if (res.error) throw new Error(res.error)
      if (res.status?.startsWith('error')) throw new Error(res.status)
      show(`Transaction revealed and queued — hash ${shorten(res.hash, 12, 8)}`, 'ok')
      setMevPendingTx(null)
    } catch (err) {
      show(err.message || 'Reveal failed', 'err')
    } finally {
      setBusy(false)
    }
  }

  return (
    <Page width="max-w-4xl">
      <PageHeader icon={Zap} eyebrow="Advanced" title="Account abstraction and MEV protection"
        description="ERC-4337-style UserOperations with wallet signatures, and full commit–reveal protection against front-running." />

      <div className="grid grid-cols-2 gap-3 mb-8 max-w-md">
        <Stat label="Pending UserOps" value={pendingOps} />
        <Stat label="Node" value={getApiUrl().replace(/^https?:\/\//, '')} />
      </div>

      <Tabs tabs={TABS} value={tab} onChange={setTab} className="mb-6" />
      <Notice type={statusType === 'ok' ? 'ok' : statusType === 'err' ? 'err' : 'info'} className="mb-6">{status}</Notice>

      {tab === 'aa' && (
        <Card title="Submit a UserOperation" icon={Zap}
          description="Signed with your wallet and queued in POST /submit_user_operation for bundler inclusion.">
          <form onSubmit={submitAa} className="space-y-4">
            <div className="grid sm:grid-cols-2 gap-3">
              <Field label="Nonce"><input value={aaNonce} onChange={(e) => setAaNonce(e.target.value)} type="number" min="0" className={inputClass} /></Field>
              <Field label="Call gas limit"><input value={aaGasLimit} onChange={(e) => setAaGasLimit(e.target.value)} type="number" min="21000" className={inputClass} /></Field>
            </div>
            <Field label="Calldata (hex)"><input value={aaCallData} onChange={(e) => setAaCallData(e.target.value)} placeholder="0x010203" className={`${inputClass} font-mono text-xs`} /></Field>
            <Field label="Paymaster (optional)" hint="Leave empty to pay gas from your sender account.">
              <input value={aaPaymaster} onChange={(e) => setAaPaymaster(e.target.value)} placeholder="0x…" className={`${inputClass} font-mono text-xs`} />
            </Field>
            <Button type="submit" icon={Send} loading={busy}>Sign &amp; submit UserOperation</Button>
          </form>
        </Card>
      )}

      {tab === 'mev' && (
        <Card title="Commit, then reveal" icon={Shield}
          description="Sign a real transfer, commit its hash + secret first, then reveal it after the delay so nobody can front-run it.">
          <form onSubmit={doMevCommit} className="space-y-4">
            <div className="grid sm:grid-cols-[2fr_1fr] gap-3">
              <Field label="Recipient"><input value={mevTo} onChange={(e) => setMevTo(e.target.value)} placeholder="0x…" className={`${inputClass} font-mono text-xs`} /></Field>
              <Field label="Amount (NBL)"><input value={mevAmount} onChange={(e) => setMevAmount(e.target.value)} inputMode="decimal" className={inputClass} /></Field>
            </div>
            <Button type="submit" icon={Shield} loading={busy}>Sign &amp; commit</Button>
          </form>
          {mevCommitment && (
            <div className="mt-5 grid gap-3 pt-4 border-t border-slate-100 dark:border-slate-800">
              <Field label="Commitment"><input readOnly value={mevCommitment} className={`${inputClass} font-mono text-xs`} /></Field>
              <Field label="Secret" hint="Used to reveal the committed transaction."><input readOnly value={mevSecret} className={`${inputClass} font-mono text-xs`} /></Field>
              {mevPendingTx && (
                <div>
                  <Button type="button" icon={Unlock} loading={busy} onClick={doMevReveal}>Reveal &amp; execute</Button>
                </div>
              )}
            </div>
          )}
          <Button variant="ghost" size="sm" icon={RefreshCw} className="mt-4" onClick={() => fetchPendingUserOps().then(setPendingOps)}>Refresh pending ops</Button>
        </Card>
      )}
    </Page>
  )
}
