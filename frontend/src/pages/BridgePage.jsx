import { useState, useEffect, useCallback } from 'react'
import {
  GitBranch, CheckCircle2, AlertCircle, Wallet, Settings, Loader2, ArrowRight, ShieldCheck,
  Circle, ArrowDownUp,
} from 'lucide-react'
import PageShell from '../components/PageShell.jsx'
import {
  BRIDGE_VAULT, encodeBridgePayload, fetchBalance, fetchTxHistory, signAndSubmit,
  loadWalletKeyPair, nblToWei, shorten, walletAddress, weiToNbl,
} from '../lib/chain.js'
import {
  connectMetaMask, connectLocalDevAccount, isLocalEthRpc, lockEthOnBridge, waitEthReceipt,
  createEthPublicClient, fetchBridgeStatus, requestNebulaMint, getEthRpcUrl, setEthRpcUrl,
  getBridgeAddress, setBridgeAddress, fetchEthBlockNumber, loadBridgeConfig, fetchLockAttestations,
} from '../lib/ethereum.js'

const input =
  'w-full h-11 px-3.5 rounded-xl bg-white dark:bg-slate-950/60 border border-slate-200 dark:border-slate-800 text-sm text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-blue-500/40 focus:border-blue-500/60 transition'
const primary =
  'inline-flex items-center justify-center gap-2 h-11 px-5 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 text-white text-sm font-semibold hover:opacity-95 disabled:opacity-50 disabled:cursor-not-allowed transition'
const secondary =
  'inline-flex items-center justify-center gap-2 h-11 px-4 rounded-xl glass text-sm font-semibold text-slate-900 dark:text-white disabled:opacity-50 transition'

function Field({ label, hint, children }) {
  return (
    <label className="block">
      <span className="block text-xs font-medium text-slate-600 dark:text-slate-300 mb-1.5">{label}</span>
      {children}
      {hint && <span className="block mt-1.5 text-xs text-slate-500">{hint}</span>}
    </label>
  )
}

function Step({ n, title, state, children }) {
  // state: 'todo' | 'active' | 'done'
  return (
    <li className="relative pl-12">
      <span className={`absolute left-0 top-0 w-8 h-8 rounded-full flex items-center justify-center text-sm font-semibold ${
        state === 'done' ? 'bg-emerald-500/15 text-emerald-500'
          : state === 'active' ? 'bg-blue-600 text-white'
          : 'bg-slate-100 dark:bg-slate-800 text-slate-400'}`}>
        {state === 'done' ? <CheckCircle2 className="w-5 h-5" /> : n}
      </span>
      <h3 className={`font-semibold leading-8 ${state === 'todo' ? 'text-slate-400' : 'text-slate-900 dark:text-white'}`}>{title}</h3>
      <div className="mt-3 mb-8">{children}</div>
    </li>
  )
}

function Notice({ type, children }) {
  if (!children) return null
  const ok = type === 'ok'
  return (
    <div className={`flex items-start gap-2 p-3 rounded-xl text-sm ${ok ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400' : 'bg-rose-500/10 text-rose-600 dark:text-rose-400'}`}>
      {ok ? <CheckCircle2 className="w-4 h-4 mt-0.5 shrink-0" /> : <AlertCircle className="w-4 h-4 mt-0.5 shrink-0" />}
      <span className="break-all">{children}</span>
    </div>
  )
}

