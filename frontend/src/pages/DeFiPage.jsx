import { useState, useEffect, useCallback } from 'react'
import { Link } from 'react-router-dom'
import { Layers, ArrowDown, Plus, Repeat } from 'lucide-react'
import { Page, PageHeader, Card, Field, Button, Notice, Stat, Tabs, buttonClass, inputClass } from '../components/ui.jsx'
import {
  DEFI_POOL, fetchBalance, fetchGasPrice, signAndSubmit,
  scanDeployedContracts, walletAddress, weiToNbl,
  nblToWei, encodeTransfer, erc20Balance, loadWalletKeyPair, estimateGas,
  ammQuoteAmountOut,
} from '../lib/chain.js'

export default function DeFiPage() {
  const [amountIn, setAmountIn] = useState('1.0')
  const [poolLiquidityWei, setPoolLiquidityWei] = useState(0n)
  const [userBalanceWei, setUserBalanceWei] = useState(0n)
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
      setPoolLiquidityWei(BigInt(poolBal.balance || 0))
      setBaseFee(gas.base_fee)
      if (addr) {
        const ub = await fetchBalance(addr)
        setUserBalanceWei(BigInt(ub.balance || 0))
      }
      const list = await scanDeployedContracts()
      const erc20s = list.filter((c) => c.type === 'ERC20' || c.name?.includes('ERC20'))
      setContracts(erc20s)
      if (erc20s[0] && !tokenOut) setTokenOut(erc20s[0].address)
    } catch {
      setPoolLiquidityWei(0n)
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
  }, [tokenOut, addr, userBalanceWei])

  const poolLiquidity = weiToNbl(poolLiquidityWei)
  const userBalance = weiToNbl(userBalanceWei)
  const amountInWei = nblToWei(amountIn || '0')
  const estimatedOut = ammQuoteAmountOut(
    userBalanceWei || 1n,
    poolLiquidityWei || 1n,
    amountInWei,
  )

  return (
    <Page width="max-w-5xl">
      <PageHeader icon={Layers} eyebrow="DeFi" title="Swap and provide liquidity"
        description="A constant-product pool on your node. The pool is a plain account, so a swap here is a signed transfer into it, priced with the AMM formula."
        actions={<Link to="/deploy" className={buttonClass('secondary', 'sm')}>Deploy a token</Link>} />

      <div className="grid grid-cols-2 sm:grid-cols-3 gap-3 mb-8">
        <Stat label="Pool liquidity" value={`${poolLiquidity} NBL`} />
        <Stat label="Your balance" value={`${userBalance} NBL`} />
        <Stat label="Base fee" value={baseFee} sub="wei per gas" />
      </div>

      <div className="grid lg:grid-cols-[1.2fr_1fr] gap-6 items-start">
        <Card title="Swap" icon={Repeat}>
          <Tabs value={swapMode} onChange={setSwapMode} className="mb-5" tabs={[
            { id: 'native', label: 'NBL transfer' },
            { id: 'erc20', label: 'ERC20 transfer' },
          ]} />

          {swapMode === 'erc20' && (
            <Field label="Token" hint={tokenBal != null ? `Your balance: ${tokenBal.toString()}` : undefined} className="mb-4">
              <select value={tokenOut} onChange={(e) => setTokenOut(e.target.value)} className={`${inputClass} font-mono text-xs`}>
                {contracts.length === 0 && <option value="">— deploy an ERC20 first —</option>}
                {contracts.map((c) => <option key={c.address} value={c.address}>{c.name || c.address}</option>)}
              </select>
            </Field>
          )}

          <div className="rounded-xl border border-slate-200 dark:border-slate-800 p-4">
            <div className="text-xs text-slate-500 mb-1">You pay</div>
            <div className="flex items-center gap-3">
              <input type="number" step="0.0001" value={amountIn} onChange={(e) => setAmountIn(e.target.value)} placeholder="Amount" aria-label="Amount"
                className="flex-1 min-w-0 bg-transparent font-display text-2xl font-semibold text-slate-900 dark:text-white focus:outline-none" />
              <span className="h-8 px-3 inline-flex items-center rounded-lg bg-slate-100 dark:bg-slate-800 text-sm font-semibold">{swapMode === 'erc20' ? 'TOKEN' : 'NBL'}</span>
            </div>
          </div>
          <div className="flex justify-center -my-3 relative z-10">
            <span className="w-8 h-8 rounded-lg bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 flex items-center justify-center"><ArrowDown className="w-4 h-4 text-slate-400" /></span>
          </div>
          <div className="rounded-xl border border-slate-200 dark:border-slate-800 p-4 bg-slate-50 dark:bg-slate-950/40">
            <div className="text-xs text-slate-500 mb-1">Estimated receive (0.3% fee)</div>
            <div className="font-display text-2xl font-semibold tabular-nums text-slate-900 dark:text-white">{weiToNbl(estimatedOut)} <span className="text-sm text-slate-500">NBL</span></div>
          </div>

          <Field label="Recipient (optional)" hint="Defaults to the pool." className="mt-4">
            <input value={recipient} onChange={(e) => setRecipient(e.target.value)} placeholder="Recipient (optional — defaults to pool)" className={`${inputClass} font-mono`} />
          </Field>

          <Button onClick={executeSwap} loading={submitting} icon={Repeat} className="w-full mt-5">Execute swap</Button>
          <Notice type={statusType === 'err' ? 'err' : 'ok'} className="mt-4">{status}</Notice>
        </Card>

        <Card title="Provide liquidity" icon={Plus} description="Deposit NBL into the pool. Uses the amount entered in the swap box.">
          <div className="rounded-xl bg-slate-50 dark:bg-slate-950/40 border border-slate-100 dark:border-slate-800 p-4 text-sm">
            <div className="flex justify-between"><span className="text-slate-500">Deposit</span><span className="tabular-nums font-semibold">{amountIn || 0} NBL</span></div>
            <div className="flex justify-between mt-2"><span className="text-slate-500">Pool after</span><span className="tabular-nums">{(parseFloat(poolLiquidity) + (parseFloat(amountIn) || 0)).toFixed(4)} NBL</span></div>
          </div>
          <Button variant="secondary" onClick={addLiquidity} loading={submitting} icon={Plus} className="w-full mt-4">Add liquidity</Button>
        </Card>
      </div>
    </Page>
  )
}
