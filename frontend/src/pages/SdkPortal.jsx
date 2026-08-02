import { FileCode, Terminal, Database, BookOpen, Github } from 'lucide-react'

export default function SdkPortal() {
  return (
    <div className="min-h-screen">
      <div className="bg-gradient-to-b from-slate-800/60 to-transparent border-b border-slate-700/50 py-16 px-4 text-center">
        <div className="max-w-3xl mx-auto">
          <h1 className="text-4xl md:text-5xl font-extrabold text-white mb-4">
            Modular <span className="text-violet-400">Blockchain</span>
          </h1>
          <p className="text-slate-400 mb-8">
            Build decentralized applications on a high-performance, modular blockchain with Ed25519 security and EIP-1559 gas mechanics.
          </p>
          <div className="flex gap-3 justify-center flex-wrap">
            {['Quick Start', 'SDK Docs', 'Templates', 'CLI'].map((b, i) => (
              <a key={b} href="#" className={`px-5 py-2.5 rounded-lg text-sm font-semibold transition-all ${
                i === 0 ? 'bg-violet-600 text-white hover:bg-violet-500' : 'border border-slate-600 text-slate-400 hover:text-slate-200'
              }`}>{b}</a>
            ))}
          </div>
        </div>
      </div>

      <div className="max-w-5xl mx-auto px-4 -mt-8">
        <div className="grid grid-cols-4 gap-4 mb-12">
          {[
            { value: '1s', label: 'Block Time' },
            { value: '10k+', label: 'TPS' },
            { value: 'Ed25519', label: 'Cryptography' },
            { value: 'EIP-1559', label: 'Fee Market' },
          ].map(({ value, label }) => (
            <div key={label} className="p-4 rounded-xl border border-slate-700/50 bg-slate-800/80 text-center">
              <div className="text-xl font-extrabold text-violet-400">{value}</div>
              <div className="text-xs text-slate-500">{label}</div>
            </div>
          ))}
        </div>

        <section className="mb-12">
          <h2 className="text-2xl font-bold text-white mb-2">Quick Start</h2>
          <p className="text-slate-400 mb-6">Get connected and send your first transaction in 30 seconds.</p>
          <div className="space-y-6">
            {[
              { num: '1', title: 'Install the SDK', code: 'npm install @modular-blockchain/sdk' },
              { num: '2', title: 'Connect to a node', code: `import { ModularClient, HttpProvider } from "@modular-blockchain/sdk";

const provider = new HttpProvider("http://localhost:8545");
const client = new ModularClient(provider);
const status = await client.getNodeStatus();
console.log("Height:", status.height);` },
              { num: '3', title: 'Create a wallet', code: `import { Wallet } from "@modular-blockchain/sdk";

const wallet = Wallet.generate();
console.log("Address:", wallet.address); // 0x...` },
              { num: '4', title: 'Send a transaction', code: `const tx = await wallet.signTransaction({
  to: "0x...",
  value: "1000000000000000000", // 1 NBL
});
const receipt = await client.sendTransaction(tx);
console.log("Tx hash:", receipt.hash);` },
            ].map(({ num, title, code }) => (
              <div key={num} className="flex gap-5 pb-6 border-b border-slate-700/50 last:border-0">
                <div className="w-10 h-10 rounded-full bg-violet-600 flex items-center justify-center font-bold text-white shrink-0">{num}</div>
                <div>
                  <h3 className="text-white font-semibold mb-2">{title}</h3>
                  <pre className="p-4 rounded-xl bg-slate-800/80 border border-slate-700/50 overflow-x-auto text-sm text-slate-300"><code>{code}</code></pre>
                </div>
              </div>
            ))}
          </div>
        </section>

        <hr className="border-slate-700/50 mb-12" />

        <section className="mb-12">
          <h2 className="text-2xl font-bold text-white mb-2">SDK Packages</h2>
          <p className="text-slate-400 mb-6">Everything you need to build on Modular Blockchain.</p>
          <div className="grid md:grid-cols-3 gap-4">
            {[
              { icon: '📦', title: '@modular-blockchain/sdk', desc: 'TypeScript SDK with wallet management, transaction building, providers.', tags: ['TypeScript', 'Node.js'] },
              { icon: '🛠️', title: '@modular-blockchain/sdk-cli', desc: 'CLI for scaffolding projects, deploying contracts, managing wallets.', tags: ['CLI', 'Scaffold'] },
              { icon: '📄', title: 'Smart Contract Templates', desc: 'Battle-tested Solidity templates for ERC-20, ERC-721, DAO.', tags: ['Solidity', 'Open Source'] },
            ].map(({ icon, title, desc, tags }) => (
              <div key={title} className="p-5 rounded-xl border border-slate-700/50 bg-slate-800/40 hover:border-violet-500/50 transition-all">
                <div className="text-2xl mb-3">{icon}</div>
                <h3 className="text-white font-semibold text-sm mb-2">{title}</h3>
                <p className="text-sm text-slate-400 mb-4">{desc}</p>
                <div className="flex gap-2">
                  {tags.map(t => (
                    <span key={t} className={`px-2 py-0.5 rounded text-xs font-medium ${
                      t === 'TypeScript' || t === 'Solidity' ? 'bg-emerald-500/15 text-emerald-400' :
                      t === 'CLI' ? 'bg-orange-500/15 text-orange-400' :
                      'bg-blue-500/15 text-blue-400'
                    }`}>{t}</span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </section>

        <hr className="border-slate-700/50 mb-12" />

        <section className="mb-12">
          <h2 className="text-2xl font-bold text-white mb-2">CLI Tool</h2>
          <p className="text-slate-400 mb-6">Scaffold, deploy, and manage your dApps from the terminal.</p>
          <pre className="p-4 rounded-xl bg-slate-800/80 border border-slate-700/50 overflow-x-auto text-sm text-slate-300 mb-6"><code>{`# Install
npm install -g @modular-blockchain/sdk-cli

# Generate wallet
modular wallet --generate

# Scaffold a DeFi project
modular init --template defi --name my-amm

# Deploy a contract
modular deploy --contract ERC20 --args "MyToken,MTK,1000000"`}</code></pre>
          <div className="grid md:grid-cols-4 gap-4">
            {[
              { icon: '🚀', title: 'modular init', desc: 'Scaffold projects from templates.' },
              { icon: '📤', title: 'modular deploy', desc: 'Deploy bytecode or templates.' },
              { icon: '➕', title: 'modular create', desc: 'Generate contract files.' },
              { icon: '👛', title: 'modular wallet', desc: 'Generate and manage wallets.' },
            ].map(({ icon, title, desc }) => (
              <div key={title} className="p-4 rounded-xl border border-slate-700/50 bg-slate-800/40">
                <div className="text-xl mb-2">{icon}</div>
                <h3 className="text-white font-semibold text-sm mb-1">{title}</h3>
                <p className="text-xs text-slate-400">{desc}</p>
              </div>
            ))}
          </div>
        </section>

        <hr className="border-slate-700/50 mb-12" />

        <section className="mb-12">
          <h2 className="text-2xl font-bold text-white mb-2">RPC API</h2>
          <p className="text-slate-400 mb-6">The Modular Blockchain node exposes these REST endpoints.</p>
          <pre className="p-4 rounded-xl bg-slate-800/80 border border-slate-700/50 overflow-x-auto text-sm text-slate-300"><code>{`# Node status
GET /status

# Submit transaction
POST /submit_tx  body: Transaction JSON

# Query
GET /block/{height}     GET /balance/{address}
GET /tx/{hash}          GET /gas_price
GET /fee_history/{count}

# Mempool
GET /mempool

# Network
GET /peers              POST /connect_peer`}</code></pre>
        </section>
      </div>
    </div>
  )
}
