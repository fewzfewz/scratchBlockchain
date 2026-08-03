import { useState, useEffect, useCallback } from 'react'
import { GitBranch, Shield, CheckCircle, AlertCircle, RefreshCw, Wallet, Settings, ExternalLink } from 'lucide-react'
import PageShell from '../components/PageShell.jsx'
import AnimatedSection from '../components/AnimatedSection.jsx'
import {
  BRIDGE_VAULT, encodeBridgePayload,
  fetchBalance, fetchTxHistory, signAndSubmit, loadWalletKeyPair,
  nblToWei, shorten, walletAddress, weiToNbl,
} from '../lib/chain.js'
import {
  connectMetaMask, lockEthOnBridge, waitEthReceipt, createEthPublicClient,
  fetchBridgeStatus, requestNebulaMint, getEthRpcUrl, setEthRpcUrl,
  getBridgeAddress, setBridgeAddress, fetchEthBlockNumber, loadBridgeConfig,
} from '../lib/ethereum.js'

export default function BridgePage() {
  const [tab, setTab] = useState('eth-nebula')
  const [amount, setAmount] = useState('1')
  const [recipient, setRecipient] = useState('')
  const [vaultBal, setVaultBal] = useState('—')
  const [bridgeStatus, setBridgeStatus] = useState(null)
  const [locks, setLocks] = useState([])
  const [loading, setLoading] = useState(true)
  const [submitting, setSubmitting] = useState(false)
  const [msg, setMsg] = useState('')
  const [msgType, setMsgType] = useState('')

  const [ethRpc, setEthRpc] = useState(getEthRpcUrl())
  const [bridgeAddr, setBridgeAddr] = useState(getBridgeAddress())
  const [ethAccount, setEthAccount] = useState(null)
  const [ethBlock, setEthBlock] = useState(null)
  const [lastEthTx, setLastEthTx] = useState('')
  const [showSettings, setShowSettings] = useState(false)

  const nebulaAddr = walletAddress()

  const refresh = useCallback(async () => {
    setLoading(true)
    try {
      const vault = await BRIDGE_VAULT()
      const [status, vb] = await Promise.all([
        fetchBridgeStatus().catch(() => null),
        fetchBalance(vault),
      ])
      setBridgeStatus(status)
      setVaultBal(weiToNbl(vb.balance))

      if (nebulaAddr) {
        const hist = await fetchTxHistory(nebulaAddr, 30)
        setLocks((hist.transactions || []).filter((t) => t.to?.toLowerCase() === vault.toLowerCase()))
        if (!recipient) setRecipient(nebulaAddr)
      }

      try {
        const block = await fetchEthBlockNumber(ethRpc)
        setEthBlock(block?.toString())
      } catch {
        setEthBlock(null)
      }
    } catch {
      setVaultBal('0')
    } finally {
      setLoading(false)
    }
  }, [nebulaAddr, ethRpc, recipient])

  useEffect(() => {
    loadBridgeConfig().then((cfg) => {
      if (cfg?.ethRpcUrl) setEthRpc(cfg.ethRpcUrl)
      if (cfg?.bridge) setBridgeAddr(cfg.bridge)
    })
    refresh()
  }, [refresh])

  const saveSettings = () => {
    setEthRpcUrl(ethRpc)
    setBridgeAddress(bridgeAddr)
    setShowSettings(false)
    refresh()
  }

  const connectEth = async () => {
    try {
      const { address } = await connectMetaMask()
      setEthAccount(address)
      setMsg(`MetaMask connected: ${shorten(address)}`)
      setMsgType('ok')
    } catch (e) {
      setMsg(e.message)
      setMsgType('err')
    }
  }

  const lockOnNebula = async () => {
    if (!loadWalletKeyPair()) {
      setMsg('Create a Nebula wallet on /wallet first')
      setMsgType('err')
      return
    }
    const recip = recipient.trim()
    if (!/^0x[a-fA-F0-9]{40}$/.test(recip)) {
      setMsg('Invalid destination address')
      setMsgType('err')
      return
    }
    setSubmitting(true)
    try {
      const vault = await BRIDGE_VAULT()
      const payload = encodeBridgePayload('ethereum', recip)
      const wei = nblToWei(amount)
      const { hash } = await signAndSubmit({
        from: nebulaAddr,
        to: vault,
        valueWei: wei,
        payload,
        gasLimit: 120000,
      })
      setMsg(`Nebula lock tx: ${shorten(hash, 10, 8)}`)
      setMsgType('ok')
      refresh()
    } catch (e) {
      setMsg(e.message)
      setMsgType('err')
    } finally {
      setSubmitting(false)
    }
  }

  const lockOnEthereum = async () => {
    if (!ethAccount) {
      setMsg('Connect MetaMask first')
      setMsgType('err')
      return
    }
    if (!bridgeAddr) {
      setMsg('Set Bridge.sol address in settings (deploy via interop/scripts/deploy.js)')
      setMsgType('err')
      return
    }
    setSubmitting(true)
    try {
      const { walletClient, address } = await connectMetaMask()
      const recip = recipient.trim() || nebulaAddr
      if (!/^0x[a-fA-F0-9]{40}$/.test(recip)) throw new Error('Invalid Nebula recipient')

      const hash = await lockEthOnBridge({
        bridgeAddress: bridgeAddr,
        amountEth: amount,
        recipientNebula: recip,
        walletClient,
        account: address,
      })
      setLastEthTx(hash)
      const publicClient = createEthPublicClient(ethRpc)
      await waitEthReceipt(publicClient, hash)
      setMsg(`ETH lock confirmed: ${shorten(hash, 10, 8)}`)
      setMsgType('ok')
    } catch (e) {
      setMsg(e.message)
      setMsgType('err')
    } finally {
      setSubmitting(false)
    }
  }

  const claimOnNebula = async () => {
    if (!lastEthTx) {
      setMsg('Lock on Ethereum first, or paste an ETH tx hash below')
      setMsgType('err')
      return
    }
    const recip = recipient.trim() || nebulaAddr
    setSubmitting(true)
    try {
      const wei = nblToWei(amount)
      const result = await requestNebulaMint({
        recipient: recip,
        amount: wei.toString(),
        ethTxHash: lastEthTx,
        ethRpcUrl: ethRpc,
        ethBridgeAddress: bridgeAddr,
      })
      setMsg(`Minted on Nebula — balance ${weiToNbl(result.balance)} NBL`)
      setMsgType('ok')
      refresh()
    } catch (e) {
      setMsg(e.message)
      setMsgType('err')
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <PageShell variant="default">
      <div className="max-w-3xl mx-auto px-4 py-10">
        <AnimatedSection className="text-center mb-8">
          <div className="inline-flex items-center justify-center w-14 h-14 rounded-2xl bg-gradient-to-br from-emerald-500 to-teal-600 text-white mb-4 shadow-lg shadow-emerald-500/30">
            <GitBranch className="w-7 h-7" />
          </div>
          <h1 className="text-3xl font-bold mb-2">Cross-Chain Bridge</h1>
          <p className="text-slate-500">Ethereum Bridge.sol ↔ Nebula relayer mint · full two-way flow</p>
        </AnimatedSection>

        <AnimatedSection delay={80} className="glass-strong rounded-2xl p-4 mb-6">
          <button type="button" onClick={() => setShowSettings(!showSettings)} className="flex items-center gap-2 text-sm font-semibold text-slate-600 dark:text-slate-300 w-full">
            <Settings className="w-4 h-4" /> Bridge configuration
          </button>
          {showSettings && (
            <div className="mt-4 space-y-3 pt-4 border-t border-slate-200/50 dark:border-slate-700/50">
              <div>
                <label className="text-xs text-slate-500">ETH RPC URL (Hardhat / Sepolia)</label>
                <input value={ethRpc} onChange={(e) => setEthRpc(e.target.value)} placeholder="http://127.0.0.1:8545"
                  className="w-full mt-1 px-3 py-2 rounded-xl font-mono text-sm bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600" />
              </div>
              <div>
                <label className="text-xs text-slate-500">Bridge.sol address (from interop deploy)</label>
                <input value={bridgeAddr} onChange={(e) => setBridgeAddr(e.target.value)} placeholder="0x…"
                  className="w-full mt-1 px-3 py-2 rounded-xl font-mono text-sm bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600" />
              </div>
              <button type="button" onClick={saveSettings} className="px-4 py-2 rounded-xl bg-emerald-600 text-white text-sm font-semibold">Save</button>
              <p className="text-xs text-slate-400">Set <code className="font-mono">ETH_RPC_URL</code> on the Nebula node for server-side tx verification.</p>
            </div>
          )}
        </AnimatedSection>

        <div className="grid sm:grid-cols-4 gap-3 mb-6">
          {[
            { label: 'Vault', val: `${vaultBal} NBL` },
            { label: 'Validators', val: bridgeStatus?.validators_count ?? '—' },
            { label: 'ETH block', val: ethBlock ?? '—' },
            { label: 'Mints', val: bridgeStatus?.processed_mints ?? 0 },
          ].map(({ label, val }) => (
            <div key={label} className="glass-strong rounded-2xl p-4 text-center card-hover-3d">
              <p className="text-xs text-slate-500">{label}</p>
              <p className="font-mono font-bold text-sm">{val}</p>
            </div>
          ))}
        </div>

        <div className="flex gap-2 mb-6">
          {[
            { id: 'eth-nebula', label: 'ETH → Nebula' },
            { id: 'nebula-eth', label: 'Nebula → ETH' },
          ].map(({ id, label }) => (
            <button key={id} type="button" onClick={() => setTab(id)}
              className={`flex-1 py-2.5 rounded-xl text-sm font-semibold transition-all ${tab === id ? 'bg-emerald-600 text-white shadow-lg' : 'glass text-slate-500'}`}>
              {label}
            </button>
          ))}
        </div>

        <AnimatedSection delay={120} className="glass-strong rounded-2xl p-6 space-y-4 mb-6">
          <input type="number" step="0.01" value={amount} onChange={(e) => setAmount(e.target.value)}
            className="w-full px-4 py-3 rounded-xl font-mono bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600" placeholder="Amount" />
          <input value={recipient} onChange={(e) => setRecipient(e.target.value)}
            className="w-full px-4 py-3 rounded-xl font-mono text-sm bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600" placeholder="Recipient on destination chain (0x…)" />

          {tab === 'eth-nebula' ? (
            <>
              <button type="button" onClick={connectEth} className="w-full py-2.5 rounded-xl border border-amber-500/50 text-amber-600 font-semibold flex items-center justify-center gap-2">
                <Wallet className="w-4 h-4" /> {ethAccount ? shorten(ethAccount) : 'Connect MetaMask'}
              </button>
              <button type="button" disabled={submitting} onClick={lockOnEthereum}
                className="w-full py-3 rounded-xl bg-gradient-to-r from-amber-500 to-orange-600 text-white font-semibold disabled:opacity-50">
                1. Lock ETH on Bridge.sol
              </button>
              <input value={lastEthTx} onChange={(e) => setLastEthTx(e.target.value)} placeholder="ETH tx hash (auto-filled after lock)"
                className="w-full px-4 py-2 rounded-xl font-mono text-xs bg-slate-100 dark:bg-slate-900/50 border border-slate-200 dark:border-slate-700" />
              <button type="button" disabled={submitting} onClick={claimOnNebula}
                className="w-full py-3 rounded-xl bg-gradient-to-r from-emerald-600 to-teal-600 text-white font-semibold disabled:opacity-50">
                2. Relayer mint on Nebula
              </button>
            </>
          ) : (
            <button type="button" disabled={submitting} onClick={lockOnNebula}
              className="w-full py-3 rounded-xl bg-gradient-to-r from-emerald-600 to-teal-600 text-white font-semibold disabled:opacity-50">
              Lock NBL on Nebula vault
            </button>
          )}

          {msg && (
            <p className={`text-sm flex gap-2 ${msgType === 'err' ? 'text-red-500' : 'text-emerald-600'}`}>
              {msgType === 'err' ? <AlertCircle className="w-4 h-4 shrink-0" /> : <CheckCircle className="w-4 h-4 shrink-0" />}
              {msg}
            </p>
          )}
        </AnimatedSection>

        <AnimatedSection delay={160} className="glass rounded-2xl p-5 mb-6">
          <h3 className="font-semibold mb-3 flex items-center gap-2"><Shield className="w-4 h-4" /> Relayer status</h3>
          {loading ? <RefreshCw className="w-5 h-5 animate-spin" /> : (
            <ul className="text-sm space-y-2">
              <li className="flex items-center gap-2">
                {bridgeStatus?.relayers_ready ? <CheckCircle className="w-4 h-4 text-emerald-500" /> : <AlertCircle className="w-4 h-4 text-red-500" />}
                {bridgeStatus?.validators_count ?? 0} validators · relayers {bridgeStatus?.relayers_ready ? 'ready' : 'offline'}
              </li>
              <li className="flex items-center gap-2">
                {bridgeStatus?.eth_rpc_configured ? <CheckCircle className="w-4 h-4 text-emerald-500" /> : <AlertCircle className="w-4 h-4 text-amber-500" />}
                Node ETH_RPC_URL {bridgeStatus?.eth_rpc_configured ? 'configured' : 'not set — pass eth_rpc_url in mint request'}
              </li>
              <li className="text-xs text-slate-400 font-mono">Vault: {bridgeStatus?.vault_address || '…'}</li>
            </ul>
          )}
        </AnimatedSection>

        {locks.length > 0 && (
          <AnimatedSection className="glass rounded-2xl p-5 mb-6">
            <h3 className="font-semibold mb-3">Your Nebula locks</h3>
            <ul className="space-y-2 text-xs font-mono">
              {locks.map((t) => (
                <li key={t.hash} className="flex justify-between"><span>{shorten(t.hash)}</span><span>{weiToNbl(t.value)} NBL</span></li>
              ))}
            </ul>
          </AnimatedSection>
        )}

        <p className="text-center text-xs text-slate-400">
          Deploy Bridge: <code className="font-mono">cd interop && npx hardhat run scripts/deploy.js</code>
          <a href="https://github.com/fewzfewz/scratchBlockchain/blob/main/interop/README.md" target="_blank" rel="noreferrer" className="inline-flex items-center gap-1 ml-2 text-emerald-500"><ExternalLink className="w-3 h-3" /> docs</a>
        </p>
      </div>
    </PageShell>
  )
}
