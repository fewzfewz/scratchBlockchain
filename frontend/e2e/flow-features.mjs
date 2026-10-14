import { launch, runSteps, sleep } from './cdp.mjs'
const OUT = process.env.E2E_OUT || new URL('./out/flow-features', import.meta.url).pathname
const UI = 'http://127.0.0.1:5173'
const ETH = 'http://127.0.0.1:9545'
const NODE = 'http://127.0.0.1:8546'
const only = process.env.ONLY ? new RegExp(process.env.ONLY, 'i') : null
const { page, close } = await launch(OUT, 9336)
const bal = async (a) => BigInt((await (await fetch(`${NODE}/balance/${a.replace(/^0x/, '')}`)).json()).balance || 0)
let addr, erc20, erc721

// A minimal EIP-1193 provider backed by Hardhat's unlocked dev accounts,
// standing in for MetaMask in headless Chrome.
const ETH_SHIM = `window.ethereum = { isMetaMask: true, request: async ({ method, params }) => {
  if (method === 'eth_requestAccounts') method = 'eth_accounts';
  if (method === 'wallet_switchEthereumChain' || method === 'wallet_addEthereumChain') return null;
  const r = await fetch('${ETH}', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params: params || [] }) });
  const j = await r.json(); if (j.error) throw new Error(j.error.message); return j.result; }, on() {}, removeListener() {} };`

const status = async () => (await page.text()).replace(/\s+/g, ' ')
// Wait for ok/bad text that was not on the page before the action.
let snap = ''
const mark = async () => { snap = await status() }
const fresh = (rx, text) => [...text.matchAll(new RegExp(rx, 'gi'))].map((m) => m[0]).filter((m) => !snap.includes(m))
const expectStatus = async (ok, bad, timeout = 60000) => {
  const t0 = Date.now()
  while (Date.now() - t0 < timeout) {
    const text = await status()
    const b = fresh(`.{0,60}(${bad}).{0,140}`, text)
    if (b.length) throw new Error(b[0])
    const g = fresh(ok, text)
    if (g.length) return g[0].slice(0, 160)
    await sleep(700)
  }
  throw new Error('no result within ' + timeout / 1000 + 's')
}

