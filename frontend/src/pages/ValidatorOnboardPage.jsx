import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { Shield, RefreshCw, CheckCircle2, AlertCircle, ExternalLink, BookOpen, Compass, Activity, Server, BellRing } from 'lucide-react'
import { signAndSubmit, waitForReceipt, walletAddress, getApiUrl, nblToWei } from '../lib/chain.js'
import { Page, PageHeader, Card, Field, Button, Notice, buttonClass, inputClass } from '../components/ui.jsx'

const GRAFANA_URL = 'http://localhost:3002/d/validator-onboarding/validator-onboarding'
const ONBOARDING_DOC = 'https://github.com/fewzfewz/scratchBlockchain/blob/main/docs/validator-onboarding.md'

const CHECKS = [
  { id: 'health', label: 'Node health', path: '/health' },
  { id: 'status', label: 'Chain status', path: '/status' },
  { id: 'validators', label: 'Validator set', path: '/validators' },
  { id: 'peers', label: 'Peer connectivity', path: '/peers' },
]

export default function ValidatorOnboardPage() {
  const [results, setResults] = useState({})
  const [loading, setLoading] = useState(false)
  const [registerMsg, setRegisterMsg] = useState('')
  const [stakeNbl, setStakeNbl] = useState('1')
  const [commission, setCommission] = useState('10')
  const [registering, setRegistering] = useState(false)

  const runChecks = async () => {
    setLoading(true)
    const next = {}
    for (const check of CHECKS) {
      try {
        const r = await window.fetch(`${getApiUrl()}${check.path}`)
        next[check.id] = { ok: r.ok, detail: r.ok ? 'OK' : `HTTP ${r.status}` }
      } catch (e) {
        next[check.id] = { ok: false, detail: e.message }
      }
    }
    setResults(next)
    setLoading(false)
  }

  useEffect(() => {
    runChecks()
  }, [])

  const registerValidator = async () => {
    const myAddr = walletAddress()
    if (!myAddr) {
      setRegisterMsg('Create a wallet first — a validator registers itself with a signed transaction.')
      return
    }
    const stake = parseFloat(stakeNbl)
    const rate = parseInt(commission, 10)
    if (!(stake >= 1)) { setRegisterMsg('Stake at least 1 NBL'); return }
    if (!(rate >= 0 && rate <= 100)) { setRegisterMsg('Commission is a percentage from 0 to 100'); return }
    setRegistering(true)
    try {
      // A signed staking transaction: bonds the stake from this wallet and
      // registers it as a validator once the transaction is in a block.
      const payload = Array.from(
        new TextEncoder().encode(
          JSON.stringify({ action: 'register_validator', stake: nblToWei(stakeNbl).toString(), commission_rate: rate }),
        ),
      )
      const { hash } = await signAndSubmit({ from: myAddr, to: null, valueWei: 0, payload })
      setRegisterMsg(`Registration submitted (tx ${hash.slice(0, 12)}…) — waiting for inclusion`)
      const receipt = await waitForReceipt(hash)
      setRegisterMsg(
        receipt?.success
          ? 'Registered: you are in the validator set from the next block'
          : `Registration failed: ${receipt?.revert_reason || 'not included yet'}`,
      )
      if (receipt?.success) runChecks()
    } catch (e) {
      setRegisterMsg('Error: ' + e.message)
    } finally {
      setRegistering(false)
    }
  }


  const steps = [
    { n: 1, title: 'Provision hardware', body: '4+ CPU cores, 16 GB RAM, 500 GB SSD. Open P2P 26656 and metrics 26657.' },
    { n: 2, title: 'Build the node', body: 'cargo build -p node --release' },
    { n: 3, title: 'Generate keys', body: 'Use scripts/setup-validator.sh or import an Ed25519 keypair.' },
    { n: 4, title: 'Configure genesis', body: 'Point --genesis at genesis.json and set data_dir in node config.' },
    { n: 5, title: 'Join the network', body: 'POST /connect_peer with bootstrap multiaddrs from deployment/local.' },
    { n: 6, title: 'Register stake', body: 'Submit a signed register_validator transaction (bonds min 1 NBL), or a delegate transaction, via POST /submit_tx.' },
    { n: 7, title: 'Monitor & alert', body: 'Watch Grafana Validator Onboarding dashboard; Alertmanager fires on validator down / stalled consensus.' },
  ]

  const myAddr = walletAddress()
  const regKind = /^Registered/.test(registerMsg) || /submitted/.test(registerMsg) ? 'ok' : /fail|Error|first|least|percentage/.test(registerMsg) ? 'err' : 'info'

  return (
    <Page width="max-w-5xl">
      <PageHeader icon={Shield} eyebrow="Validators" title="Run a validator"
        description="Check your node, register with a signed staking transaction, and watch it in the validator set."
        actions={<>
          <a href={ONBOARDING_DOC} target="_blank" rel="noopener noreferrer" className={buttonClass('secondary', 'sm')}><BookOpen className="w-4 h-4" /> Guide</a>
          <a href={GRAFANA_URL} target="_blank" rel="noopener noreferrer" className={buttonClass('secondary', 'sm')}><ExternalLink className="w-4 h-4" /> Grafana</a>
        </>} />

      <div className="grid lg:grid-cols-2 gap-6 mb-6 items-start">
        <Card title="Pre-flight checks" icon={Activity} description={getApiUrl()}
          actions={<Button variant="ghost" size="sm" icon={RefreshCw} loading={loading} onClick={runChecks}>Run checks</Button>}>
          <ul className="divide-y divide-slate-100 dark:divide-slate-800 -my-2">
            {CHECKS.map(({ id, label }) => {
              const r = results[id]
              return (
                <li key={id} className="flex items-center justify-between gap-3 py-2.5 text-sm">
                  <span className="text-slate-700 dark:text-slate-200">{label}</span>
                  {r ? (
                    <span className={`inline-flex items-center gap-1.5 text-xs ${r.ok ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-600 dark:text-rose-400'}`}>
                      {r.ok ? <CheckCircle2 className="w-4 h-4" /> : <AlertCircle className="w-4 h-4" />}{r.detail}
                    </span>
                  ) : <span className="text-xs text-slate-400">…</span>}
                </li>
              )
            })}
          </ul>
        </Card>

        <Card title="Register" icon={Shield} description="Bonds your stake from this wallet. Proposer turns are proportional to stake.">
          <div className="space-y-4">
            <Field label="Validator address" hint={myAddr ? 'Your wallet — validators register themselves.' : undefined}>
              <input readOnly value={myAddr || ''} placeholder="Create a wallet first" className={`${inputClass} font-mono text-xs`} />
            </Field>
            <div className="grid grid-cols-2 gap-3">
              <Field label="Stake (NBL)"><input value={stakeNbl} onChange={(e) => setStakeNbl(e.target.value)} inputMode="decimal" className={inputClass} /></Field>
              <Field label="Commission (%)"><input value={commission} onChange={(e) => setCommission(e.target.value)} inputMode="numeric" className={inputClass} /></Field>
            </div>
            <Notice type="warn">Only register if you will run a node with this key: until it joins, your proposer turns time out and slow the chain.</Notice>
            <Button icon={Shield} loading={registering} onClick={registerValidator} className="w-full">Register my wallet as validator</Button>
            <Notice type={regKind}>{registerMsg}</Notice>
          </div>
        </Card>
      </div>

      <Card title="Operator checklist" icon={Server}>
        <ol className="grid md:grid-cols-2 gap-x-8 gap-y-5">
          {steps.map(({ n, title, body }) => (
            <li key={n} className="flex gap-3">
              <span className="w-7 h-7 shrink-0 rounded-full bg-blue-500/10 text-blue-700 dark:text-blue-300 text-xs font-semibold flex items-center justify-center">{n}</span>
              <div>
                <p className="text-sm font-medium text-slate-900 dark:text-white">{title}</p>
                <p className="mt-0.5 text-sm text-slate-500">{body}</p>
              </div>
            </li>
          ))}
        </ol>
        <div className="mt-6 pt-4 border-t border-slate-100 dark:border-slate-800 flex flex-wrap items-center justify-between gap-3 text-xs text-slate-500">
          <span className="inline-flex items-center gap-1.5"><BellRing className="w-3.5 h-3.5" /> Alerts: ValidatorDown, ConsensusStalled, LowPeerCount → Alertmanager</span>
          <Link to="/explorer?tab=validators" className="inline-flex items-center gap-1.5 font-medium text-blue-600 dark:text-blue-300 hover:underline"><Compass className="w-3.5 h-3.5" /> Validator set</Link>
        </div>
      </Card>
    </Page>
  )
}
