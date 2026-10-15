// Load every UI route in headless Chromium (CDP) and report errors.
// Usage: node ui-check.mjs [route ...]   (E2E_OUT=<dir> to change the output dir)
import { spawn } from 'node:child_process'
import { mkdirSync, writeFileSync } from 'node:fs'

const args = process.argv.slice(2)
const OUT = process.env.E2E_OUT || new URL('./out/ui-check', import.meta.url).pathname
const BASE = process.env.UI_BASE || 'http://127.0.0.1:5173'
const ROUTES = args.length ? args : [
  '/', '/explorer', '/wallet', '/history', '/deploy', '/contracts', '/defi', '/bridge', '/wasm',
  '/advanced', '/nft', '/faucet', '/docs', '/developer-portal', '/sdk', '/governance',
  '/validators/onboard', '/api-docs',
]
const WAIT = Number(process.env.WAIT_MS || 6000)
mkdirSync(OUT, { recursive: true })

const port = 9333
const chrome = spawn('chromium', [
  '--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${OUT}/profile`,
  '--no-first-run', '--no-sandbox', '--disable-gpu', '--use-gl=swiftshader', '--enable-unsafe-swiftshader',
  'about:blank',
], { stdio: 'ignore' })
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

let ver
for (let i = 0; i < 50 && !ver; i++) {
  try { ver = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json() } catch { await sleep(200) }
}
const ws = new WebSocket(ver.webSocketDebuggerUrl)
await new Promise((r) => ws.addEventListener('open', r))
let id = 0
const pending = new Map()
const listeners = []
ws.addEventListener('message', (ev) => {
  const m = JSON.parse(ev.data)
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id) } else listeners.forEach((l) => l(m))
})
const send = (method, params = {}, sessionId) => new Promise((resolve) => {
  const i = ++id
  pending.set(i, resolve)
  ws.send(JSON.stringify({ id: i, method, params, sessionId }))
})

const report = {}
for (const route of ROUTES) {
  const { result: { targetId } } = await send('Target.createTarget', { url: 'about:blank' })
  const { result: { sessionId } } = await send('Target.attachToTarget', { targetId, flatten: true })
  const issues = []
  const urls = new Map()
  const onMsg = (m) => {
    if (m.sessionId !== sessionId) return
    const p = m.params || {}
    if (m.method === 'Runtime.exceptionThrown') issues.push(`EXCEPTION ${p.exceptionDetails?.exception?.description?.split('\n')[0] || p.exceptionDetails?.text}`)
    if (m.method === 'Runtime.consoleAPICalled' && (p.type === 'error' || p.type === 'warning' || p.type === 'warn'))
      issues.push(`CONSOLE.${p.type} ${p.args.map((a) => a.value ?? a.description ?? '').join(' ').slice(0, 300)}`)
    if (m.method === 'Network.requestWillBeSent') urls.set(p.requestId, `${p.request.method} ${p.request.url}`)
    if (m.method === 'Network.responseReceived' && p.response.status >= 400)
      issues.push(`HTTP ${p.response.status} ${urls.get(p.requestId) || p.response.url}`)
    if (m.method === 'Network.loadingFailed' && !p.canceled)
      issues.push(`NETFAIL ${p.errorText} ${urls.get(p.requestId)}`)
  }
  listeners.push(onMsg)
  await send('Runtime.enable', {}, sessionId)
  await send('Network.enable', {}, sessionId)
  await send('Page.enable', {}, sessionId)
  for (const [name, w, h, mobile] of [['desktop', 1440, 900, false], ['mobile', 390, 844, true]]) {
    await send('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: 1, mobile }, sessionId)
    await send('Page.navigate', { url: BASE + route }, sessionId)
    await sleep(WAIT)
    const { result } = await send('Runtime.evaluate', {
      expression: `JSON.stringify({w: document.documentElement.scrollWidth, vw: innerWidth, h: document.documentElement.scrollHeight, text: document.body.innerText.slice(0, 400)})`,
      returnByValue: true,
    }, sessionId)
    const info = JSON.parse(result.result.value)
    if (info.w > info.vw + 1) issues.push(`OVERFLOW ${name}: page ${info.w}px wide in ${info.vw}px viewport`)
    const shot = await send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false }, sessionId)
    const file = `${OUT}/${route.replace(/\//g, '_') || '_'}-${name}.png`
    writeFileSync(file, Buffer.from(shot.result.data, 'base64'))
    if (name === 'desktop') report[route] = { text: info.text.replace(/\s+/g, ' ').slice(0, 200) }
  }
  report[route].issues = [...new Set(issues)]
  listeners.splice(listeners.indexOf(onMsg), 1)
  await send('Target.closeTarget', { targetId })
}
writeFileSync(`${OUT}/report.json`, JSON.stringify(report, null, 2))
for (const [r, v] of Object.entries(report)) {
  console.log(`\n## ${r}  (${v.issues.length} issues)`)
  v.issues.forEach((i) => console.log('  ' + i))
}
ws.close()
chrome.kill()
process.exit(0)
