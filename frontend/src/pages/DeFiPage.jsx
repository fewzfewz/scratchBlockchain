import { useState, useEffect, useCallback } from 'react'
import { Link } from 'react-router-dom'
import { Layers, ArrowDown, Plus, Repeat, Rocket } from 'lucide-react'
import { Page, PageHeader, Card, Field, Button, Notice, Stat, Tabs, buttonClass, inputClass } from '../components/ui.jsx'
import {
  DEFI_POOL, fetchBalance, fetchGasPrice, signAndSubmit, waitForReceipt,
  scanDeployedContracts, walletAddress, weiToNbl, nblToWei, padAddr,
  encodeTransfer, encodeApprove, erc20Balance, loadWalletKeyPair, estimateGas,
  callContract, decodeUint256, ammQuoteAmountOut, saveContract, shorten,
} from '../lib/chain.js'
import simpleAmm from '../lib/simpleAmm.json'

const DEFAULT_AMM_ADDRESS = import.meta.env.VITE_AMM_ADDRESS || '0xb65dba4b46bdaf9588904dedc58959fa4033e6cf'
const isAddress = (a) => /^0x[a-fA-F0-9]{40}$/.test((a || '').trim())
const u256Hex = (v) => BigInt(v).toString(16).padStart(64, '0')
const AMM_SEL = {
  token0: '0x0dfe1681',
  token1: '0xd21220a7',
  reserve0: '0x443cb4bc',
  reserve1: '0x5a76f25e',
  addLiquidity: '0x9cd441da',
  swap: '0x6d9a640a',
}

function decodeAddr(res) {
  if (!res?.success || !res.result) return ''
  const clean = res.result.replace(/^0x/, '')
  return clean.length >= 40 ? `0x${clean.slice(-40)}` : ''
}