/** Live view of one lock: confirmations → relayer attestations → minted. */
function RelayerProgress({ lockHash, nebulaRecipient, onDevAttest, devBusy }) {
  const [state, setState] = useState(null)
  const [since] = useState(() => Date.now())
  const [, tick] = useState(0)

  useEffect(() => {
    if (!lockHash) return undefined
    let active = true
    const poll = async () => {
      try {
        const d = await fetchLockAttestations(lockHash)
        if (active) setState(d)
      } catch { /* keep last */ }
      tick((t) => t + 1)
    }
    poll()
    const t = setInterval(poll, 1500)
    return () => { active = false; clearInterval(t) }
  }, [lockHash])

  if (!lockHash) return <p className="text-sm text-slate-500">Lock ETH above, or paste a lock transaction hash to track it.</p>

  const entry = state?.entries?.[0]
  const signed = new Set((entry?.attestations || []).map((a) => a.relayer.toLowerCase()))
  const threshold = state?.threshold || 0
  const minted = state?.minted
  const waitedLong = Date.now() - since > 20000

  return (
    <div className="rounded-2xl glass-strong p-5">
      <div className="flex items-center justify-between gap-3">
        <div className="font-mono text-xs text-slate-500 truncate">Lock {shorten(lockHash, 10, 8)}</div>
        <span className={`inline-flex items-center gap-1.5 h-6 px-2.5 rounded-full text-xs font-medium ${minted ? 'bg-emerald-500/15 text-emerald-500' : 'bg-blue-500/10 text-blue-600 dark:text-blue-300'}`}>
          {minted ? <><CheckCircle2 className="w-3.5 h-3.5" /> Minted on Nebula</> : <><Loader2 className="w-3.5 h-3.5 animate-spin" /> {signed.size}/{threshold || '…'} relayers</>}
        </span>
      </div>

      <ul className="mt-4 space-y-2">
        {(state?.relayers || []).map((r, i) => {
          const done = minted || signed.has(r.toLowerCase())
          return (
            <li key={r} className="flex items-center gap-3 text-sm">
              {done ? <CheckCircle2 className="w-4 h-4 text-emerald-500" /> : <Circle className="w-4 h-4 text-slate-300 dark:text-slate-600" />}
              <span className="text-slate-600 dark:text-slate-300">Relayer {i + 1}</span>
              <span className="font-mono text-xs text-slate-400">{shorten(r, 6, 4)}</span>
              <span className="ml-auto text-xs text-slate-500">{done ? 'attested' : 'waiting'}</span>
            </li>
          )
        })}
      </ul>

      {minted && nebulaRecipient && (
        <p className="mt-4 text-sm text-emerald-600 dark:text-emerald-400">Credited to {shorten(nebulaRecipient, 8, 6)} — check your wallet.</p>
      )}
      {!minted && waitedLong && onDevAttest && (
        <div className="mt-4 pt-4 border-t border-slate-200 dark:border-slate-800 flex flex-wrap items-center justify-between gap-3">
          <p className="text-xs text-slate-500">Relayers slow or offline? On the devnet this node holds one relayer key.</p>
          <button type="button" onClick={onDevAttest} disabled={devBusy} className={`${secondary} h-9`}>
            {devBusy ? <Loader2 className="w-4 h-4 animate-spin" /> : <ShieldCheck className="w-4 h-4" />} Attest from this node
          </button>
        </div>
      )}
    </div>
  )
}

