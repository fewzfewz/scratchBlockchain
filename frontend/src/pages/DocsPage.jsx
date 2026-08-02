import { useState } from 'react'
import { BookOpen, Zap, Shield, Layers, Code, FileText, Copy, Check, FileJson } from 'lucide-react'

const SECTIONS = [
  { id: 'intro', title: 'Introduction' },
  { id: 'install', title: 'Installation' },
  { id: 'quickstart', title: 'Quick Start' },
  { id: 'architecture', title: 'Architecture' },
  { id: 'accounts', title: 'Accounts' },
  { id: 'transactions', title: 'Transactions' },
  { id: 'rpc', title: 'RPC API' },
  { id: 'errors', title: 'Error Codes' },
  { id: 'local-dev', title: 'Local Dev' },
]

function CopyBtn({ text }) {
  const [copied, setCopied] = useState(false)
  return (
    <button
      onClick={() => { navigator.clipboard.writeText(text); setCopied(true); setTimeout(() => setCopied(false), 2000) }}
      className="text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 transition-colors"
    >
      {copied ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
    </button>
  )
}

function CodeBlock({ code, lang = '' }) {
  return (
    <div className="group rounded-xl border border-slate-200 dark:border-slate-700/50 bg-slate-50 dark:bg-slate-800/80 overflow-hidden mb-4">
      {lang && (
        <div className="flex items-center justify-between px-4 py-1.5 bg-slate-100 dark:bg-black/30 border-b border-slate-200 dark:border-slate-700/50">
          <span className="text-[10px] uppercase tracking-wider text-slate-500 dark:text-slate-500">{lang}</span>
          <CopyBtn text={code} />
        </div>
      )}
      <pre className="p-4 overflow-x-auto text-sm text-slate-700 dark:text-slate-300 font-mono leading-relaxed"><code>{code}</code></pre>
    </div>
  )
}

export default function DocsPage() {
  const [activeSection, setActiveSection] = useState('intro')

  return (
    <div className="max-w-6xl mx-auto px-4 py-8 animate-fade-in">
      <div className="grid grid-cols-[220px_1fr] gap-8">
        <aside className="sticky top-20 h-[calc(100vh-6rem)] overflow-y-auto scrollbar-thin">
          <div className="space-y-6">
            <div className="flex items-center gap-2 text-blue-500 dark:text-blue-400 mb-2">
              <BookOpen className="w-4 h-4" />
              <span className="text-xs uppercase tracking-wider font-medium">Nebula Docs</span>
            </div>
            <div className="space-y-0.5">
              {SECTIONS.map(({ id, title }) => (
                <button
                  key={id}
                  onClick={() => setActiveSection(id)}
                  className={`block w-full text-left px-3 py-1.5 rounded-lg text-sm transition-all ${
                    activeSection === id
                      ? 'text-blue-600 dark:text-blue-300 bg-blue-50 dark:bg-blue-600/10 border-l-2 border-blue-500 dark:border-blue-400'
                      : 'text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200'
                  }`}
                >
                  {title}
                </button>
              ))}
            </div>
          </div>
        </aside>

        <div className="min-w-0">
          {activeSection === 'intro' && (
            <section className="animate-slide-up">
              <h1 className="text-4xl font-bold text-slate-900 dark:text-white mb-4 bg-gradient-to-r from-slate-800 to-slate-400 dark:from-white dark:to-slate-400 bg-clip-text text-transparent">Introduction to Nebula</h1>
              <p className="text-lg text-blue-600 dark:text-blue-300 mb-4">A high-performance, modular blockchain built in Rust.</p>
              <p className="text-slate-600 dark:text-slate-400 mb-8 leading-relaxed">Nebula is designed for scalability and flexibility, featuring a multi-VM execution environment, validator-based consensus with instant finality, and built-in Layer 2 support.</p>
              <div className="grid md:grid-cols-3 gap-4">
                {[
                  { icon: Zap, title: 'High Performance', desc: 'Parallel transaction execution and high-throughput mempool.' },
                  { icon: Shield, title: 'Secure', desc: 'Ed25519 signatures and GRANDPA-style finality gadget.' },
                  { icon: Layers, title: 'Modular', desc: 'Pluggable consensus, execution, and data availability layers.' },
                ].map(({ icon: Icon, title, desc }) => (
                  <div key={title} className="p-5 rounded-xl glass">
                    <Icon className="w-6 h-6 text-blue-500 dark:text-blue-400 mb-3" />
                    <h4 className="text-slate-800 dark:text-white font-semibold mb-1">{title}</h4>
                    <p className="text-sm text-slate-500 dark:text-slate-400">{desc}</p>
                  </div>
                ))}
              </div>
            </section>
          )}

          {activeSection === 'install' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">Installation</h2>
              <p className="text-slate-600 dark:text-slate-400 mb-4">Prerequisites: Rust (latest stable) and Docker.</p>
              <CodeBlock lang="bash" code={`# Clone the repository
git clone https://github.com/your-org/nebula.git
cd nebula

# Build release binary
cargo build --release

# Or run with Docker Compose (recommended for local dev)
docker compose up -d`} />
              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">System Requirements</h3>
              <ul className="text-sm text-slate-600 dark:text-slate-400 space-y-1 list-disc list-inside mb-4">
                <li>Rust 1.80+ (stable toolchain)</li>
                <li>Docker Engine 24+ with Compose v2</li>
                <li>4 GB RAM minimum (8 GB recommended for devnet)</li>
                <li>Node.js 18+ (for frontends)</li>
              </ul>
            </section>
          )}

          {activeSection === 'quickstart' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">Quick Start</h2>
              <p className="text-slate-600 dark:text-slate-400 mb-4">Deploy a local 3-node testnet in under 60 seconds.</p>

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">1. Single-node devnet</h3>
              <CodeBlock lang="bash" code={`./scripts/deploy.sh

# Check node status
curl http://localhost:8545/status

# Expected response:
# {"height":42,"peers":2,"uptime_secs":120,"consensus":"BFT","finalized":41}`} />

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">2. Multi-node testnet</h3>
              <CodeBlock lang="bash" code={`cd deployment/local
docker-compose up -d

# Each node exposes an API:
# validator1: http://localhost:8545
# validator2: http://localhost:8546
# validator3: http://localhost:8547
# rpc1:       http://localhost:8548
# rpc2:       http://localhost:8549`} />

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">3. Verify consensus</h3>
              <CodeBlock lang="bash" code={`# Check all nodes are on the same block height
for port in 8545 8546 8547 8548 8549; do
  echo "Node $port: $(curl -s http://localhost:$port/status | grep -o '"height":[0-9]*')"
done`} />
            </section>
          )}

          {activeSection === 'architecture' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">Architecture</h2>
              <div className="space-y-6">
                <div className="p-5 rounded-xl glass">
                  <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Consensus Layer</h3>
                  <p className="text-sm text-slate-600 dark:text-slate-400 leading-relaxed">Validator-based BFT consensus with Ed25519 signatures and GRANDPA-style finality gadget. Blocks are finalized in batches for instant irreversibility. Validator set is managed through on-chain governance.</p>
                  <div className="mt-3 grid grid-cols-3 gap-3 text-xs">
                    {[
                      ['Finality', '~2s (batch)'],
                      ['Block time', '1s'],
                      ['Max validators', '32 (configurable)'],
                    ].map(([k, v]) => (
                      <div key={k} className="p-2 rounded-lg bg-slate-100 dark:bg-slate-700/40 text-center">
                        <span className="text-slate-500 dark:text-slate-400">{k}</span>
                        <br />
                        <span className="font-semibold text-slate-800 dark:text-slate-200">{v}</span>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="p-5 rounded-xl glass">
                  <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Networking Layer</h3>
                  <p className="text-sm text-slate-600 dark:text-slate-400 mb-3">P2P discovery via libp2p with Gossipsub for block and transaction propagation.</p>
                  <ul className="text-sm text-slate-600 dark:text-slate-400 space-y-1 list-disc list-inside">
                    <li>Peer reputation system prevents spam and DoS</li>
                    <li>Automatic NAT traversal with hole-punching</li>
                    <li>Encrypted transport with Noise protocol</li>
                    <li>Configurable gossip topics for scalability</li>
                  </ul>
                </div>

                <div className="p-5 rounded-xl glass">
                  <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Storage Layer</h3>
                  <p className="text-sm text-slate-600 dark:text-slate-400 mb-3">High-performance storage layer using RocksDB (default) with Sled legacy support.</p>
                  <ul className="text-sm text-slate-600 dark:text-slate-400 space-y-1 list-disc list-inside">
                    <li>Trie-based state storage for efficient Merkle proofs</li>
                    <li>Pruning: configurable archive vs. recent-only modes</li>
                    <li>Snapshot-based state sync for fast node bootstrap</li>
                    <li>Column families for separate block/trie/metadata stores</li>
                  </ul>
                </div>
              </div>
            </section>
          )}

          {activeSection === 'accounts' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">Accounts</h2>
              <p className="text-slate-600 dark:text-slate-400 mb-6">Nebula uses Ed25519 key pairs for account identity. Addresses are derived from the public key.</p>

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Key Format</h3>
              <p className="text-sm text-slate-600 dark:text-slate-400 mb-3">Each account is identified by a 20-byte address (truncated SHA-256 hash of the Ed25519 public key). The full 32-byte public key is used for signature verification.</p>
              <CodeBlock lang="text" code={`Public key:  0x02aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899 (32 bytes)
Address:      0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18 (20 bytes)
Private key:  0xdeadbeef... (64 bytes, keep secret!)`} />

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Account State</h3>
              <div className="overflow-x-auto mb-4">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-slate-200 dark:border-slate-700">
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Field</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Type</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Description</th>
                    </tr>
                  </thead>
                  <tbody className="text-slate-700 dark:text-slate-300">
                    {[
                      ['nonce', 'u64', 'Transaction count (increments with each tx)'],
                      ['balance', 'U256', 'Account balance in wei'],
                      ['storage_root', 'H256', 'Merkle root of contract storage (EOA: zero)'],
                      ['code_hash', 'H256', 'Keccak hash of contract bytecode (EOA: zero)'],
                    ].map(([field, type, desc]) => (
                      <tr key={field} className="border-b border-slate-100 dark:border-slate-700/50">
                        <td className="py-2 font-mono text-xs">{field}</td>
                        <td className="py-2 text-xs text-slate-500 dark:text-slate-400">{type}</td>
                        <td className="py-2 text-xs text-slate-500 dark:text-slate-400">{desc}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </section>
          )}

          {activeSection === 'transactions' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">Transactions</h2>
              <p className="text-slate-600 dark:text-slate-400 mb-6">Nebula uses an EIP-1559 style transaction format with Ed25519 signatures.</p>

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Transaction Fields</h3>
              <div className="overflow-x-auto mb-6">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-slate-200 dark:border-slate-700">
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Field</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Type</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Description</th>
                    </tr>
                  </thead>
                  <tbody className="text-slate-700 dark:text-slate-300">
                    {[
                      ['sender', '[u8; 20]', 'Sender address (derived from public key)'],
                      ['to', '[u8; 20]', 'Recipient address (or empty for contract deploy)'],
                      ['nonce', 'u64', 'Sender nonce (prevents replay attacks)'],
                      ['value', 'U256', 'Amount to transfer in wei'],
                      ['gas_limit', 'u64', 'Maximum gas units for this tx'],
                      ['max_fee_per_gas', 'u128', 'Maximum total fee per gas unit (wei)'],
                      ['max_priority_fee_per_gas', 'u128', 'Maximum tip for validators (wei)'],
                      ['payload', 'bytes', 'Calldata (for contract calls) or bytecode (for deploy)'],
                      ['chain_id', 'u64', 'Chain identifier (prevents cross-chain replay)'],
                      ['signature', '[u8; 64]', 'Ed25519 signature over the tx hash'],
                    ].map(([field, type, desc]) => (
                      <tr key={field} className="border-b border-slate-100 dark:border-slate-700/50">
                        <td className="py-2 font-mono text-xs">{field}</td>
                        <td className="py-2 text-xs text-slate-500 dark:text-slate-400">{type}</td>
                        <td className="py-2 text-xs text-slate-500 dark:text-slate-400">{desc}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Submit Transaction</h3>
              <CodeBlock lang="bash" code={`curl -X POST http://localhost:8545/submit_tx \\
  -H "Content-Type: application/json" \\
  -d '{
    "sender": [116,55,...],
    "to": [116,...],
    "nonce": 0,
    "value": 1000000000000000000,
    "gas_limit": 21000,
    "max_fee_per_gas": 1000000000,
    "max_priority_fee_per_gas": 100000000,
    "payload": [],
    "chain_id": 1,
    "signature": [1,2,...]
  }'`} />
            </section>
          )}

          {activeSection === 'rpc' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">RPC API</h2>
              <p className="text-slate-600 dark:text-slate-400 mb-6">Interact with the node via HTTP JSON-RPC on port 8545. All endpoints return JSON responses.</p>

              <div className="flex flex-wrap items-center gap-3 mb-6">
                <p className="text-sm text-slate-600 dark:text-slate-400">An OpenAPI 3.0 spec is available at <code className="text-xs text-blue-600 dark:text-blue-400">docs/openapi.yaml</code>.</p>
                <a href="/api-docs"
                  className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-blue-600/10 text-blue-600 dark:text-blue-400 text-xs font-medium hover:bg-blue-600/20 transition-all">
                  <FileJson className="w-3.5 h-3.5" />
                  Interactive API Docs
                </a>
              </div>

              <div className="space-y-4 mb-8">
                {[
                  { method: 'GET', path: '/status', desc: 'Node status: height, finalized height, mempool size, peer count.', real: '{"height":0,"finalized_height":0,"mempool_size":0,"peer_count":0}' },
                  { method: 'GET', path: '/health', desc: 'Health check (returns 200 when node is operational).', real: '{"status":"healthy","version":"0.1.0"}' },
                  { method: 'GET', path: '/balance/{address}', desc: 'Query account balance and nonce by 20-byte hex address.', real: '{"address":"0x...","balance":"1000000000000000000","nonce":0}' },
                  { method: 'GET', path: '/block/{height}', desc: 'Get block by number (use 0 for genesis).', real: '{"block":{...},"error":null}' },
                  { method: 'GET', path: '/block/latest', desc: 'Get the most recent block.', real: '{"block":{...},"error":null}' },
                  { method: 'GET', path: '/block/hash/{hash}', desc: 'Get block by 32-byte hex hash.', real: '{"block":{...},"error":null}' },
                  { method: 'GET', path: '/tx/{hash}', desc: 'Get transaction receipt by 32-byte hex hash.', real: '{"receipt":{...},"error":null}' },
                  { method: 'GET', path: '/mempool', desc: 'Pending transactions in the mempool queue.', real: '{"size":0,"transactions":[]}' },
                  { method: 'GET', path: '/gas_price', desc: 'EIP-1559 gas price suggestions (base + 3 priority tiers).', real: '{"base_fee":"1000000000","suggested_priority_fee_low":"1000000000","suggested_priority_fee_medium":"2000000000","suggested_priority_fee_high":"5000000000","block_height":0}' },
                  { method: 'GET', path: '/fee_history/{count}', desc: 'Historical base fee and gas used ratio for last N blocks (max 100).', real: '{"base_fee_per_gas":[],"gas_used_ratio":[],"oldest_block":0}' },
                  { method: 'POST', path: '/estimate_gas', desc: 'Estimate gas for a transaction. Body: {from, to, data}.', real: '{"estimated_gas":23100,"base_fee":"1000000000","total_cost_estimate":"23100000000000","estimated_priority_fee":"100000000"}' },
                  { method: 'GET', path: '/validators', desc: 'Active validator set from on-chain state.', real: '{"validators":[],"count":0}' },
                  { method: 'GET', path: '/delegations/{address}', desc: 'Staking delegations for an address.', real: '{"delegations":[],"address":"0x..."}' },
                  { method: 'GET', path: '/peers', desc: 'Connected peers list.', real: '{"peers":[],"count":0}' },
                  { method: 'POST', path: '/connect_peer', desc: 'Dial a peer by libp2p multiaddress. Body: {multiaddr}.', real: '{"status":"success"}' },
                  { method: 'POST', path: '/submit_tx', desc: 'Submit a signed transaction (see Transaction schema).', real: '{"status":"success","hash":"abc123..."}' },
                  { method: 'GET', path: '/metrics', desc: 'Prometheus-format metrics (text/plain).', real: '# HELP ... TYPE ... gauge ...' },
                ].map(({ method, path, desc, real }) => (
                  <div key={path} className="glass rounded-xl p-4">
                    <div className="flex items-start gap-3">
                      <span className={`shrink-0 px-2 py-0.5 rounded text-xs font-bold mt-0.5 ${
                        method === 'GET' ? 'bg-blue-100 dark:bg-blue-500/20 text-blue-600 dark:text-blue-400' : 'bg-emerald-100 dark:bg-emerald-500/20 text-emerald-600 dark:text-emerald-400'
                      }`}>{method}</span>
                      <div className="min-w-0 flex-1">
                        <code className="text-sm text-slate-700 dark:text-slate-200 font-semibold break-all">{path}</code>
                        <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">{desc}</p>
                        {real && (
                          <div className="mt-2 p-2 rounded-lg bg-slate-800/10 dark:bg-slate-700/40">
                            <code className="text-[10px] text-slate-600 dark:text-slate-400 break-all">{real}</code>
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
              </div>

              <div className="p-5 rounded-xl glass">
                <h3 className="text-sm font-semibold text-slate-800 dark:text-white mb-2">RPC URL Configuration</h3>
                <p className="text-sm text-slate-600 dark:text-slate-400">All frontends connect to the RPC endpoint. You can configure the URL in each app's settings panel. Default: <code className="text-xs text-blue-600 dark:text-blue-400">http://localhost:8545</code></p>
              </div>
            </section>
          )}

          {activeSection === 'errors' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">Error Codes</h2>
              <p className="text-slate-600 dark:text-slate-400 mb-6">The RPC API returns standardized error codes in JSON format.</p>

              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-slate-200 dark:border-slate-700">
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Code</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Description</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Typical Cause</th>
                    </tr>
                  </thead>
                  <tbody className="text-slate-700 dark:text-slate-300">
                    {[
                      ['400', 'Bad Request', 'Invalid address format, missing fields, or malformed JSON'],
                      ['401', 'Unauthorized', 'Signature verification failed or invalid nonce'],
                      ['404', 'Not Found', 'Block height not found, account has no state, or transaction unknown'],
                      ['429', 'Too Many Requests', 'Rate limit exceeded (configurable in node config)'],
                      ['500', 'Internal Error', 'Node state error or internal runtime failure'],
                      ['503', 'Service Unavailable', 'Node still syncing or in bootstrap mode'],
                    ].map(([code, desc, cause]) => (
                      <tr key={code} className="border-b border-slate-100 dark:border-slate-700/50">
                        <td className="py-2.5 font-mono text-xs text-red-600 dark:text-red-400 font-semibold">{code}</td>
                        <td className="py-2.5 text-xs">{desc}</td>
                        <td className="py-2.5 text-xs text-slate-500 dark:text-slate-400">{cause}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              <div className="mt-6 p-4 rounded-xl bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/20">
                <p className="text-xs text-red-600 dark:text-red-300">
                  <strong>Note:</strong> Error responses follow the format: <code className="text-xs">{'{"error":"message","code":400}'}</code>
                </p>
              </div>
            </section>
          )}

          {activeSection === 'local-dev' && (
            <section className="animate-slide-up">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-4">Local Development</h2>
              <p className="text-slate-600 dark:text-slate-400 mb-6">Everything you need to run a full local development environment.</p>

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Service Port Map</h3>
              <div className="overflow-x-auto mb-6">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-slate-200 dark:border-slate-700">
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Service</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Port</th>
                      <th className="text-left py-2 text-slate-500 dark:text-slate-400 font-medium">Description</th>
                    </tr>
                  </thead>
                  <tbody className="text-slate-700 dark:text-slate-300">
                    {[
                      ['RPC Node', '8545', 'Primary JSON-RPC endpoint for chain interactions'],
                      ['Faucet Backend', '3001', 'Test token distribution service'],
                      ['Frontend (Unified)', '5173', 'Wallet, explorer, governance, docs, portals'],
                      ['Metrics', '9090', 'Prometheus metrics endpoint'],
                      ['P2P', '26656', 'Libp2p peer-to-peer networking'],
                    ].map(([service, port, desc]) => (
                      <tr key={service} className="border-b border-slate-100 dark:border-slate-700/50">
                        <td className="py-2 text-xs font-medium text-slate-800 dark:text-slate-200">{service}</td>
                        <td className="py-2 font-mono text-xs text-blue-600 dark:text-blue-400">{port}</td>
                        <td className="py-2 text-xs text-slate-500 dark:text-slate-400">{desc}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Test Data</h3>
              <p className="text-sm text-slate-600 dark:text-slate-400 mb-3">These addresses are pre-loaded in the genesis state with test tokens:</p>
              <CodeBlock lang="text" code={`# Test Addresses (pre-funded)
0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18  — Primary test account
0x8fD8fB8fB8fB8fD8fB8fB8fD8fB8fB8fD8fB8fD  — Validator #2
0x5B38Da6a701c568545dCfcB03FcB875f56beddC4  — Validator #3

# Faucet
curl -X POST http://localhost:3006/faucet \\
  -H "Content-Type: application/json" \\
  -d '{"address":"0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18"}'`} />

              <h3 className="text-lg font-semibold text-slate-800 dark:text-white mb-2">Common Workflows</h3>
              <div className="space-y-3 mb-6">
                {[
                  ['Reset Devnet', `docker compose down -v && docker compose up -d\n# Removes all state and starts fresh`],
                  ['Check Logs', `docker compose logs -f validator-1`],
                  ['Query Balance', `curl http://localhost:8545/balance/0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18`],
                  ['Send Tokens', `# Use the wallet frontend at http://localhost:5173/wallet`],
                ].map(([title, cmd]) => (
                  <div key={title} className="p-4 rounded-xl glass">
                    <h4 className="text-sm font-semibold text-slate-800 dark:text-white mb-1">{title}</h4>
                    <code className="text-xs text-slate-600 dark:text-slate-400 block">{cmd}</code>
                  </div>
                ))}
              </div>
            </section>
          )}
        </div>
      </div>
    </div>
  )
}
