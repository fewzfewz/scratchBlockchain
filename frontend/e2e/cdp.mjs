// Minimal Chrome DevTools Protocol driver for UI flow tests.
import { spawn } from 'node:child_process'
import { mkdirSync, writeFileSync } from 'node:fs'

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

export async function launch(outDir, port = 9334) {
  mkdirSync(outDir, { recursive: true })
  const chrome = spawn('chromium', [
    '--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${outDir}/profile`,
    '--no-first-run', '--no-sandbox', '--disable-gpu', '--use-gl=swiftshader', '--enable-unsafe-swiftshader',
    'about:blank',
  ], { stdio: 'ignore' })
  let ver
  for (let i = 0; i < 50 && !ver; i++) {
    try { ver = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json() } catch { await sleep(200) }
  }
  const ws = new WebSocket(ver.webSocketDebuggerUrl)
  await new Promise((r) => ws.addEventListener('open', r))
  let id = 0
  const pending = new Map()
  const listeners = new Set()
  ws.addEventListener('message', (ev) => {
    const m = JSON.parse(ev.data)
    if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id) } else listeners.forEach((l) => l(m))
  })
  const send = (method, params = {}, sessionId) => new Promise((resolve, reject) => {
    const i = ++id
    pending.set(i, (m) => (m.error ? reject(new Error(`${method}: ${m.error.message}`)) : resolve(m.result)))
    ws.send(JSON.stringify({ id: i, method, params, sessionId }))
  })

  const { targetId } = await send('Target.createTarget', { url: 'about:blank' })
  const { sessionId } = await send('Target.attachToTarget', { targetId, flatten: true })
  const issues = []
  const urls = new Map()
  listeners.add((m) => {
    if (m.sessionId !== sessionId) return
    const p = m.params || {}
    if (m.method === 'Runtime.exceptionThrown') issues.push(`EXCEPTION ${p.exceptionDetails?.exception?.description?.split('\n')[0] || p.exceptionDetails?.text}`)
    if (m.method === 'Runtime.consoleAPICalled' && p.type === 'error')
      issues.push(`CONSOLE.error ${p.args.map((a) => a.value ?? a.description ?? '').join(' ').slice(0, 300)}`)
    if (m.method === 'Network.requestWillBeSent') urls.set(p.requestId, `${p.request.method} ${p.request.url}`)
    if (m.method === 'Network.responseReceived' && p.response.status >= 400)
      issues.push(`HTTP ${p.response.status} ${urls.get(p.requestId) || p.response.url}`)
    if (m.method === 'Network.loadingFailed' && !p.canceled) issues.push(`NETFAIL ${p.errorText} ${urls.get(p.requestId)}`)
  })
  for (const d of ['Runtime', 'Network', 'Page']) await send(`${d}.enable`, {}, sessionId)
  await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false }, sessionId)

  const evaluate = async (expression) => {
    const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, sessionId)
    if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description || r.exceptionDetails.text)
    return r.result.value
  }
  // In-page helpers: find elements by visible text / label / placeholder.
  const HELPERS = `
    window.__vis = (el) => !!(el.offsetWidth || el.offsetHeight || el.getClientRects().length);
    window.__byText = (text, sel = 'button, a, [role=tab], [role=button], label, summary') => {
      const els = [...document.querySelectorAll(sel)].filter(window.__vis).filter((e) => e.innerText);
      const t = text.toLowerCase();
      // Prefer the topmost (dialogs come last in the DOM) exact match, then a partial one.
      return els.reverse().find((e) => e.innerText.trim().toLowerCase() === t)
        || els.find((e) => e.innerText.trim().toLowerCase().includes(t));
    };
    window.__input = (hint) => {
      const all = [...document.querySelectorAll('input, textarea, select')].filter(window.__vis);
      const h = hint.toLowerCase();
      return all.find((e) => (e.placeholder || '').toLowerCase().includes(h) || (e.name || '').toLowerCase() === h || (e.id || '').toLowerCase() === h)
        || all.find((e) => { const l = e.closest('label') || (e.id && document.querySelector('label[for="'+e.id+'"]'));
             if (l && l.innerText.toLowerCase().includes(h)) return true;
             let p = e.parentElement; for (let i = 0; i < 3 && p; i++, p = p.parentElement) {
               const lab = p.querySelector('label'); if (lab && lab.innerText.toLowerCase().includes(h)) return true; }
             return false; });
    };
    window.__set = (el, v) => {
      const proto = el.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : el.tagName === 'SELECT' ? HTMLSelectElement.prototype : HTMLInputElement.prototype;
      Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, v);
      el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true }));
    };`

  const page = {
    issues,
    evaluate,
    async goto(url, wait = 3000) {
      await send('Page.navigate', { url }, sessionId)
      await sleep(wait)
      await evaluate(HELPERS)
    },
    async reloadHelpers() { await evaluate(HELPERS) },
    async click(text, sel) {
      let ok = false
      for (let i = 0; i < 16 && ok !== true; i++) {
        if (i) await sleep(500)
        await evaluate(HELPERS)
        ok = await evaluate(`(() => { const e = window.__byText(${JSON.stringify(text)}${sel ? ', ' + JSON.stringify(sel) : ''}); if (!e) return false; if (e.disabled) return 'disabled'; e.scrollIntoView({block:'center'}); e.click(); return true })()`)
      }
      if (ok !== true) throw new Error(`click "${text}": ${ok === 'disabled' ? 'button disabled' : 'not found'}`)
      await sleep(400)
    },
    async fill(hint, value) {
      await evaluate(HELPERS)
      const ok = await evaluate(`(() => { const e = window.__input(${JSON.stringify(hint)}); if (!e) return false; window.__set(e, ${JSON.stringify(value)}); return true })()`)
      if (!ok) throw new Error(`fill "${hint}": input not found`)
      await sleep(200)
    },
    text: () => evaluate('document.body.innerText'),
    async waitText(re, timeout = 30000) {
      const rx = re instanceof RegExp ? re : new RegExp(re, 'i')
      const t0 = Date.now()
      let last = ''
      while (Date.now() - t0 < timeout) {
        last = await evaluate('document.body.innerText')
        const m = last.match(rx)
        if (m) return m
        await sleep(500)
      }
      throw new Error(`timed out waiting for ${rx}`)
    },
    async shot(file) {
      const r = await send('Page.captureScreenshot', { format: 'png' }, sessionId)
      writeFileSync(`${outDir}/${file}.png`, Buffer.from(r.data, 'base64'))
    },
  }
  return { page, close: () => { ws.close(); chrome.kill() } }
}

// Run named steps, recording pass/fail without stopping at the first failure.
export async function runSteps(page, steps) {
  const results = []
  for (const [name, fn] of steps) {
    const before = page.issues.length
    const started = Date.now()
    try {
      const note = await fn()
      results.push({ name, ok: true, note: note || '', issues: page.issues.slice(before) })
    } catch (e) {
      results.push({ name, ok: false, note: e.message.slice(0, 300), issues: page.issues.slice(before) })
      try { await page.shot(`FAIL-${name.replace(/[^a-z0-9]+/gi, '_')}`) } catch {}
    }
    const r = results[results.length - 1]
    console.log(`${r.ok ? 'PASS' : 'FAIL'}  ${name} (${((Date.now() - started) / 1000).toFixed(1)}s)${r.note ? ' — ' + r.note : ''}`)
    r.issues.forEach((i) => console.log('      ' + i))
  }
  return results
}
