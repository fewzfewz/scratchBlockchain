import { useState, useEffect, useCallback } from 'react'
import { Link } from 'react-router-dom'
import { Layers, ArrowLeftRight, Droplets, TrendingUp, RefreshCw, AlertCircle, CheckCircle } from 'lucide-react'
import PageShell from '../components/PageShell.jsx'
import {
  DEFI_POOL, fetchBalance, fetchGasPrice, signAndSubmit,
  scanDeployedContracts, walletAddress, weiToNbl,
  nblToWei, encodeTransfer, erc20Balance, loadWalletKeyPair, estimateGas,
} from '../lib/chain.js'

export default function DeFiPage() {
  const [amountIn, setAmountIn] = useState('1.0')
  const [poolLiquidity, setPoolLiquidity] = useState('—')
  const [userBalance, setUserBalance] = useState('—')
  const [baseFee, setBaseFee] = useState('—')
  const [swapMode, setSwapMode] = useState('native')
  const [tokenOut, setTokenOut] = useState('')
  const [contracts, setContracts] = useState([])
  const [recipient, setRecipient] = useState('')
  const [status, setStatus] = useState('')
  const [statusType, setStatusType] = useState('')
  const [submitting, setSubmitting] = useState(false)

  const addr = walletAddress()

  const refresh = useCallback(async () => {
    try {
      const pool = await DEFI_POOL()
      const [poolBal, gas] = await Promise.all([
        fetchBalance(pool),
        fetchGasPrice(),
      ])
      setPoolLiquidity(weiToNbl(poolBal.balance))
      setBaseFee(gas.base_fee)
      if (addr) {
        const ub = await fetchBalance(addr)
        setUserBalance(weiToNbl(ub.balance))
      }
      const list = await scanDeployedContracts()
      const erc20s = list.filter((c) => c.type === 'ERC20' || c.name?.includes('ERC20'))
      setContracts(erc20s)
      if (erc20s[0] && !tokenOut) setTokenOut(erc20s[0].address)
    } catch {
      setPoolLiquidity('0')
    }
  }, [addr, tokenOut])

  useEffect(() => { refresh() }, [refresh])

  const addLiquidity = async () => {
    if (!loadWalletKeyPair()) {
      setStatus('Create a wallet on /wallet first')
      setStatusType('err')
      return
    }
    setSubmitting(true)
    setStatus('Submitting liquidity deposit…')
    try {
      const pool = await DEFI_POOL()
      const wei = nblToWei(amountIn)
      const est = await estimateGas(addr, pool, '0x', wei.toString())
      const { hash } = await signAndSubmit({
        from: addr,
        to: pool,
        valueWei: wei,
        gasLimit: est.estimated_gas || 21000,
      })
      setStatus(`Liquidity added — tx ${hash.slice(0, 10)}…`)
      setStatusType('ok')
      refresh()
    } catch (e) {
      setStatus(e.message || 'Deposit failed')
      setStatusType('err')
    } finally {
      setSubmitting(false)
    }
  }

  const executeSwap = async () => {
    if (!loadWalletKeyPair()) {
      setStatus('Create a wallet on /wallet first')
      setStatusType('err')
      return
    }
    setSubmitting(true)
    setStatus('Signing swap transaction…')
    try {
      if (swapMode === 'native') {
        const to = recipient.trim() || await DEFI_POOL()
        const wei = nblToWei(amountIn)
        const est = await estimateGas(addr, to, '0x', wei.toString())
        const { hash } = await signAndSubmit({
          from: addr,
          to,
          valueWei: wei,
          gasLimit: est.estimated_gas || 21000,
        })
        setStatus(`Swap sent — tx ${hash.slice(0, 10)}…`)
      } else if (tokenOut) {
        const to = recipient.trim() || await DEFI_POOL()
        const data = encodeTransfer(to, nblToWei(amountIn))
        const est = await estimateGas(addr, tokenOut, data, '0')
        const { hash } = await signAndSubmit({
          from: addr,
          to: tokenOut,
          valueWei: 0n,
          payload: data.replace(/^0x/, ''),
          gasLimit: est.estimated_gas || 100000,
        })
        setStatus(`Token transfer sent — tx ${hash.slice(0, 10)}…`)
      }
      setStatusType('ok')
      refresh()
    } catch (e) {
      setStatus(e.message || 'Swap failed')
      setStatusType('err')
    } finally {
      setSubmitting(false)
    }
  }

  const [tokenBal, setTokenBal] = useState(null)
  useEffect(() => {
    if (!tokenOut || !addr) return
    erc20Balance(tokenOut, addr).then(setTokenBal).catch(() => setTokenBal(null))
  }, [tokenOut, addr, userBalance])

  return (
    <PageShell variant="default">
      <div className="max-w-3xl mx-auto px-4 py-10 animate-fade-in">
        <header className="text-center mb-8">
          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-blue-600 to-cyan-600 flex items-center justify-center text-white shadow-lg">
              <Layers className="w-6 h-6" />
            </div>
            <h1 className="text-3xl font-bold text-slate-900 dark:text-white">DeFi Hub</h1>
          </div>
          <p className="text-slate-500 dark:text-slate-400">Live pool balances and on-chain swaps via signed transactions.</p>
        </header>

        <div className="grid sm:grid-cols-3 gap-3 mb-6">
          <div className="glass-strong rounded-2xl p-4 text-center">
            <Droplets className="w-5 h-5 mx-auto mb-2 text-cyan-500" />
            <p className="text-xs text-slate-500">Pool (on-chain)</p>
            <p className="text-lg font-bold font-mono">{poolLiquidity} NBL</p>
          </div>
          <div className="glass-strong rounded-2xl p-4 text-center">
            <TrendingUp className="w-5 h-5 mx-auto mb-2 text-emerald-500" />
            <p className="text-xs text-slate-500">Your balance</p>
            <p className="text-lg font-bold font-mono">{userBalance} NBL</p>
          </div>
          <div className="glass-strong rounded-2xl p-4 text-center">
            <ArrowLeftRight className="w-5 h-5 mx-auto mb-2 text-violet-500" />
            <p className="text-xs text-slate-500">Base fee</p>
            <p className="text-lg font-bold font-mono text-xs">{baseFee}</p>
          </div>
        </div>

        <div className="glass-strong rounded-2xl p-6 mb-6 space-y-4">
          <div className="flex gap-2">
            <button type="button" onClick={() => setSwapMode('native')}
              className={`flex-1 py-2 rounded-xl text-sm font-semibold ${swapMode === 'native' ? 'bg-cyan-600 text-white' : 'text-slate-500'}`}>
              NBL transfer
            </button>
            <button type="button" onClick={() => setSwapMode('erc20')}
              className={`flex-1 py-2 rounded-xl text-sm font-semibold ${swapMode === 'erc20' ? 'bg-cyan-600 text-white' : 'text-slate-500'}`}>
              ERC20 transfer
            </button>
          </div>

          {swapMode === 'erc20' && (
            <div>
              <label className="text-xs text-slate-500">Token contract</label>
              <select value={tokenOut} onChange={(e) => setTokenOut(e.target.value)}
                className="w-full mt-1 px-4 py-3 rounded-xl bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600 font-mono text-sm">
                {contracts.map((c) => (
                  <option key={c.address} value={c.address}>{c.name || c.address}</option>
                ))}
              </select>
              {tokenBal != null && <p className="text-xs text-slate-400 mt-1">Your token balance: {tokenBal.toString()}</p>}
            </div>
          )}

          <input type="number" step="0.0001" value={amountIn} onChange={(e) => setAmountIn(e.target.value)}
            placeholder="Amount"
            className="w-full px-4 py-3 rounded-xl font-mono bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600" />
          <input value={recipient} onChange={(e) => setRecipient(e.target.value)}
            placeholder="Recipient (optional — defaults to pool)"
            className="w-full px-4 py-3 rounded-xl font-mono text-sm bg-white/70 dark:bg-slate-800/60 border border-slate-300 dark:border-slate-600" />

          <div className="flex gap-3">
            <button type="button" disabled={submitting} onClick={executeSwap}
              className="flex-1 py-3 rounded-xl bg-gradient-to-r from-blue-600 to-cyan-600 text-white font-semibold disabled:opacity-50">
              {submitting ? <RefreshCw className="w-4 h-4 animate-spin mx-auto" /> : 'Execute swap'}
            </button>
            <button type="button" disabled={submitting} onClick={addLiquidity}
              className="flex-1 py-3 rounded-xl border border-cyan-500/50 text-cyan-600 font-semibold disabled:opacity-50">
              Add liquidity
            </button>
          </div>

          {status && (
            <p className={`text-sm flex items-center gap-2 ${statusType === 'err' ? 'text-red-500' : 'text-emerald-600'}`}>
              {statusType === 'err' ? <AlertCircle className="w-4 h-4" /> : <CheckCircle className="w-4 h-4" />}
              {status}
            </p>
          )}
        </div>

        <p className="text-center text-xs text-slate-400">
          Deploy tokens at <Link to="/deploy" className="text-cyan-500 underline">/deploy</Link> · Pool address is deterministic on-chain
        </p>
      </div>
    </PageShell>
  )
}
