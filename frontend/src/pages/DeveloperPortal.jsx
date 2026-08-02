import { ExternalLink, Github, Code, FileCode, Terminal } from 'lucide-react'

export default function DeveloperPortal() {
  return (
    <div className="min-h-screen">
      <div className="sticky top-14 z-40 bg-[#0a0e1a]/90 backdrop-blur-xl border-b border-slate-800">
        <div className="max-w-5xl mx-auto px-4 h-14 flex items-center justify-between">
          <span className="text-sm font-bold text-white">Modular<span className="text-blue-400">Blockchain</span></span>
          <div className="flex gap-6 text-xs text-slate-400">
            {['Docs', 'SDK', 'Starter Kits', 'CLI', 'GitHub'].map(l => (
              <a key={l} href="#" className="hover:text-white transition-colors">{l}</a>
            ))}
          </div>
        </div>
      </div>

      <section className="py-16 px-4 text-center max-w-2xl mx-auto">
        <span className="inline-block text-xs font-semibold uppercase tracking-widest text-blue-400 bg-blue-500/10 px-3 py-1 rounded-full mb-4">Modular Blockchain</span>
        <h1 className="text-4xl md:text-6xl font-extrabold text-white mb-4 bg-gradient-to-r from-slate-100 to-slate-400 bg-clip-text text-transparent">Developer Portal</h1>
        <p className="text-slate-400 mb-8">Build decentralized applications on a high-performance, EVM-compatible blockchain.</p>
        <div className="flex gap-3 justify-center flex-wrap">
          {['Get Started', 'Starter Kits', 'CLI Tools'].map((b, i) => (
            <a key={b} href="#" className={`px-5 py-2.5 rounded-xl text-sm font-semibold transition-all ${
              i === 0 ? 'bg-blue-500 text-white hover:bg-blue-400' :
              i === 1 ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 hover:bg-emerald-500/20' :
              'border border-slate-700 text-slate-400 hover:text-slate-200'
            }`}>{b}</a>
          ))}
        </div>
      </section>

      <main className="max-w-5xl mx-auto px-4 space-y-16 pb-16">
        <section>
          <h2 className="text-2xl font-bold text-white mb-2">JavaScript SDK</h2>
          <p className="text-slate-400 mb-6">Connect, transact, and build with the Modular Blockchain SDK.</p>
          <div className="space-y-4 mb-6">
            {[
              { header: 'Install', code: 'npm install @modular-blockchain/sdk' },
              { header: 'Quick Start', code: `import { ModularClient, HttpProvider, Wallet } from '@modular-blockchain/sdk';

const client = new ModularClient(
  new HttpProvider('http://localhost:8545')
);

// Generate a wallet
const wallet = Wallet.generate();

// Check balance
const balance = await client.getBalance(wallet.address);

// Send a transaction
const tx = await wallet.signTransaction({
  to: '0x...', value: '1000000000000000000',
});
const result = await client.sendTransaction(tx);` },
            ].map(({ header, code }) => (
              <div key={header} className="rounded-xl border border-slate-700/50 bg-slate-800/40 overflow-hidden">
                <div className="flex items-center justify-between px-4 py-2 bg-black/20 border-b border-slate-700/50">
                  <span className="text-xs text-slate-500">{header}</span>
                  <button className="text-xs text-slate-500 hover:text-blue-400" onClick={(e) => { navigator.clipboard.writeText(code); e.target.textContent = 'Copied!'; setTimeout(() => e.target.textContent = 'Copy', 2000) }}>Copy</button>
                </div>
                <pre className="p-4 overflow-x-auto text-sm text-slate-300 font-mono"><code>{code}</code></pre>
              </div>
            ))}
          </div>
          <div className="grid md:grid-cols-3 gap-4">
            {[
              { icon: 'W', title: 'Wallet', desc: 'Generate, import, sign transactions with Ed25519 keys.' },
              { icon: 'C', title: 'Client', desc: 'Full RPC coverage: blocks, transactions, accounts.' },
              { icon: 'G', title: 'Governance', desc: 'Proposals, voting, treasury via SDK methods.' },
            ].map(({ icon, title, desc }) => (
              <div key={title} className="p-5 rounded-xl border border-slate-700/50 bg-slate-800/40 hover:border-slate-600/60 transition-all">
                <div className="w-10 h-10 rounded-xl bg-blue-500/10 text-blue-400 flex items-center justify-center font-bold mb-3">{icon}</div>
                <h3 className="text-white font-semibold mb-1">{title}</h3>
                <p className="text-sm text-slate-400">{desc}</p>
              </div>
            ))}
          </div>
        </section>

        <section>
          <h2 className="text-2xl font-bold text-white mb-2">Starter Kits</h2>
          <p className="text-slate-400 mb-6">Ready-to-deploy templates for common dApp patterns.</p>
          <div className="grid md:grid-cols-4 gap-4">
            {[
              { icon: 'T', title: 'Token', desc: 'ERC-20 and ERC-721 token deployment.' },
              { icon: 'D', title: 'DeFi', desc: 'AMM DEX with liquidity pools.' },
              { icon: 'N', title: 'NFT Marketplace', desc: 'Mint, list, buy, sell NFTs.' },
              { icon: 'G', title: 'DAO', desc: 'On-chain governance with voting.' },
            ].map(({ icon, title, desc }) => (
              <a key={title} href="#" className="p-5 rounded-xl border border-slate-700/50 bg-slate-800/40 hover:border-blue-500/50 transition-all group">
                <div className="w-10 h-10 rounded-xl bg-blue-500/10 text-blue-400 flex items-center justify-center font-bold mb-3">{icon}</div>
                <h3 className="text-white font-semibold mb-1">{title}</h3>
                <p className="text-sm text-slate-400 mb-2">{desc}</p>
                <span className="text-xs text-blue-400 group-hover:text-blue-300">View Kit →</span>
              </a>
            ))}
          </div>
        </section>

        <section>
          <h2 className="text-2xl font-bold text-white mb-2">CLI Tool</h2>
          <p className="text-slate-400 mb-6">Command-line tools for development and deployment.</p>
          <div className="rounded-xl border border-slate-700/50 bg-slate-800/40 overflow-hidden mb-6">
            <div className="flex items-center justify-between px-4 py-2 bg-black/20 border-b border-slate-700/50">
              <span className="text-xs text-slate-500">Install</span>
              <button className="text-xs text-slate-500 hover:text-blue-400">Copy</button>
            </div>
            <pre className="p-4 text-sm text-slate-300 font-mono"><code>npm install -g @modular-blockchain/cli</code></pre>
          </div>
          <div className="grid md:grid-cols-3 gap-3">
            {[
              { cmd: 'modular init my-dapp --kit token', desc: 'Scaffold a new project from a template.' },
              { cmd: 'modular deploy ERC20 --args "MyToken,MTK,1000000"', desc: 'Deploy smart contracts from the CLI.' },
              { cmd: 'modular wallet generate', desc: 'Generate wallets and check balances.' },
            ].map(({ cmd, desc }) => (
              <div key={cmd} className="p-4 rounded-xl border border-slate-700/50 bg-slate-800/40">
                <code className="text-xs text-blue-400 bg-black/30 px-2 py-1 rounded block mb-2">{cmd}</code>
                <p className="text-sm text-slate-400">{desc}</p>
              </div>
            ))}
          </div>
        </section>
      </main>
    </div>
  )
}
