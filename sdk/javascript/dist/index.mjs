"use strict";

// src/client.ts
import EventEmitter from "eventemitter3";
var ModularClient = class extends EventEmitter {
  constructor(provider, options = {}) {
    super();
    this.connected = false;
    this.provider = provider;
    this.options = options;
  }
  async connect() {
    const chainId = await this.getChainId();
    if (this.options.chainId && chainId !== this.options.chainId) {
      throw new Error(
        `Chain ID mismatch: expected ${this.options.chainId}, got ${chainId}`
      );
    }
    this.connected = true;
    this.emit("connected", chainId);
  }
  isConnected() {
    return this.connected;
  }
  async disconnect() {
    this.connected = false;
    this.emit("disconnected");
  }
  // Chain info
  async getChainId() {
    const status = await this.provider.request("status");
    return status.chain_id ?? 1;
  }
  async getBlockNumber() {
    const status = await this.provider.request("block_number");
    return status.height || 0;
  }
  async getBlock(blockNumber) {
    return await this.provider.request("get_block", [blockNumber]);
  }
  async getBlockByHash(hash) {
    const data = await this.provider.request("get_block_by_hash", [hash]);
    if (data.error)
      throw new Error(data.error);
    return data.block;
  }
  async getLatestBlock() {
    const data = await this.provider.request("get_latest_block");
    if (data.error)
      throw new Error(data.error);
    return data.block;
  }
  async getTxHistory(address, limit = 50) {
    const response = await this.provider.request("get_tx_history", [address, limit]);
    return response.transactions || [];
  }
  // Account
  async getBalance(address) {
    const response = await this.provider.request("get_balance", [address]);
    return response.balance || "0";
  }
  async getAccount(address) {
    const response = await this.provider.request("get_account", [address]);
    return {
      address,
      balance: response.balance || "0",
      nonce: response.nonce || 0
    };
  }
  async getNonce(address) {
    const account = await this.getAccount(address);
    return account.nonce;
  }
  // Transactions
  async sendTransaction(tx) {
    const response = await this.provider.request("submit_tx", [tx]);
    if (response.status && response.status.startsWith("error")) {
      throw new Error(response.status);
    }
    return response;
  }
  async sendRawTransaction(signedTx) {
    const tx = JSON.parse(signedTx);
    return await this.sendTransaction(tx);
  }
  async getTransaction(hash) {
    try {
      const data = await this.provider.request("get_transaction", [hash]);
      if (data.error)
        return null;
      return data.receipt;
    } catch {
      return null;
    }
  }
  async getTransactionReceipt(hash) {
    try {
      const data = await this.provider.request("get_transaction_receipt", [hash]);
      if (data.error || !data.receipt)
        return null;
      return data.receipt;
    } catch {
      return null;
    }
  }
  async waitForTransaction(hash, confirmations = 1, timeout = 6e4) {
    const startTime = Date.now();
    while (Date.now() - startTime < timeout) {
      const receipt = await this.getTransactionReceipt(hash);
      if (receipt) {
        const currentBlock = await this.getBlockNumber();
        const confirms = currentBlock - receipt.block_height + 1;
        if (confirms >= confirmations) {
          this.emit("transactionConfirmed", receipt);
          return receipt;
        }
        this.emit("transactionPending", {
          hash,
          confirmations: confirms,
          required: confirmations
        });
      }
      await new Promise((resolve) => setTimeout(resolve, 1e3));
    }
    throw new Error(`Transaction ${hash} not confirmed within ${timeout}ms`);
  }
  // Gas & Fees
  async getGasPrice() {
    return await this.provider.request("gas_price");
  }
  async estimateGas(tx) {
    return await this.provider.request("estimate_gas", [tx]);
  }
  async getFeeHistory(blockCount) {
    return await this.provider.request("fee_history", [blockCount]);
  }
  // Mempool
  async getMempool(_limit = 100) {
    const response = await this.provider.request("get_mempool");
    return response;
  }
  async getMempoolSize() {
    const mempool = await this.getMempool(1);
    return mempool.size || 0;
  }
  // Network
  async getPeers() {
    const response = await this.provider.request("get_peers");
    return response.peers || [];
  }
  async connectPeer(multiaddr) {
    await this.provider.request("connect_peer", [{ multiaddr }]);
  }
  // Node Status
  async getNodeStatus() {
    return await this.provider.request("status");
  }
  async getMetrics() {
    return await this.provider.request("get_metrics");
  }
  async healthCheck() {
    try {
      const response = await this.provider.request("health");
      return response.status === "healthy";
    } catch {
      return false;
    }
  }
  // Governance (read from on-chain state)
  async getProposals() {
    const response = await this.provider.request("get_governance");
    return response.proposals || [];
  }
  async getProposal(id) {
    try {
      const response = await this.provider.request("get_proposal", [id]);
      return response.proposal || null;
    } catch {
      return null;
    }
  }
  /** Governance writes use signed POST /submit_tx — use Wallet.signTransaction with governance payload. */
  async createProposal(_req) {
    throw new Error("Use Wallet.signTransaction with governance payload and client.sendTransaction()");
  }
  async vote(_req) {
    throw new Error("Use Wallet.signTransaction with governance vote payload and client.sendTransaction()");
  }
  async getVotes(_proposalId) {
    const proposal = await this.getProposal(_proposalId);
    if (!proposal || !proposal.voters)
      return [];
    return Object.entries(proposal.voters).map(([voter, choice]) => ({
      proposalId: _proposalId,
      voter,
      support: String(choice) || "Abstain",
      weight: "0",
      timestamp: 0
    }));
  }
  async getTreasury() {
    try {
      const response = await this.provider.request("get_governance");
      return response.treasury || null;
    } catch {
      return null;
    }
  }
  async getGovParams() {
    try {
      const response = await this.provider.request("get_governance");
      return response.params || null;
    } catch {
      return null;
    }
  }
  async getDelegations(address) {
    const response = await this.provider.request("get_delegations", [address]);
    return response.delegations || [];
  }
  async delegate(delegator, validator, amount) {
    return await this.provider.request("delegate_stake", [{ delegator, validator, amount }]);
  }
  async undelegate(_delegator, _validator, _amount) {
    throw new Error("Undelegate via signed governance/staking transaction (POST /submit_tx)");
  }
  async getValidators() {
    const response = await this.provider.request("get_validators");
    return response.validators || response || [];
  }
  async registerValidator(req) {
    return await this.provider.request("register_validator", [req]);
  }
  async executeProposal(_proposalId) {
    throw new Error("Execute via signed governance transaction (POST /submit_tx)");
  }
  async requestFaucet(address, amount) {
    return await this.provider.request("faucet_request", [{ address, amount }]);
  }
  async getSlashingEvents() {
    const response = await this.provider.request("get_slashing_events");
    return response.events || [];
  }
  // WASM contracts
  async deployWasm(name, wasmBase64) {
    return await this.provider.request("deploy_wasm", [{ name, wasm: wasmBase64 }]);
  }
  async callWasm(name, func, arg = 0) {
    return await this.provider.request("call_wasm", [{ name, func, arg }]);
  }
  async listWasmContracts() {
    const response = await this.provider.request("list_wasm_contracts");
    return response.contracts || [];
  }
  // Account abstraction & MEV
  async submitUserOperation(op) {
    return await this.provider.request("submit_user_operation", [op]);
  }
  async getPendingUserOperations() {
    const response = await this.provider.request("pending_user_ops");
    return response.pending ?? response.count ?? 0;
  }
  async getGovStats() {
    try {
      const [proposals, validators, params] = await Promise.all([
        this.getProposals(),
        this.getValidators(),
        this.getGovParams()
      ]);
      const activeProposals = proposals.filter((p) => p.status === "Active");
      const activeValidators = validators.filter((v) => v.isActive);
      const totalStaked = validators.reduce((sum, v) => sum + BigInt(v.stake || "0"), 0n).toString();
      const totalVotes = proposals.reduce((sum, p) => sum + BigInt(p.yesVotes || "0") + BigInt(p.noVotes || "0"), 0n).toString();
      return {
        totalProposals: proposals.length,
        activeProposals: activeProposals.length,
        totalVotes: Number(totalVotes),
        totalDelegators: 0,
        totalStaked,
        votingPower: "0",
        inflationRate: params ? 100 / Number(params.votingPeriod) * 365 * 24 * 60 * 60 / 6 : 0,
        activeValidators: activeValidators.length
      };
    } catch {
      return null;
    }
  }
  // Utilities
  getProvider() {
    return this.provider;
  }
};

