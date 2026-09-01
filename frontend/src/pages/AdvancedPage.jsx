import { useState, useEffect } from 'react'
import { Shield, Zap, Send, RefreshCw, CheckCircle, AlertCircle } from 'lucide-react'
import PageShell from '../components/PageShell.jsx'
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

  const inputCls =
    'w-full px-3 py-2.5 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/40'

  return (
    <PageShell>
      <div className="max-w-3xl mx-auto px-4 py-10 animate-fade-in">
        <header className="text-center mb-8">
          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-amber-500 to-orange-600 flex items-center justify-center text-white shadow-lg">
              <Shield className="w-6 h-6" />
            </div>
            <h1 className="text-3xl font-bold text-slate-900 dark:text-white">Advanced</h1>
          </div>
          <p className="text-slate-500 dark:text-slate-400">
            Account abstraction and MEV commit-reveal against {getApiUrl()}.
          </p>
          <p className="text-xs text-slate-500 mt-2">Pending UserOperations: {pendingOps}</p>
        </header>

        <div className="flex gap-1 p-1 rounded-2xl glass-strong mb-6">
          {TABS.map(({ id, label, icon: Icon }) => (
            <button key={id} type="button" onClick={() => setTab(id)}
              className={`flex-1 flex items-center justify-center gap-1.5 px-3 py-2.5 rounded-xl text-sm font-semibold transition-all ${
                tab === id ? 'bg-gradient-to-r from-amber-500 to-orange-600 text-white shadow-lg' : 'text-slate-500 hover:text-slate-700 dark:hover:text-slate-200'
              }`}>
              <Icon className="w-4 h-4" /> {label}
            </button>
          ))}
        </div>

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

        {tab === 'aa' && (
          <div className="glass-strong rounded-2xl p-6 space-y-4">
            <p className="text-sm text-slate-400">
              Submits a minimal ERC-4337-style UserOperation to <code className="text-xs">POST /submit_user_operation</code>.
              The bundler validates and queues it for inclusion.
            </p>
            <button type="button" onClick={submitAa} disabled={busy}
              className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-amber-600 hover:bg-amber-500 text-white text-sm font-semibold disabled:opacity-50">
              <Send className="w-4 h-4" /> Submit test UserOperation
            </button>
          </div>
        )}

        {tab === 'mev' && (
          <div className="glass-strong rounded-2xl p-6 space-y-4">
            <p className="text-sm text-slate-400">
              Commit-reveal flow: submit a hash commitment via <code className="text-xs">POST /mev/commit</code>,
              then reveal the transaction with <code className="text-xs">POST /mev/reveal</code> once included.
            </p>
            <button type="button" onClick={doMevCommit} disabled={busy}
              className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-orange-600 hover:bg-orange-500 text-white text-sm font-semibold disabled:opacity-50">
              <Shield className="w-4 h-4" /> Generate &amp; commit
            </button>
            {mevCommitment && (
              <div className="space-y-2 text-xs font-mono">
                <div>
                  <span className="text-slate-500">Commitment</span>
                  <input readOnly value={mevCommitment} className={inputCls} />
                </div>
                <div>
                  <span className="text-slate-500">Secret (save for reveal)</span>
                  <input readOnly value={mevSecret} className={inputCls} />
                </div>
              </div>
            )}
            <button type="button" onClick={() => fetchPendingUserOps().then(setPendingOps)}
              className="flex items-center gap-2 text-xs text-slate-500 hover:text-slate-300">
              <RefreshCw className="w-3 h-3" /> Refresh pending ops
            </button>
          </div>
        )}
      </div>
    </PageShell>
  )
}