export default function BridgePage() {
  const [tab, setTab] = useState('in')
  const [amount, setAmount] = useState('0.5')
  const [recipient, setRecipient] = useState('')
  const [vaultBal, setVaultBal] = useState(null)
  const [bridgeStatus, setBridgeStatus] = useState(null)
  const [locks, setLocks] = useState([])
  const [busy, setBusy] = useState(false)
  const [msg, setMsg] = useState({ type: '', text: '' })

  const [ethRpc, setEthRpc] = useState(getEthRpcUrl())
  const [bridgeAddr, setBridgeAddr] = useState(getBridgeAddress())
  const [ethConn, setEthConn] = useState(null)
  const [ethBlock, setEthBlock] = useState(null)
  const [lockHash, setLockHash] = useState('')
  const [trackInput, setTrackInput] = useState('')
  const [showSettings, setShowSettings] = useState(false)

  const nebulaAddr = walletAddress()
  const ok = (text) => setMsg({ type: 'ok', text })
  const err = (text) => setMsg({ type: 'err', text })

  const refresh = useCallback(async () => {
    try {
      const vault = await BRIDGE_VAULT()
      const [status, vb] = await Promise.all([fetchBridgeStatus().catch(() => null), fetchBalance(vault)])
      setBridgeStatus(status)
      setVaultBal(weiToNbl(vb.balance))
      if (nebulaAddr) {
        const hist = await fetchTxHistory(nebulaAddr, 30)
        setLocks((hist.transactions || []).filter((t) => t.to?.toLowerCase() === vault.toLowerCase()))
      }
      setEthBlock((await fetchEthBlockNumber(ethRpc).catch(() => null))?.toString() ?? null)
    } catch { /* offline */ }
  }, [nebulaAddr, ethRpc])

  useEffect(() => {
    if (nebulaAddr) setRecipient((r) => r || nebulaAddr)
    loadBridgeConfig().then((cfg) => {
      if (cfg?.ethRpcUrl) setEthRpc(cfg.ethRpcUrl)
      if (cfg?.bridge) setBridgeAddr(cfg.bridge)
    })
    refresh()
    const t = setInterval(refresh, 10000)
    return () => clearInterval(t)
  }, [refresh, nebulaAddr])

  useEffect(() => { setMsg({ type: '', text: '' }) }, [tab])

  const connect = async (dev) => {
    try {
      const conn = dev ? await connectLocalDevAccount() : await connectMetaMask()
      setEthConn({ ...conn, dev })
      ok(`${dev ? 'Local Hardhat account' : 'MetaMask'} connected: ${shorten(conn.address)}`)
    } catch (e) { err(e.message) }
  }

  const lockEth = async () => {
    const recip = recipient.trim()
    if (!/^0x[a-fA-F0-9]{40}$/.test(recip)) return err('Enter the Nebula address that should receive NBL')
    if (!bridgeAddr) return err('Set the Bridge.sol address under Settings')
    setBusy(true)
    try {
      const hash = await lockEthOnBridge({
        bridgeAddress: bridgeAddr, amountEth: amount, recipientNebula: recip,
        walletClient: ethConn.walletClient, account: ethConn.account,
      })
      await waitEthReceipt(createEthPublicClient(ethRpc), hash)
      setLockHash(hash)
      ok('ETH locked — relayers are picking it up')
    } catch (e) { err(e.shortMessage || e.message) } finally { setBusy(false) }
  }

  const devAttest = async () => {
    setBusy(true)
    try {
      const r = await requestNebulaMint({ recipient: recipient.trim(), ethTxHash: lockHash })
      ok(r.status === 'submitted' ? 'Threshold reached — mint submitted' : `Attested (${r.attestations}/${r.threshold})`)
    } catch (e) { err(e.message) } finally { setBusy(false) }
  }

  const lockNbl = async () => {
    if (!loadWalletKeyPair()) return err('Create a Nebula wallet first')
    const dest = recipient.trim()
    if (!/^0x[a-fA-F0-9]{40}$/.test(dest)) return err('Enter the Ethereum address that should receive ETH')
    setBusy(true)
    try {
      const vault = await BRIDGE_VAULT()
      const { hash } = await signAndSubmit({
        from: nebulaAddr, to: vault, valueWei: nblToWei(amount),
        payload: encodeBridgePayload('ethereum', dest), gasLimit: 120000,
      })
      ok(`Nebula lock tx: ${shorten(hash, 10, 8)} — the unlock relayer releases ETH on Ethereum`)
      refresh()
    } catch (e) { err(e.message) } finally { setBusy(false) }
  }

  const relayerSet = bridgeStatus?.relayer_count ? `${bridgeStatus.relayer_threshold}-of-${bridgeStatus.relayer_count}` : '—'

  return (
    <PageShell>
      <div className="max-w-5xl mx-auto px-4 py-12">
        <header className="flex flex-wrap items-end justify-between gap-6 mb-8">
          <div>
            <div className="flex items-center gap-2 text-sm font-medium text-blue-600 dark:text-blue-300"><GitBranch className="w-4 h-4" /> Bridge</div>
            <h1 className="mt-2 text-3xl sm:text-4xl font-semibold text-slate-900 dark:text-white">Move value between Ethereum and Nebula</h1>
            <p className="mt-2 text-slate-500 max-w-2xl">
              Locks on Ethereum are minted on Nebula once {bridgeStatus?.relayer_threshold ?? 'enough'} independent relayers have signed for them — consensus checks every signature.
            </p>
          </div>
          <button type="button" onClick={() => setShowSettings((s) => !s)} className={`${secondary} h-9`}>
            <Settings className="w-4 h-4" /> Settings
          </button>
        </header>

        {showSettings && (
          <div className="glass-strong rounded-2xl p-5 mb-6 grid sm:grid-cols-2 gap-4">
            <Field label="Ethereum RPC URL"><input value={ethRpc} onChange={(e) => setEthRpc(e.target.value)} className={`${input} font-mono`} /></Field>
            <Field label="Bridge.sol address"><input value={bridgeAddr} onChange={(e) => setBridgeAddr(e.target.value)} className={`${input} font-mono`} /></Field>
            <div className="sm:col-span-2 flex justify-end">
              <button type="button" className={primary} onClick={() => { setEthRpcUrl(ethRpc); setBridgeAddress(bridgeAddr); setShowSettings(false); refresh() }}>Save</button>
            </div>
          </div>
        )}

        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 mb-8">
          {[
            ['Relayer set', relayerSet],
            ['Locked in vault', vaultBal != null ? `${vaultBal} NBL` : '—'],
            ['Ethereum block', ethBlock ?? '—'],
            ['ETH node', bridgeStatus?.eth_rpc_configured ? 'Connected' : '—'],
          ].map(([k, v]) => (
            <div key={k} className="rounded-2xl glass-strong p-4">
              <div className="text-[11px] uppercase tracking-wider text-slate-500">{k}</div>
              <div className="mt-1 font-display text-lg font-semibold tabular-nums text-slate-900 dark:text-white">{v}</div>
            </div>
          ))}
        </div>

        <div role="tablist" className="inline-flex p-1 rounded-xl bg-slate-100 dark:bg-slate-900 border border-slate-200 dark:border-slate-800 mb-8">
          {[['in', 'ETH → Nebula'], ['out', 'Nebula → ETH']].map(([id, label]) => (
            <button key={id} role="tab" aria-selected={tab === id} type="button" onClick={() => setTab(id)}
              className={`h-9 px-4 rounded-lg text-sm font-medium transition ${tab === id ? 'bg-white dark:bg-slate-800 text-slate-900 dark:text-white shadow-sm' : 'text-slate-500 hover:text-slate-900 dark:hover:text-white'}`}>
              {label}
            </button>
          ))}
        </div>

        <div className="grid lg:grid-cols-[1.4fr_1fr] gap-8">
          <div>
            {tab === 'in' ? (
              <ol>
                <Step n={1} title="Connect an Ethereum account" state={ethConn ? 'done' : 'active'}>
                  {ethConn ? (
                    <div className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-300">
                      <Wallet className="w-4 h-4" /> <span className="font-mono">{shorten(ethConn.address, 8, 6)}</span>
                      <span className="text-slate-400">· {ethConn.dev ? 'local Hardhat account' : 'MetaMask'}</span>
                    </div>
                  ) : (
                    <div className="flex flex-wrap gap-2">
                      <button type="button" className={primary} onClick={() => connect(false)}><Wallet className="w-4 h-4" /> Connect MetaMask</button>
                      {isLocalEthRpc(ethRpc) && (
                        <button type="button" className={secondary} onClick={() => connect(true)} title="Hardhat's default account #0 — local chains only">Use local Hardhat account</button>
                      )}
                    </div>
                  )}
                </Step>
                <Step n={2} title="Lock ETH on Ethereum" state={lockHash ? 'done' : ethConn ? 'active' : 'todo'}>
                  <div className="grid sm:grid-cols-[160px_1fr] gap-3">
                    <Field label="Amount (ETH)"><input value={amount} onChange={(e) => setAmount(e.target.value)} inputMode="decimal" className={input} /></Field>
                    <Field label="Nebula recipient" hint={recipient && recipient === nebulaAddr ? 'Your Nebula wallet' : undefined}>
                      <input value={recipient} onChange={(e) => setRecipient(e.target.value)} placeholder="0x…" className={`${input} font-mono`} />
                    </Field>
                  </div>
                  <button type="button" className={`${primary} mt-4`} disabled={!ethConn || busy} onClick={lockEth}>
                    {busy ? <Loader2 className="w-4 h-4 animate-spin" /> : <ArrowRight className="w-4 h-4" />} Lock {amount || 0} ETH
                  </button>
                </Step>
                <Step n={3} title="Relayers mint NBL on Nebula" state={lockHash ? 'active' : 'todo'}>
                  <RelayerProgress lockHash={lockHash} nebulaRecipient={recipient} onDevAttest={devAttest} devBusy={busy} />
                  <div className="mt-3 flex gap-2">
                    <input value={trackInput} onChange={(e) => setTrackInput(e.target.value)} placeholder="Track an existing lock: 0x… tx hash" className={`${input} h-9 font-mono text-xs`} />
                    <button type="button" className={`${secondary} h-9`} onClick={() => trackInput.trim() && setLockHash(trackInput.trim())}>Track</button>
                  </div>
                </Step>
              </ol>
            ) : (
              <div className="space-y-4">
                <div className="grid sm:grid-cols-[160px_1fr] gap-3">
                  <Field label="Amount (NBL)"><input value={amount} onChange={(e) => setAmount(e.target.value)} inputMode="decimal" className={input} /></Field>
                  <Field label="Ethereum recipient"><input value={recipient} onChange={(e) => setRecipient(e.target.value)} placeholder="0x…" className={`${input} font-mono`} /></Field>
                </div>
                <button type="button" className={primary} disabled={busy} onClick={lockNbl}>
                  {busy ? <Loader2 className="w-4 h-4 animate-spin" /> : <ArrowDownUp className="w-4 h-4" />} Lock {amount || 0} NBL on Nebula
                </button>
                {locks.length > 0 && (
                  <div className="rounded-2xl glass-strong p-5">
                    <h3 className="font-semibold text-slate-900 dark:text-white mb-3">Your locks</h3>
                    <ul className="space-y-2">
                      {locks.map((t) => (
                        <li key={t.hash} className="flex items-center justify-between text-sm">
                          <span className="font-mono text-slate-500">{shorten(t.hash, 10, 8)}</span>
                          <span className="tabular-nums text-slate-900 dark:text-white">{weiToNbl(String(t.value))} NBL</span>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </div>
            )}
            <div className="mt-2"><Notice type={msg.type}>{msg.text}</Notice></div>
          </div>

          <aside className="space-y-4">
            <div className="rounded-2xl glass-strong p-5">
              <h3 className="font-semibold text-slate-900 dark:text-white">How the bridge is secured</h3>
              <ul className="mt-3 space-y-3 text-sm text-slate-500">
                <li className="flex gap-2"><ShieldCheck className="w-4 h-4 mt-0.5 text-cyan-500 shrink-0" />Each relayer watches Ethereum on its own and signs only locks it has seen confirmed.</li>
                <li className="flex gap-2"><ShieldCheck className="w-4 h-4 mt-0.5 text-cyan-500 shrink-0" />A mint needs {relayerSet} relayer signatures; every validator re-checks them.</li>
                <li className="flex gap-2"><ShieldCheck className="w-4 h-4 mt-0.5 text-cyan-500 shrink-0" />The lock's transaction hash is recorded on chain, so it can never mint twice.</li>
              </ul>
            </div>
            <p className="text-xs text-slate-500 px-1">
              Deploy Bridge.sol with <code className="font-mono">cd interop && npx hardhat run scripts/deploy.js</code>; relayers run as <code className="font-mono">nebula-relayer</code>.
            </p>
          </aside>
        </div>
      </div>
    </PageShell>
  )
}