// src/wallet.ts
import { ed25519 } from "@noble/curves/ed25519";
import { sha256 } from "@noble/hashes/sha256";
import { bytesToHex, hexToBytes } from "@noble/hashes/utils";
import { generateMnemonic, mnemonicToSeedSync } from "bip39";
function uint64LE(value) {
  const buf = new ArrayBuffer(8);
  const view = new DataView(buf);
  view.setBigUint64(0, BigInt(value), true);
  return new Uint8Array(buf);
}
var Wallet = class _Wallet {
  constructor(privateKey) {
    if (typeof privateKey === "string") {
      const cleanKey = privateKey.startsWith("0x") ? privateKey.slice(2) : privateKey;
      this.privateKey = hexToBytes(cleanKey);
    } else {
      this.privateKey = privateKey;
    }
    const pubKey = ed25519.getPublicKey(this.privateKey);
    this.publicKey = bytesToHex(pubKey);
    const hash = sha256(pubKey);
    this.address = "0x" + bytesToHex(hash.slice(-20));
  }
  static generate() {
    const privateKey = ed25519.utils.randomPrivateKey();
    return new _Wallet(privateKey);
  }
  static fromPrivateKey(privateKey) {
    return new _Wallet(privateKey);
  }
  static fromMnemonic(mnemonic, _index = 0) {
    const seed = mnemonicToSeedSync(mnemonic);
    const privateKey = seed.slice(0, 32);
    return new _Wallet(privateKey);
  }
  static generateMnemonic() {
    return generateMnemonic();
  }
  async signMessage(message) {
    const msgBytes = new TextEncoder().encode(message);
    const h = sha256(msgBytes);
    const signature = ed25519.sign(h, this.privateKey);
    return bytesToHex(signature);
  }
  async signTransaction(tx) {
    const sender = this.address;
    const nonce = tx.nonce ?? 0;
    const rawPayload = tx.data ? hexToBytes(tx.data.startsWith("0x") ? tx.data.slice(2) : tx.data) : new Uint8Array();
    const gasLimit = Number(tx.gasLimit ?? 21e3);
    const maxFeePerGas = Number(tx.maxFeePerGas ?? 1e9);
    const maxPriorityFeePerGas = Number(tx.maxPriorityFeePerGas ?? 1e8);
    const chainId = tx.chainId ?? null;
    const to = tx.to ?? null;
    const value = Number(tx.value ?? 0);
    const parts = [
      hexToBytes(sender.startsWith("0x") ? sender.slice(2) : sender),
      uint64LE(nonce),
      rawPayload,
      uint64LE(gasLimit),
      uint64LE(maxFeePerGas),
      uint64LE(maxPriorityFeePerGas)
    ];
    if (chainId !== null)
      parts.push(uint64LE(chainId));
    if (to !== null) {
      parts.push(hexToBytes(to.startsWith("0x") ? to.slice(2) : to));
    }
    parts.push(uint64LE(value));
    const total = parts.reduce((sum, p) => sum + p.length, 0);
    const message = new Uint8Array(total);
    let offset = 0;
    for (const p of parts) {
      message.set(p, offset);
      offset += p.length;
    }
    const hash = sha256(message);
    const signatureBytes = ed25519.sign(hash, this.privateKey);
    return {
      sender,
      nonce,
      payload: "0x" + bytesToHex(rawPayload),
      signature: "0x" + bytesToHex(signatureBytes),
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas,
      chainId,
      to: to ?? null,
      value,
      hash: "0x" + bytesToHex(hash)
    };
  }
  connect(provider) {
    return new ConnectedWallet(this.privateKey, provider);
  }
  getPrivateKey() {
    return bytesToHex(this.privateKey);
  }
  getPublicKey() {
    return this.publicKey;
  }
  getAddress() {
    return this.address;
  }
  static verifySignature(message, signature, publicKey) {
    try {
      const msgBytes = new TextEncoder().encode(message);
      const h = sha256(msgBytes);
      const sigBytes = hexToBytes(signature);
      const pkBytes = hexToBytes(publicKey);
      return ed25519.verify(sigBytes, h, pkBytes);
    } catch {
      return false;
    }
  }
};
var ConnectedWallet = class {
  constructor(privateKey, provider) {
    this.wallet = new Wallet(privateKey);
    this.client = new ModularClient(provider);
  }
  async sendTransaction(tx) {
    if (!tx.nonce) {
      tx.nonce = await this.getNonce();
    }
    const signedTx = await this.wallet.signTransaction(tx);
    return await this.client.sendTransaction(signedTx);
  }
  async getBalance() {
    return await this.client.getBalance(this.address);
  }
  get address() {
    return this.wallet.address;
  }
  async getNonce() {
    return await this.client.getNonce(this.address);
  }
  getClient() {
    return this.client;
  }
};

