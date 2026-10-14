import { launch, runSteps, sleep } from './cdp.mjs'
const OUT = process.env.E2E_OUT || new URL('./out/flow-wallet', import.meta.url).pathname
const UI = 'http://127.0.0.1:5173'
const { page, close } = await launch(OUT)
let addr
const nodeBalance = async (a) => BigInt((await (await fetch(`http://127.0.0.1:8546/balance/${a.replace(/^0x/, '')}`)).json()).balance || 0)

await runSteps(page, [
  ['wallet: generate', async () => {
    await page.goto(`${UI}/wallet`)
    await page.click('Generate a new wallet')
    await page.click('Generate')
    await page.waitText(/New wallet generated/)
    addr = await page.evaluate("localStorage.getItem('nebula_wallet_addr')")
    if (!/^0x[0-9a-f]{40}$/i.test(addr || '')) throw new Error(`bad address ${addr}`)
    return addr
  }],
  ['faucet: request tokens', async () => {
    await page.goto(`${UI}/faucet`)
    await page.fill('0x...', addr)
    await page.click('Request')
    await page.waitText(/on the way|error/i, 20000)
    const t = await page.text()
    if (/error/i.test(t.match(/(on the way.*|Error.*)/i)?.[0] || '')) throw new Error(t.match(/Error.*/)?.[0])
    for (let i = 0; i < 30; i++) { if ((await nodeBalance(addr)) > 0n) return `balance ${await nodeBalance(addr)}`; await sleep(1000) }
    throw new Error('faucet never credited the wallet')
  }],
  ['wallet: balance shown', async () => {
    await page.goto(`${UI}/wallet`, 5000)
    const m = await page.waitText(/(\d+(\.\d+)?)\s*NBL/, 15000)
    return m[0]
  }],
  ['wallet: send transfer', async () => {
    const before = await nodeBalance('0x' + '42'.repeat(20))
    await page.fill('0x...', '0x' + '42'.repeat(20))
    await page.fill('0.00', '1.5')
    await page.click('Send transaction')
    await page.waitText(/Transaction submitted|failed|Error/i, 20000)
    const t = await page.text()
    if (!/Transaction submitted/.test(t)) throw new Error(t.match(/(Transaction failed.*|Error.*)/)?.[0])
    for (let i = 0; i < 30; i++) {
      const after = await nodeBalance('0x' + '42'.repeat(20))
      if (after > before) return `recipient +${after - before} wei`
      await sleep(1000)
    }
    throw new Error('transfer not applied on chain')
  }],
  ['history: shows the transfer', async () => {
    await page.goto(`${UI}/history`, 6000)
    const t = await page.text()
    if (!/4242|0x4242/i.test(t)) throw new Error('history does not list the transfer: ' + t.slice(0, 300).replace(/\s+/g, ' '))
  }],
  ['explorer: address tab finds wallet', async () => {
    await page.goto(`${UI}/explorer`)
    await page.click('Address')
    await page.fill('0x', addr)
    await page.evaluate(`(() => { const f = document.querySelector('form'); if (f) f.requestSubmit(); })()`)
    await sleep(1500)
    try { await page.click('Search') } catch {}
    await page.waitText(/NBL|balance/i, 10000)
  }],
  ['explorer: blocks tab + block detail', async () => {
    await page.click('Blocks', '[role=tab], button')
    await page.waitText(/#\d+|Block/i, 10000)
    await sleep(2000)
    await page.evaluate(`(() => { const b = [...document.querySelectorAll('button')].find(e => /#\\s?\\d{2,}/.test(e.innerText)); b && b.click(); })()`)
    await sleep(1500)
    await page.waitText(/state root|parent|proposer/i, 8000)
  }],
  ['explorer: validators tab', async () => {
    await page.click('Validators', '[role=tab], button')
    await page.waitText(/401c76|9f140c|stake/i, 10000)
  }],
])
console.log('\nWALLET', addr)
close()
process.exit(0)
