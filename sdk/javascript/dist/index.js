"use strict";
"use strict";
var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// src/index.ts
var src_exports = {};
__export(src_exports, {
  ConnectedWallet: () => ConnectedWallet,
  EMPTY_ADDRESS: () => EMPTY_ADDRESS,
  EMPTY_HASH: () => EMPTY_HASH,
  HttpProvider: () => HttpProvider,
  ModularClient: () => ModularClient,
  NATIVE_TOKEN_DECIMALS: () => NATIVE_TOKEN_DECIMALS,
  NATIVE_TOKEN_SYMBOL: () => NATIVE_TOKEN_SYMBOL,
  Wallet: () => Wallet,
  WebSocketProvider: () => WebSocketProvider
});
module.exports = __toCommonJS(src_exports);

// src/client.ts
var import_eventemitter3 = __toESM(require("eventemitter3"));
var ModularClient = class extends import_eventemitter3.default {
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
    return 1;
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
    const blockNumber = await this.getBlockNumber();
    return await this.getBlock(blockNumber);
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
  // Utilities
  getProvider() {
    return this.provider;
  }
};

// src/wallet.ts
var import_ed25519 = require("@noble/curves/ed25519");
var import_sha256 = require("@noble/hashes/sha256");
var import_utils = require("@noble/hashes/utils");
var import_bip39 = require("bip39");
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
      this.privateKey = (0, import_utils.hexToBytes)(cleanKey);
    } else {
      this.privateKey = privateKey;
    }
    const pubKey = import_ed25519.ed25519.getPublicKey(this.privateKey);
    this.publicKey = (0, import_utils.bytesToHex)(pubKey);
    const hash = (0, import_sha256.sha256)(pubKey);
    this.address = "0x" + (0, import_utils.bytesToHex)(hash.slice(-20));
  }
  static generate() {
    const privateKey = import_ed25519.ed25519.utils.randomPrivateKey();
    return new _Wallet(privateKey);
  }
  static fromPrivateKey(privateKey) {
    return new _Wallet(privateKey);
  }
  static fromMnemonic(mnemonic, _index = 0) {
    const seed = (0, import_bip39.mnemonicToSeedSync)(mnemonic);
    const privateKey = seed.slice(0, 32);
    return new _Wallet(privateKey);
  }
  static generateMnemonic() {
    return (0, import_bip39.generateMnemonic)();
  }
  async signMessage(message) {
    const msgBytes = new TextEncoder().encode(message);
    const h = (0, import_sha256.sha256)(msgBytes);
    const signature = import_ed25519.ed25519.sign(h, this.privateKey);
    return (0, import_utils.bytesToHex)(signature);
  }
  async signTransaction(tx) {
    const sender = this.address;
    const nonce = tx.nonce ?? 0;
    const rawPayload = tx.data ? (0, import_utils.hexToBytes)(tx.data.startsWith("0x") ? tx.data.slice(2) : tx.data) : new Uint8Array();
    const gasLimit = Number(tx.gasLimit ?? 21e3);
    const maxFeePerGas = Number(tx.maxFeePerGas ?? 1e9);
    const maxPriorityFeePerGas = Number(tx.maxPriorityFeePerGas ?? 1e8);
    const chainId = tx.chainId ?? null;
    const to = tx.to ?? null;
    const value = Number(tx.value ?? 0);
    const parts = [
      (0, import_utils.hexToBytes)(sender.startsWith("0x") ? sender.slice(2) : sender),
      uint64LE(nonce),
      rawPayload,
      uint64LE(gasLimit),
      uint64LE(maxFeePerGas),
      uint64LE(maxPriorityFeePerGas)
    ];
    if (chainId !== null)
      parts.push(uint64LE(chainId));
    if (to !== null) {
      parts.push((0, import_utils.hexToBytes)(to.startsWith("0x") ? to.slice(2) : to));
    }
    parts.push(uint64LE(value));
    const total = parts.reduce((sum, p) => sum + p.length, 0);
    const message = new Uint8Array(total);
    let offset = 0;
    for (const p of parts) {
      message.set(p, offset);
      offset += p.length;
    }
    const hash = (0, import_sha256.sha256)(message);
    const signatureBytes = import_ed25519.ed25519.sign(hash, this.privateKey);
    return {
      sender,
      nonce,
      payload: "0x" + (0, import_utils.bytesToHex)(rawPayload),
      signature: "0x" + (0, import_utils.bytesToHex)(signatureBytes),
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas,
      chainId,
      to: to ?? null,
      value,
      hash: "0x" + (0, import_utils.bytesToHex)(hash)
    };
  }
  connect(provider) {
    return new ConnectedWallet(this.privateKey, provider);
  }
  getPrivateKey() {
    return (0, import_utils.bytesToHex)(this.privateKey);
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
      const h = (0, import_sha256.sha256)(msgBytes);
      const sigBytes = (0, import_utils.hexToBytes)(signature);
      const pkBytes = (0, import_utils.hexToBytes)(publicKey);
      return import_ed25519.ed25519.verify(sigBytes, h, pkBytes);
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
var import_axios = __toESM(require("axios"));
var HttpProvider = class {
  constructor(url, options = {}) {
    this.isConnected = false;
    this.url = url.replace(/\/+$/, "");
    this.client = import_axios.default.create({
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
        const url = this.buildGetUrl(ep.path, params);
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
      if (import_axios.default.isAxiosError(error)) {
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
      get_balance: "/balance/",
      get_account: "/balance/",
      get_transaction: "/tx/",
      get_transaction_receipt: "/tx/",
      estimate_gas: "/estimate_gas",
      gas_price: "/gas_price",
      get_mempool: "/mempool",
      get_peers: "/peers",
      get_metrics: "/metrics",
      status: "/status",
      health: "/health",
      fee_history: "/fee_history/"
    };
    const post = {
      send_transaction: "/submit_tx",
      submit_tx: "/submit_tx",
      send_raw_transaction: "/submit_tx",
      connect_peer: "/connect_peer",
      estimate_gas_post: "/estimate_gas"
    };
    if (post[method]) {
      return { method: "POST", path: post[method] };
    }
    return { method: "GET", path: get[method] || "/" };
  }
  buildGetUrl(basePath, params) {
    if (params.length === 0)
      return basePath;
    const param = params[0];
    if (param === void 0 || param === null)
      return basePath;
    const encoded = typeof param === "string" ? encodeURIComponent(param) : String(param);
    return basePath + encoded;
  }
  getUrl() {
    return this.url;
  }
  isConnectedToNode() {
    return this.isConnected;
  }
};

// src/providers/websocket.ts
var import_ws = __toESM(require("ws"));
var import_eventemitter32 = __toESM(require("eventemitter3"));
var WebSocketProvider = class extends import_eventemitter32.default {
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
        this.ws = new import_ws.default(this.url);
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
// Annotate the CommonJS export names for ESM import in node:
0 && (module.exports = {
  ConnectedWallet,
  EMPTY_ADDRESS,
  EMPTY_HASH,
  HttpProvider,
  ModularClient,
  NATIVE_TOKEN_DECIMALS,
  NATIVE_TOKEN_SYMBOL,
  Wallet,
  WebSocketProvider
});
//# sourceMappingURL=index.js.map