export default function DeFiPage() {
  const [tab, setTab] = useState('amm')
  const [amountIn, setAmountIn] = useState('1.0')
  const [poolLiquidityWei, setPoolLiquidityWei] = useState(0n)
  const [userBalanceWei, setUserBalanceWei] = useState(0n)
  const [baseFee, setBaseFee] = useState('—')
  const [swapMode, setSwapMode] = useState('native')
  const [tokenOut, setTokenOut] = useState('')
  const [customToken, setCustomToken] = useState('')
  const [contracts, setContracts] = useState([])
  const [recipient, setRecipient] = useState('')
  const [status, setStatus] = useState('')
  const [statusType, setStatusType] = useState('')
  const [submitting, setSubmitting] = useState(false)

  // SimpleAMM state
  const [ammAddress, setAmmAddress] = useState(() => localStorage.getItem('nebula_amm_address') || DEFAULT_AMM_ADDRESS)
  const [deployToken0, setDeployToken0] = useState('')
  const [deployToken1, setDeployToken1] = useState('')
  const [ammState, setAmmState] = useState({ token0: '', token1: '', reserve0: 0n, reserve1: 0n })
  const [ammDir, setAmmDir] = useState('0to1')
  const [liqAmount0, setLiqAmount0] = useState('10')
  const [liqAmount1, setLiqAmount1] = useState('10')

  const addr = walletAddress()
  const activeErc20 = isAddress(customToken) ? customToken.trim() : tokenOut

  const loadAmmState = useCallback(async (amm) => {
    if (!isAddress(amm)) {
      setAmmState({ token0: '', token1: '', reserve0: 0n, reserve1: 0n })
      return
    }
    const caller = addr || '0x0000000000000000000000000000000000000001'
    try {
      const [t0, t1, r0, r1] = await Promise.all([
        callContract(caller, amm, AMM_SEL.token0),
        callContract(caller, amm, AMM_SEL.token1),
        callContract(caller, amm, AMM_SEL.reserve0),
        callContract(caller, amm, AMM_SEL.reserve1),
      ])
      setAmmState({
        token0: decodeAddr(t0),
        token1: decodeAddr(t1),
        reserve0: r0?.success ? decodeUint256(r0.result) : 0n,
        reserve1: r1?.success ? decodeUint256(r1.result) : 0n,
      })
    } catch {
      /* ignore */
    }
  }, [addr])

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
      const erc20s = list.filter(
        (c) => c.type === 'ERC20' || c.type === 'unknown' || c.name?.includes('ERC20'),
      )
      setContracts(erc20s)
      if (erc20s[0] && !tokenOut) setTokenOut(erc20s[0].address)
      if (isAddress(ammAddress)) loadAmmState(ammAddress)
    } catch {
      setPoolLiquidityWei(0n)
    }
  }, [addr, tokenOut, ammAddress, loadAmmState])

  useEffect(() => { refresh() }, [refresh])

  const deployAmm = async () => {
    if (!loadWalletKeyPair()) {
      setStatus('Create a wallet on /wallet first'); setStatusType('err'); return
    }
    if (!isAddress(deployToken0) || !isAddress(deployToken1)) {
      setStatus('Enter valid Token0 and Token1 ERC20 addresses'); setStatusType('err'); return
    }
    setSubmitting(true)
    setStatus('Deploying SimpleAMM contract…')
    try {
      const payload = simpleAmm.bytecode.replace(/^0x/, '') + padAddr(deployToken0) + padAddr(deployToken1)
      const est = await estimateGas(addr, '', '0x' + payload, '0')
      if (est.error) throw new Error(est.error)
      const { hash } = await signAndSubmit({
        from: addr,
        to: null,
        valueWei: 0n,
        payload,
        gasLimit: est.estimated_gas || 1_200_000,
      })
      const receipt = await waitForReceipt(hash)
      const created = receipt?.contract_address || receipt?.created_address
      if (!created) throw new Error('Deploy confirmed but no contract address returned')
      const full = created.startsWith('0x') ? created : `0x${created}`
      localStorage.setItem('nebula_amm_address', full)
      setAmmAddress(full)
      saveContract({ address: full, type: 'AMM', name: 'SimpleAMM', txHash: hash })
      setStatus(`SimpleAMM deployed at ${shorten(full, 10, 8)}`)
      setStatusType('ok')
      loadAmmState(full)
    } catch (e) {
      setStatus(e.message || 'AMM deploy failed'); setStatusType('err')
    } finally {
      setSubmitting(false)
    }
  }

  const ammAddLiquidity = async () => {
    if (!loadWalletKeyPair()) { setStatus('Create a wallet on /wallet first'); setStatusType('err'); return }
    if (!isAddress(ammAddress) || !ammState.token0 || !ammState.token1) {
      setStatus('Load or deploy a valid SimpleAMM first'); setStatusType('err'); return
    }
    setSubmitting(true)
    try {
      const a0 = nblToWei(liqAmount0 || '0')
      const a1 = nblToWei(liqAmount1 || '0')
      setStatus('Step 1/3: Approving Token0…')
      const d0 = encodeApprove(ammAddress, a0)
      const { hash: h0 } = await signAndSubmit({ from: addr, to: ammState.token0, valueWei: 0n, payload: d0.replace(/^0x/, ''), gasLimit: 100000 })
      await waitForReceipt(h0)

      setStatus('Step 2/3: Approving Token1…')
      const d1 = encodeApprove(ammAddress, a1)
      const { hash: h1 } = await signAndSubmit({ from: addr, to: ammState.token1, valueWei: 0n, payload: d1.replace(/^0x/, ''), gasLimit: 100000 })
      await waitForReceipt(h1)

      setStatus('Step 3/3: Adding liquidity to SimpleAMM…')
      const callData = AMM_SEL.addLiquidity + u256Hex(a0) + u256Hex(a1)
      const est = await estimateGas(addr, ammAddress, callData, '0')
      if (est.error) throw new Error(est.error)
      const { hash: h2 } = await signAndSubmit({ from: addr, to: ammAddress, valueWei: 0n, payload: callData.replace(/^0x/, ''), gasLimit: est.estimated_gas || 250000 })
      await waitForReceipt(h2)
      setStatus(`Liquidity added — tx ${shorten(h2, 10, 8)}`)
      setStatusType('ok')
      loadAmmState(ammAddress)
    } catch (e) {
      setStatus(e.message || 'Add liquidity failed'); setStatusType('err')
    } finally {
      setSubmitting(false)
    }
  }

  const ammSwap = async () => {
    if (!loadWalletKeyPair()) { setStatus('Create a wallet on /wallet first'); setStatusType('err'); return }
    if (!isAddress(ammAddress) || !ammState.token0 || !ammState.token1) {
      setStatus('Load or deploy a valid SimpleAMM first'); setStatusType('err'); return
    }
    const ain = nblToWei(amountIn || '0')
    const resIn = ammDir === '0to1' ? ammState.reserve0 : ammState.reserve1
    const resOut = ammDir === '0to1' ? ammState.reserve1 : ammState.reserve0
    const outWei = ammQuoteAmountOut(resIn, resOut, ain)
    if (outWei <= 0n || outWei >= resOut) {
      setStatus('Insufficient AMM reserves for this swap amount'); setStatusType('err'); return
    }
    setSubmitting(true)
    try {
      const tokenIn = ammDir === '0to1' ? ammState.token0 : ammState.token1
      setStatus('Step 1/2: Transferring input token to AMM…')
      const tData = encodeTransfer(ammAddress, ain)
      const { hash: h0 } = await signAndSubmit({ from: addr, to: tokenIn, valueWei: 0n, payload: tData.replace(/^0x/, ''), gasLimit: 100000 })
      await waitForReceipt(h0)

      setStatus('Step 2/2: Executing SimpleAMM swap…')
      const a0Out = ammDir === '0to1' ? 0n : outWei
      const a1Out = ammDir === '0to1' ? outWei : 0n
      const sData = AMM_SEL.swap + u256Hex(a0Out) + u256Hex(a1Out) + padAddr(addr)
      const est = await estimateGas(addr, ammAddress, sData, '0')
      if (est.error) throw new Error(est.error)
      const { hash: h1 } = await signAndSubmit({ from: addr, to: ammAddress, valueWei: 0n, payload: sData.replace(/^0x/, ''), gasLimit: est.estimated_gas || 250000 })
      await waitForReceipt(h1)
      setStatus(`Swapped! Received ~${weiToNbl(outWei)} output tokens (tx ${shorten(h1, 10, 8)})`)
      setStatusType('ok')
      loadAmmState(ammAddress)
    } catch (e) {
      setStatus(e.message || 'AMM swap failed'); setStatusType('err')
    } finally {
      setSubmitting(false)
    }
  }

  const addLiquidity = async () => {
    if (!loadWalletKeyPair()) {
      setStatus('Create a wallet on /wallet first'); setStatusType('err'); return
    }
    setSubmitting(true)
    setStatus('Submitting liquidity deposit…')
    try {
      const pool = await DEFI_POOL()
      const wei = nblToWei(amountIn)
      const est = await estimateGas(addr, pool, '0x', wei.toString())
      const { hash } = await signAndSubmit({
        from: addr, to: pool, valueWei: wei, gasLimit: est.estimated_gas || 21000,
      })
      await waitForReceipt(hash)
      setStatus(`Liquidity added — tx ${hash.slice(0, 10)}…`)
      setStatusType('ok')
      refresh()
    } catch (e) {
      setStatus(e.message || 'Deposit failed'); setStatusType('err')
    } finally {
      setSubmitting(false)
    }
  }

  const executeSwap = async () => {
    if (!loadWalletKeyPair()) {
      setStatus('Create a wallet on /wallet first'); setStatusType('err'); return
    }
    setSubmitting(true)
    setStatus('Signing transfer…')
    try {
      if (swapMode === 'native') {
        const to = recipient.trim() || await DEFI_POOL()
        const wei = nblToWei(amountIn)
        const est = await estimateGas(addr, to, '0x', wei.toString())
        const { hash } = await signAndSubmit({
          from: addr, to, valueWei: wei, gasLimit: est.estimated_gas || 21000,
        })
        await waitForReceipt(hash)
        setStatus(`NBL transfer confirmed — tx ${hash.slice(0, 10)}…`)
      } else if (activeErc20) {
        const to = recipient.trim() || await DEFI_POOL()
        const data = encodeTransfer(to, nblToWei(amountIn))
        const est = await estimateGas(addr, activeErc20, data, '0')
        const { hash } = await signAndSubmit({
          from: addr, to: activeErc20, valueWei: 0n, payload: data.replace(/^0x/, ''), gasLimit: est.estimated_gas || 100000,
        })
        await waitForReceipt(hash)
        setStatus(`Token transfer confirmed — tx ${hash.slice(0, 10)}…`)
      }
      setStatusType('ok')
      refresh()
    } catch (e) {
      setStatus(e.message || 'Transfer failed'); setStatusType('err')
    } finally {
      setSubmitting(false)
    }
  }

  const [tokenBal, setTokenBal] = useState(null)
  useEffect(() => {
    if (!isAddress(activeErc20) || !addr) { setTokenBal(null); return }
    erc20Balance(activeErc20, addr).then(setTokenBal).catch(() => setTokenBal(null))
  }, [activeErc20, addr, userBalanceWei])

  const poolLiquidity = weiToNbl(poolLiquidityWei)
  const userBalance = weiToNbl(userBalanceWei)
  const amountInWei = nblToWei(amountIn || '0')
  const ammResIn = ammDir === '0to1' ? ammState.reserve0 : ammState.reserve1
  const ammResOut = ammDir === '0to1' ? ammState.reserve1 : ammState.reserve0
  const ammEstimatedOut = ammQuoteAmountOut(ammResIn, ammResOut, amountInWei)

  return (
    <Page width="max-w-5xl">
      <PageHeader icon={Layers} eyebrow="DeFi" title="AMM DEX &amp; Pool"
        description="Swap ERC20 pairs on-chain with SimpleAMM, or send NBL/ERC20 transfers to the node pool."
        actions={<Link to="/deploy" className={buttonClass('secondary', 'sm')}>Deploy a token</Link>} />

      <div className="grid grid-cols-2 sm:grid-cols-3 gap-3 mb-6">
        <Stat label="Node pool" value={`${poolLiquidity} NBL`} />
        <Stat label="Your balance" value={`${userBalance} NBL`} />
        <Stat label="Base fee" value={baseFee} sub="wei per gas" />
      </div>

      <Tabs value={tab} onChange={setTab} className="mb-6" tabs={[
        { id: 'amm', label: 'SimpleAMM DEX' },
        { id: 'transfer', label: 'Pool Transfer (NBL / ERC20)' },
      ]} />

      <Notice type={statusType === 'err' ? 'err' : 'ok'} className="mb-6">{status}</Notice>

      {tab === 'amm' && (
        <div className="grid lg:grid-cols-[1.2fr_1fr] gap-6 items-start">
          <Card title="Swap via SimpleAMM" icon={Repeat}>
            <div className="space-y-4">
              <Field label="SimpleAMM contract" hint={ammState.token0 ? `Pair: ${shorten(ammState.token0, 6, 4)} / ${shorten(ammState.token1, 6, 4)}` : 'Paste an existing SimpleAMM address or deploy one on the right.'}>
                <div className="flex gap-2">
                  <input value={ammAddress} onChange={(e) => { setAmmAddress(e.target.value); localStorage.setItem('nebula_amm_address', e.target.value) }}
                    placeholder="0x…" className={`${inputClass} font-mono text-xs`} />
                  {ammAddress.toLowerCase() !== DEFAULT_AMM_ADDRESS.toLowerCase() && (
                    <Button type="button" variant="secondary" size="sm" onClick={() => { setAmmAddress(DEFAULT_AMM_ADDRESS); localStorage.setItem('nebula_amm_address', DEFAULT_AMM_ADDRESS) }}>
                      Official Pool
                    </Button>
                  )}
                </div>
              </Field>
              <Tabs value={ammDir} onChange={setAmmDir} tabs={[
                { id: '0to1', label: 'Token0 → Token1' },
                { id: '1to0', label: 'Token1 → Token0' },
              ]} />
              <div className="rounded-xl border border-slate-200 dark:border-slate-800 p-4">
                <div className="text-xs text-slate-500 mb-1">You pay ({ammDir === '0to1' ? 'Token0' : 'Token1'})</div>
                <input type="number" step="0.0001" value={amountIn} onChange={(e) => setAmountIn(e.target.value)}
                  className="w-full bg-transparent font-display text-2xl font-semibold text-slate-900 dark:text-white focus:outline-none" />
              </div>
              <div className="flex justify-center -my-2 relative z-10">
                <span className="w-8 h-8 rounded-lg bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 flex items-center justify-center"><ArrowDown className="w-4 h-4 text-slate-400" /></span>
              </div>
              <div className="rounded-xl border border-slate-200 dark:border-slate-800 p-4 bg-slate-50 dark:bg-slate-950/40">
                <div className="text-xs text-slate-500 mb-1">Estimated receive (0.3% fee · live reserves)</div>
                <div className="font-display text-2xl font-semibold tabular-nums text-slate-900 dark:text-white">
                  {weiToNbl(ammEstimatedOut)} <span className="text-sm text-slate-500">{ammDir === '0to1' ? 'Token1' : 'Token0'}</span>
                </div>
              </div>
              <div className="text-xs text-slate-500 flex justify-between">
                <span>Reserve0: {weiToNbl(ammState.reserve0)}</span>
                <span>Reserve1: {weiToNbl(ammState.reserve1)}</span>
              </div>
              <Button onClick={ammSwap} loading={submitting} icon={Repeat} className="w-full">Execute AMM swap</Button>
            </div>
          </Card>

          <div className="space-y-6">
            <Card title="Add AMM liquidity" icon={Plus} description="Approves both ERC20s and deposits into SimpleAMM.">
              <div className="space-y-3">
                <Field label="Token0 amount"><input value={liqAmount0} onChange={(e) => setLiqAmount0(e.target.value)} className={inputClass} /></Field>
                <Field label="Token1 amount"><input value={liqAmount1} onChange={(e) => setLiqAmount1(e.target.value)} className={inputClass} /></Field>
                <Button variant="secondary" onClick={ammAddLiquidity} loading={submitting} icon={Plus} className="w-full">Approve &amp; add liquidity</Button>
              </div>
            </Card>

            <Card title="Deploy SimpleAMM pair" icon={Rocket} description="Deploy a new constant-product pool for any two ERC20 tokens.">
              <div className="space-y-3">
                <Field label="Token0 address"><input value={deployToken0} onChange={(e) => setDeployToken0(e.target.value)} placeholder="0x…" className={`${inputClass} font-mono text-xs`} /></Field>
                <Field label="Token1 address"><input value={deployToken1} onChange={(e) => setDeployToken1(e.target.value)} placeholder="0x…" className={`${inputClass} font-mono text-xs`} /></Field>
                <Button variant="secondary" onClick={deployAmm} loading={submitting} icon={Rocket} className="w-full">Deploy SimpleAMM</Button>
              </div>
            </Card>
          </div>
        </div>
      )}

      {tab === 'transfer' && (
        <div className="grid lg:grid-cols-[1.2fr_1fr] gap-6 items-start">
          <Card title="Pool transfer" icon={Repeat}>
            <Tabs value={swapMode} onChange={setSwapMode} className="mb-5" tabs={[
              { id: 'native', label: 'NBL transfer' },
              { id: 'erc20', label: 'ERC20 transfer' },
            ]} />

            {swapMode === 'erc20' && (
              <div className="space-y-3 mb-4">
                <Field label="Token" hint={tokenBal != null ? `Your balance: ${weiToNbl(tokenBal)} tokens` : undefined}>
                  <select value={tokenOut} onChange={(e) => { setTokenOut(e.target.value); setCustomToken('') }} className={`${inputClass} font-mono text-xs`}>
                    {contracts.length === 0 && <option value="">— deploy an ERC20 first —</option>}
                    {contracts.map((c) => <option key={c.address} value={c.address}>{c.name || c.address}</option>)}
                  </select>
                </Field>
                <Field label="Or paste ERC20 address">
                  <input value={customToken} onChange={(e) => setCustomToken(e.target.value)} placeholder="0x…" className={`${inputClass} font-mono text-xs`} />
                </Field>
              </div>
            )}

            <div className="rounded-xl border border-slate-200 dark:border-slate-800 p-4">
              <div className="text-xs text-slate-500 mb-1">Amount</div>
              <div className="flex items-center gap-3">
                <input type="number" step="0.0001" value={amountIn} onChange={(e) => setAmountIn(e.target.value)} placeholder="Amount" aria-label="Amount"
                  className="flex-1 min-w-0 bg-transparent font-display text-2xl font-semibold text-slate-900 dark:text-white focus:outline-none" />
                <span className="h-8 px-3 inline-flex items-center rounded-lg bg-slate-100 dark:bg-slate-800 text-sm font-semibold">{swapMode === 'erc20' ? 'TOKEN' : 'NBL'}</span>
              </div>
            </div>

            <Field label="Recipient (optional)" hint="Defaults to the node pool." className="mt-4">
              <input value={recipient} onChange={(e) => setRecipient(e.target.value)} placeholder="Recipient (optional — defaults to pool)" className={`${inputClass} font-mono`} />
            </Field>

            <Button onClick={executeSwap} loading={submitting} icon={Repeat} className="w-full mt-5">Send transfer</Button>
          </Card>

          <Card title="Provide NBL liquidity" icon={Plus} description="Deposit NBL into the node pool.">
            <div className="rounded-xl bg-slate-50 dark:bg-slate-950/40 border border-slate-100 dark:border-slate-800 p-4 text-sm">
              <div className="flex justify-between"><span className="text-slate-500">Deposit</span><span className="tabular-nums font-semibold">{amountIn || 0} NBL</span></div>
              <div className="flex justify-between mt-2"><span className="text-slate-500">Pool after</span><span className="tabular-nums">{(parseFloat(poolLiquidity) + (parseFloat(amountIn) || 0)).toFixed(4)} NBL</span></div>
            </div>
            <Button variant="secondary" onClick={addLiquidity} loading={submitting} icon={Plus} className="w-full mt-4">Add liquidity</Button>
          </Card>
        </div>
      )}
    </Page>
  )
}