// src/providers/http.ts
import axios from "axios";
var HttpProvider = class {
  constructor(url, options = {}) {
    this.isConnected = false;
    this.url = url.replace(/\/+$/, "");
    this.client = axios.create({
      baseURL: this.url,
      timeout: options.timeout || 3e4,
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        ...options.headers
      }
    });
  }
  async request(method, params = []) {
    try {
      const ep = this.mapMethodToEndpoint(method);
      let response;
      if (ep.method === "GET") {
        const url = this.buildGetUrl(ep.path, params, method);
        response = await this.client.get(url);
      } else {
        response = await this.client.post(ep.path, params[0] || {});
      }
      this.isConnected = true;
      if (response.data && typeof response.data === "object") {
        if (response.data.error) {
          throw new Error(response.data.error);
        }
        return response.data;
      }
      return response.data;
    } catch (error) {
      this.isConnected = false;
      if (axios.isAxiosError(error)) {
        throw new Error(`HTTP Error: ${error.message}`);
      }
      throw error;
    }
  }
  mapMethodToEndpoint(method) {
    const get = {
      chain_id: "/status",
      block_number: "/status",
      get_block: "/block/",
      get_block_by_hash: "/block/hash/",
      get_latest_block: "/block/latest",
      get_balance: "/balance/",
      get_account: "/balance/",
      get_transaction: "/tx/",
      get_transaction_receipt: "/tx/",
      get_tx_history: "/txs/",
      gas_price: "/gas_price",
      get_mempool: "/mempool",
      get_peers: "/peers",
      get_metrics: "/metrics",
      status: "/status",
      health: "/health",
      fee_history: "/fee_history/",
      get_governance: "/governance",
      get_proposals: "/governance",
      get_proposal: "/proposal/",
      get_treasury: "/governance",
      get_gov_params: "/governance",
      get_delegations: "/delegations/",
      get_validators: "/validators",
      get_slashing_events: "/slashing/events",
      list_wasm_contracts: "/wasm/contracts",
      pending_user_ops: "/user_operations/pending"
    };
    const post = {
      send_transaction: "/submit_tx",
      submit_tx: "/submit_tx",
      send_raw_transaction: "/submit_tx",
      connect_peer: "/connect_peer",
      estimate_gas: "/estimate_gas",
      delegate_stake: "/delegate",
      register_validator: "/validators/register",
      faucet_request: "/faucet/request",
      deploy_wasm: "/deploy_wasm",
      call_wasm: "/call_wasm",
      submit_user_operation: "/submit_user_operation",
      mev_commit: "/mev/commit",
      mev_reveal: "/mev/reveal",
      mev_encrypted: "/mev/encrypted",
      mev_decryption_share: "/mev/decryption_share"
    };
    if (post[method]) {
      return { method: "POST", path: post[method] };
    }
    return { method: "GET", path: get[method] || "/" };
  }
  buildGetUrl(basePath, params, method) {
    if (params.length === 0)
      return basePath;
    const param = params[0];
    if (param === void 0 || param === null)
      return basePath;
    const encoded = typeof param === "string" ? encodeURIComponent(param) : String(param);
    let url = basePath + encoded;
    if (method === "get_tx_history" && params[1] !== void 0) {
      url += `?limit=${encodeURIComponent(String(params[1]))}`;
    }
    return url;
  }
  getUrl() {
    return this.url;
  }
  isConnectedToNode() {
    return this.isConnected;
  }
};

