import {
  createPublicClient,
  createWalletClient,
  custom,
  http,
  parseEther,
  encodeFunctionData,
  padHex,
  toHex,
} from 'viem'
import { mainnet, sepolia, hardhat } from 'viem/chains'

export const BRIDGE_ABI = [
  {
    name: 'lockTokens',
    type: 'function',
    stateMutability: 'payable',
    inputs: [
      { name: 'token', type: 'address' },
      { name: 'amount', type: 'uint256' },
      { name: 'recipient', type: 'bytes32' },
    ],
    outputs: [{ name: 'messageId', type: 'uint64' }],
  },
  {
    name: 'nextMessageId',
    type: 'function',
    stateMutability: 'view',
    inputs: [],
    outputs: [{ type: 'uint64' }],
  },
]

const STORAGE = {
  ethRpc: 'nebula_eth_rpc_url',
  bridge: 'nebula_eth_bridge_address',
}

export function getEthRpcUrl() {
  return localStorage.getItem(STORAGE.ethRpc) || import.meta.env.VITE_ETH_RPC_URL || 'http://127.0.0.1:9545'
}

export function setEthRpcUrl(url) {
  localStorage.setItem(STORAGE.ethRpc, url)
}

export function getBridgeAddress() {
  return localStorage.getItem(STORAGE.bridge) || import.meta.env.VITE_BRIDGE_ADDRESS || ''
}

export function setBridgeAddress(addr) {
  localStorage.setItem(STORAGE.bridge, addr)
}

/** Load auto-generated config from /bridge-config.json (written by deploy-bridge.sh) */
export async function loadBridgeConfig() {
  try {
    const r = await window.fetch('/bridge-config.json')
    if (!r.ok) return null
    const cfg = await r.json()
    if (cfg.ethRpcUrl) setEthRpcUrl(cfg.ethRpcUrl)
    if (cfg.bridge) setBridgeAddress(cfg.bridge)
    return cfg
  } catch {
    return null
  }
}

function chainForRpc(rpcUrl, chainId) {
  if (chainId === 31337 || chainId === 1337) {
    return {
      ...hardhat,
      id: chainId || 31337,
      rpcUrls: { default: { http: [rpcUrl] } },
    }
  }
  if (chainId === 11155111) return sepolia
  return { ...mainnet, rpcUrls: { default: { http: [rpcUrl] } } }
}

export function createEthPublicClient(rpcUrl = getEthRpcUrl(), chainId) {
  const chain = chainForRpc(rpcUrl, chainId)
  return createPublicClient({ chain, transport: http(rpcUrl) })
}

export async function connectMetaMask() {
  if (!window.ethereum) throw new Error('MetaMask not installed')
  const accounts = await window.ethereum.request({ method: 'eth_requestAccounts' })
  const chainIdHex = await window.ethereum.request({ method: 'eth_chainId' })
  const chainId = parseInt(chainIdHex, 16)
  const rpcUrl = getEthRpcUrl()
  const chain = chainForRpc(rpcUrl, chainId)
  const walletClient = createWalletClient({
    chain,
    transport: custom(window.ethereum),
    account: accounts[0],
  })
  return { address: accounts[0], chainId, walletClient }
}

export function nebulaAddressToBytes32(addr) {
  const clean = addr.replace(/^0x/, '').toLowerCase().padStart(64, '0')
  return padHex(`0x${clean}`, { size: 32 })
}

export async function lockEthOnBridge({ bridgeAddress, amountEth, recipientNebula, walletClient, account }) {
  if (!bridgeAddress) throw new Error('Set Bridge.sol contract address')
  const data = encodeFunctionData({
    abi: BRIDGE_ABI,
    functionName: 'lockTokens',
    args: ['0x0000000000000000000000000000000000000000', parseEther(String(amountEth)), nebulaAddressToBytes32(recipientNebula)],
  })
  const hash = await walletClient.sendTransaction({
    account,
    to: bridgeAddress,
    value: parseEther(String(amountEth)),
    data,
  })
  return hash
}

export async function waitEthReceipt(publicClient, hash) {
  return publicClient.waitForTransactionReceipt({ hash })
}

export async function fetchEthBlockNumber(rpcUrl) {
  const client = createEthPublicClient(rpcUrl)
  return client.getBlockNumber()
}

export async function requestNebulaMint({ recipient, amount, ethTxHash, ethRpcUrl, ethBridgeAddress }) {
  const base = localStorage.getItem('nebula_rpc_url') || 'http://localhost:8545'
  const r = await window.fetch(`${base}/bridge/mint`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      recipient,
      amount: amount?.toString(),
      eth_tx_hash: ethTxHash,
      eth_rpc_url: ethRpcUrl || getEthRpcUrl(),
      eth_bridge_address: ethBridgeAddress || getBridgeAddress(),
    }),
  })
  const d = await r.json()
  if (d.error) throw new Error(d.error)
  return d
}

export async function fetchBridgeStatus() {
  const base = localStorage.getItem('nebula_rpc_url') || 'http://localhost:8545'
  const r = await window.fetch(`${base}/bridge/status`)
  if (!r.ok) throw new Error('Bridge status unavailable')
  return r.json()
}
