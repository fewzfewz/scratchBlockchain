import { useState, useEffect } from 'react'
import { Shield, Zap, Send, RefreshCw } from 'lucide-react'
import { Page, PageHeader, Card, Field, Button, Notice, Tabs, Stat, inputClass } from '../components/ui.jsx'
import {
  getApiUrl, walletAddress, loadWalletKeyPair, fromHex, toHex,
  submitUserOperation, mevCommit, fetchPendingUserOps,
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

export default function AdvancedPage() {
  const [tab, setTab] = useState('aa')
  const [pendingOps, setPendingOps] = useState(0)
  const [status, setStatus] = useState('')
  const [statusType, setStatusType] = useState('')
  const [busy, setBusy] = useState(false)
  const [mevSecret, setMevSecret] = useState('')
  const [mevCommitment, setMevCommitment] = useState('')

  const addr = walletAddress()

  useEffect(() => {
    fetchPendingUserOps().then((n) => setPendingOps(n)).catch(() => setPendingOps(0))
  }, [status])

  const show = (msg, type = 'info') => {
    setStatus(msg)
    setStatusType(type)
  }

  const submitAa = async () => {
    const kp = loadWalletKeyPair()
    if (!kp || !addr) {
      show('Create a wallet on /wallet first', 'err')
      return
    }
    setBusy(true)
    show('Submitting UserOperation…')
    try {
      const senderBytes = Array.from(fromHex(addr.replace(/^0x/, '')))
      const res = await submitUserOperation({
        sender: senderBytes,
        nonce: 0,
        init_code: [],
        call_data: [1, 2, 3],
        verification_gas_limit: 100000,
        call_gas_limit: 200000,
        max_fee_per_gas: 1000,
        max_priority_fee_per_gas: 100,
        paymaster: null,
        paymaster_data: [],
        signature: Array(64).fill(0),
      })
      if (res.status?.startsWith('error')) throw new Error(res.status)
      show(`UserOperation accepted — hash ${res.hash?.slice(0, 14)}…`, 'ok')
    } catch (e) {
      show(e.message || 'Submit failed', 'err')
    } finally {
      setBusy(false)
    }
  }

  const doMevCommit = async () => {
    const kp = loadWalletKeyPair()
    if (!kp || !addr) {
      show('Create a wallet on /wallet first', 'err')
      return
    }
    setBusy(true)
    show('Submitting MEV commitment…')
    try {
      const secret = randomHex32()
      const txHash = randomHex32()
      const res = await mevCommit({
        txHash,
        secret,
        sender: addr,
        nonce: 0,
      })
      if (res.error) throw new Error(res.error)
      setMevSecret(secret)
      setMevCommitment(res.commitment)
      show(`Commitment stored — reveal with secret after block inclusion`, 'ok')
    } catch (e) {
      show(e.message || 'Commit failed', 'err')
    } finally {
      setBusy(false)
    }
  }

  return (
    <Page width="max-w-4xl">
      <PageHeader icon={Zap} eyebrow="Advanced" title="Account abstraction and MEV protection"
        description="Experimental transaction paths: ERC-4337-style UserOperations, and commit–reveal ordering against front-running." />

      <div className="grid grid-cols-2 gap-3 mb-8 max-w-md">
        <Stat label="Pending UserOps" value={pendingOps} />
        <Stat label="Node" value={getApiUrl().replace(/^https?:\/\//, '')} />
      </div>

      <Tabs tabs={TABS} value={tab} onChange={setTab} className="mb-6" />
      <Notice type={statusType === 'ok' ? 'ok' : statusType === 'err' ? 'err' : 'info'} className="mb-6">{status}</Notice>

      {tab === 'aa' && (
        <Card title="Submit a UserOperation" icon={Zap}
          description="A minimal UserOperation goes to POST /submit_user_operation; the bundler validates it and queues it for inclusion.">
          <Button icon={Send} loading={busy} onClick={submitAa}>Submit test UserOperation</Button>
        </Card>
      )}

      {tab === 'mev' && (
        <Card title="Commit, then reveal" icon={Shield}
          description="Commit to a hash of your transaction first; reveal it once the commitment is in a block, so nobody can see and front-run it in between.">
          <Button icon={Shield} loading={busy} onClick={doMevCommit}>Generate &amp; commit</Button>
          {mevCommitment && (
            <div className="mt-5 grid gap-3">
              <Field label="Commitment"><input readOnly value={mevCommitment} className={`${inputClass} font-mono text-xs`} /></Field>
              <Field label="Secret" hint="Keep it to reveal later."><input readOnly value={mevSecret} className={`${inputClass} font-mono text-xs`} /></Field>
            </div>
          )}
          <Button variant="ghost" size="sm" icon={RefreshCw} className="mt-4" onClick={() => fetchPendingUserOps().then(setPendingOps)}>Refresh pending ops</Button>
        </Card>
      )}
    </Page>
  )
}