// src/providers/websocket.ts
import WebSocket from "ws";
import EventEmitter2 from "eventemitter3";
var WebSocketProvider = class extends EventEmitter2 {
  constructor(url, options = {}) {
    super();
    this.ws = null;
    this.connected = false;
    this.requestId = 1;
    this.pending = /* @__PURE__ */ new Map();
    this.subscriptions = /* @__PURE__ */ new Map();
    this.reconnectAttempts = 0;
    this.maxReconnectAttempts = 10;
    this.reconnectDelay = 1e3;
    this.disconnectRequested = false;
    // Polling fallback for subscriptions when WS isn't available
    this.pollTimers = /* @__PURE__ */ new Map();
    this.url = url;
    this.options = options;
  }
  async request(method, params = []) {
    if (!this.connected || !this.ws) {
      await this.connect();
    }
    return new Promise((resolve, reject) => {
      const id = this.requestId++;
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`Request timed out: ${method}`));
      }, this.options.timeout || 3e4);
      this.pending.set(id, { resolve, reject, timer });
      const message = JSON.stringify({
        jsonrpc: "2.0",
        id,
        method,
        params
      });
      try {
        this.ws.send(message);
      } catch (err) {
        this.pending.delete(id);
        clearTimeout(timer);
        reject(err);
      }
    });
  }
  async connect() {
    if (this.ws && this.connected)
      return;
    this.disconnectRequested = false;
    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(this.url);
        const timeout = setTimeout(() => {
          reject(new Error(`WebSocket connection timeout: ${this.url}`));
        }, this.options.timeout || 1e4);
        this.ws.on("open", () => {
          clearTimeout(timeout);
          this.connected = true;
          this.reconnectAttempts = 0;
          this.emit("connected");
          resolve();
        });
        this.ws.on("message", (data) => {
          this.handleMessage(data);
        });
        this.ws.on("close", () => {
          this.connected = false;
          this.emit("disconnected");
          this.rejectAllPending(new Error("WebSocket closed"));
          if (!this.disconnectRequested) {
            this.attemptReconnect();
          }
        });
        this.ws.on("error", (err) => {
          clearTimeout(timeout);
          this.connected = false;
          this.emit("error", err);
          reject(err);
        });
      } catch (err) {
        reject(err);
      }
    });
  }
  async disconnect() {
    this.disconnectRequested = true;
    for (const timer of this.pollTimers.values()) {
      clearInterval(timer);
    }
    this.pollTimers.clear();
    this.subscriptions.clear();
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.connected = false;
    this.emit("disconnected");
  }
  // Subscriptions
  async subscribe(event, filter) {
    const id = `${event}_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    const handler = { id, event, filter };
    if (this.connected) {
      try {
        const result = await this.request("eth_subscribe", [event, filter]);
        handler.id = result || id;
      } catch {
        this.startPolling(handler);
      }
    } else {
      this.startPolling(handler);
    }
    this.subscriptions.set(handler.id, handler);
    this.on(event, (...args) => {
      this.emit(`subscription:${handler.id}`, ...args);
    });
    return handler.id;
  }
  async unsubscribe(subscriptionId) {
    const handler = this.subscriptions.get(subscriptionId);
    if (!handler)
      return false;
    const timer = this.pollTimers.get(subscriptionId);
    if (timer) {
      clearInterval(timer);
      this.pollTimers.delete(subscriptionId);
    }
    if (this.connected) {
      try {
        await this.request("eth_unsubscribe", [subscriptionId]);
      } catch {
      }
    }
    this.subscriptions.delete(subscriptionId);
    return true;
  }
  // Event listener interface
  on(event, fn, context) {
    return super.on(event, fn, context);
  }
  off(event, fn) {
    return super.off(event, fn);
  }
  // Connection state
  isConnected() {
    return this.connected;
  }
  getUrl() {
    return this.url;
  }
  // Private helpers
  handleMessage(data) {
    try {
      const text = data.toString();
      const msg = JSON.parse(text);
      if (msg.id !== void 0 && this.pending.has(msg.id)) {
        const pending = this.pending.get(msg.id);
        clearTimeout(pending.timer);
        this.pending.delete(msg.id);
        if (msg.error) {
          pending.reject(new Error(msg.error.message || "RPC Error"));
        } else {
          pending.resolve(msg.result);
        }
        return;
      }
      if (msg.method === "eth_subscription" && msg.params) {
        const subId = msg.params.subscription;
        const result = msg.params.result;
        const handler = this.subscriptions.get(subId);
        if (handler) {
          this.emit(handler.event, result);
        }
      }
    } catch {
    }
  }
  startPolling(handler) {
    const interval = setInterval(async () => {
      try {
        let result;
        switch (handler.event) {
          case "newBlocks": {
            const status = await this.request("block_number");
            result = { height: (status == null ? void 0 : status.height) || 0 };
            break;
          }
          case "newTransactions": {
            const mempool = await this.request("get_mempool");
            result = { transactions: (mempool == null ? void 0 : mempool.transactions) || [] };
            break;
          }
          case "logs": {
            result = { logs: [] };
            break;
          }
          default:
            result = {};
        }
        this.emit(handler.event, result);
      } catch {
      }
    }, 2e3);
    this.pollTimers.set(handler.id, interval);
  }
  attemptReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      this.emit("error", new Error("Max reconnection attempts reached"));
      return;
    }
    const delay = Math.min(
      this.reconnectDelay * Math.pow(2, this.reconnectAttempts),
      3e4
    );
    this.reconnectAttempts++;
    this.emit("reconnecting", {
      attempt: this.reconnectAttempts,
      maxAttempts: this.maxReconnectAttempts,
      delay
    });
    setTimeout(async () => {
      try {
        await this.connect();
        for (const [, handler] of this.subscriptions) {
          try {
            await this.subscribe(handler.event, handler.filter);
          } catch {
            this.startPolling(handler);
          }
        }
        this.emit("reconnected");
      } catch {
        this.attemptReconnect();
      }
    }, delay);
  }
  rejectAllPending(error) {
    for (const [, pending] of this.pending) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.pending.clear();
  }
};

// src/types/client.ts
var EMPTY_HASH = "0x0000000000000000000000000000000000000000000000000000000000000000";
var EMPTY_ADDRESS = "0x0000000000000000000000000000000000000000";
var NATIVE_TOKEN_SYMBOL = "NBL";
var NATIVE_TOKEN_DECIMALS = 18;
export {
  ConnectedWallet,
  EMPTY_ADDRESS,
  EMPTY_HASH,
  HttpProvider,
  ModularClient,
  NATIVE_TOKEN_DECIMALS,
  NATIVE_TOKEN_SYMBOL,
  Wallet,
  WebSocketProvider
};
//# sourceMappingURL=index.mjs.map