const steps = [
  ['setup: wallet + faucet', async () => {
    await page.goto(`${UI}/wallet`)
    await page.click('Generate a new wallet'); await page.click('Generate')
    await page.waitText(/New wallet generated/)
    addr = await page.evaluate("localStorage.getItem('nebula_wallet_addr')")
    await page.goto(`${UI}/faucet`); await page.fill('0x...', addr); await page.click('Request')
    for (let i = 0; i < 30 && (await bal(addr)) === 0n; i++) await sleep(1000)
    if ((await bal(addr)) === 0n) throw new Error('faucet did not fund')
    return addr
  }],
  ['deploy: ERC20', async () => {
    await page.goto(`${UI}/deploy`, 4000)
    await page.evaluate(`(() => { const s = [...document.querySelectorAll('select')].find(e => [...e.options].some(o => o.value === 'ERC20')); s && window.__set(s, 'ERC20') })()`)
    await page.click('Deploy', 'button[type=submit]')
    const m = await expectStatus('Contract address[\\s\\S]{0,80}?(0x[0-9a-f]{40})', 'Error:|failed|reverted', 90000)
    erc20 = (await status()).match(/0x[0-9a-fA-F]{40}/g)?.find((a) => a.toLowerCase() !== addr.toLowerCase())
    return erc20
  }],
  ['deploy: ERC721', async () => {
    await page.goto(`${UI}/deploy`, 4000)
    await page.evaluate(`(() => { const s = [...document.querySelectorAll('select')].find(e => [...e.options].some(o => o.value === 'ERC721')); s && window.__set(s, 'ERC721') })()`)
    await sleep(500)
    await mark()
    await mark()
    await page.click('Deploy', 'button[type=submit]')
    erc721 = await expectStatus('0x[0-9a-fA-F]{40}', 'Error:|failed|reverted', 90000)
    return erc721
  }],
  ['contracts: read balanceOf', async () => {
    await page.goto(`${UI}/contracts`, 5000)
    await page.evaluate(`(() => { const i = [...document.querySelectorAll('input')].filter(window.__vis)[0]; window.__set(i, ${JSON.stringify(erc20 || '')}) })()`)
    await page.click('balanceOf')
    await mark()
    await page.click('Read (call)')
    return await expectStatus('Read OK[^\\n]*', 'revert|failed|error')
  }],
  ['contracts: estimate + write (transfer)', async () => {
    const data = '0xa9059cbb' + '42'.repeat(20).padStart(64, '0') + (10n ** 18n).toString(16).padStart(64, '0')
    await page.evaluate(`(() => { const t = [...document.querySelectorAll('textarea')].filter(window.__vis)[0]; window.__set(t, ${JSON.stringify(data)}) })()`)
    await mark()
    await page.click('Estimate')
    await expectStatus('Estimated gas', 'Estimate failed')
    await mark()
    await page.click('Write')
    return await expectStatus('Tx [^—]+— success', '— failed|Send failed|error', 90000)
  }],
  ['nft: mint', async () => {
    await page.goto(`${UI}/nft`, 6000)
    await mark()
    await page.click('Mint on-chain')
    return await expectStatus('Minted[^\\n]*', 'failed|error|would fail', 200000)
  }],
  ['defi: add liquidity', async () => {
    await page.goto(`${UI}/defi`, 5000)
    await page.fill('Amount', '1')
    await mark()
    await page.click('Add liquidity')
    return await expectStatus('(added|deposit|success|confirmed)[^\\n]*', 'failed|error', 90000)
  }],
  ['defi: swap (NBL transfer mode)', async () => {
    await page.goto(`${UI}/defi`, 5000)
    await page.click('NBL transfer')
    await page.fill('Amount', '0.5')
    await mark()
    await page.click('Execute swap')
    return await expectStatus('(swap|success|confirmed)[^\\n]*', 'failed|error', 90000)
  }],
  ['wasm: call demo', async () => {
    await page.goto(`${UI}/wasm`, 5000)
    await mark()
    await page.click('Deploy', 'button')
    await expectStatus('Deployed "[^"]+"', 'failed|error|invalid|not on chain', 60000)
    await mark()
    await page.click('Call')
    return await expectStatus('Result: \\d+', 'failed|error|not found', 30000)
  }],
  ['advanced: AA user operation', async () => {
    await page.goto(`${UI}/advanced`, 4000)
    await mark()
    await page.click('Submit test UserOperation')
    return await expectStatus('(submitted|accepted|pending|hash)[^\\n]*', 'failed|error|invalid', 30000)
  }],
  ['advanced: MEV commit', async () => {
    await page.click('MEV Commit-Reveal')
    await mark()
    await page.click('Generate & commit')
    return await expectStatus('Commitment stored[^\n]*', 'failed|error|invalid', 30000)
  }],
  ['validators: onboard checks + register', async () => {
    await page.goto(`${UI}/validators/onboard`, 4000)
    await page.fill('Your wallet address', addr)
    const btns = await page.evaluate(`[...document.querySelectorAll('button')].filter(window.__vis).map(b => b.innerText.trim())`)
    const check = btns.find((b) => /check/i.test(b))
    if (check) await page.click(check)
    await mark()
    await page.click('Register my wallet as validator')
    return await expectStatus('(submitted|registered|success|hash)[^\\n]*', 'failed|error|insufficient', 60000)
  }],
  ['governance: create proposal', async () => {
    await page.goto(`${UI}/governance`, 5000)
    await page.click('Create')
    await sleep(800)
    const inputs = await page.evaluate(`[...document.querySelectorAll('input, textarea, select')].filter(window.__vis).map(e => e.tagName + ':' + (e.placeholder || e.name))`)
    const t = await page.evaluate(`(() => { const f = [...document.querySelectorAll('input, textarea')].filter(window.__vis);
      if (f[0]) window.__set(f[0], 'UI test proposal'); if (f[1]) window.__set(f[1], 'Created by the automated UI check'); return f.length })()`)
    const btns = await page.evaluate(`[...document.querySelectorAll('button')].filter(window.__vis).map(b => b.innerText.trim())`)
    const submit = btns.find((b) => /submit|create proposal|propose/i.test(b))
    if (!submit) throw new Error('no submit button: ' + JSON.stringify(btns) + ' inputs ' + JSON.stringify(inputs))
    await mark()
    await page.click(submit)
    return await expectStatus('(submitted|created|success)[^\\n]*', 'failed|error|insufficient', 60000)
  }],
  ['governance: vote on a proposal', async () => {
    await page.goto(`${UI}/governance`, 6000)
    await page.click('Proposals')
    await sleep(2000)
    // Show only active proposals, then open the first card (cards are not
    // buttons: click its status badge and let the click bubble).
    await page.click('Active', 'button')
    await sleep(1000)
    await page.click('UI test proposal', 'h2, h3, h4, p, span, div')
    await sleep(1000)
    await mark()
    await page.click('For')
    return await expectStatus('(vote (cast|submitted)|voted|submitted)[^\n]*', 'failed|error|not active|already voted', 60000)
  }],
  ['bridge: Nebula → ETH lock', async () => {
    await page.goto(`${UI}/bridge`, 4000)
    await page.click('Nebula → ETH')
    await page.fill('Amount', '0.1')
    await page.fill('Recipient on destination chain', '0x70997970C51812dc3A010C7d01b50e0d17dc79C8')
    const btns = await page.evaluate(`[...document.querySelectorAll('button')].filter(window.__vis).map(b => b.innerText.trim())`)
    const lock = btns.find((b) => /lock/i.test(b))
    if (!lock) throw new Error('no lock button: ' + JSON.stringify(btns))
    await mark()
    await page.click(lock)
    return await expectStatus('(lock tx|locked|submitted|queued)[^\\n]*', 'failed|error|invalid', 60000)
  }],
  ['bridge: ETH → Nebula lock + mint', async () => {
    await page.goto(`${UI}/bridge`, 3000)
    await page.evaluate(ETH_SHIM)
    await page.click('ETH → Nebula')
    await mark()
    await page.click('Connect MetaMask')
    await expectStatus('connected[^\\n]*', 'not installed|error|failed', 15000)
    await page.fill('Amount', '0.25')
    await page.fill('Recipient on destination chain', addr)
    const before = await bal(addr)
    await mark()
    await page.click('1. Lock ETH on Bridge.sol')
    await expectStatus('(locked|confirmed|tx)[^\\n]*', 'failed|error|reverted', 60000)
    await mark()
    await page.click('2. Relayer mint on Nebula')
    await expectStatus('(submitted|minted|success)[^\\n]*', 'failed|error|do not match', 60000)
    for (let i = 0; i < 30; i++) { const b = await bal(addr); if (b > before) return `+${b - before} wei`; await sleep(1000) }
    throw new Error('mint not credited')
  }],
  ['explorer: staking tab', async () => {
    await page.goto(`${UI}/explorer`, 3000)
    await page.click('Staking')
    await sleep(2500)
    return (await status()).match(/(total staked|staked)[^.]{0,60}/i)?.[0] || 'rendered'
  }],
]
await runSteps(page, only ? steps.filter(([n], i) => i === 0 || only.test(n)) : steps)
close()
process.exit(0)
