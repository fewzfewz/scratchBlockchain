import WebSocket from "ws";
import EventEmitter from "eventemitter3";
import { Provider, ProviderOptions } from "../types/provider";

interface PendingRequest {
  resolve: (value: any) => void;
  reject: (reason: any) => void;
  timer: ReturnType<typeof setTimeout>;
}

interface SubscriptionHandler {
  id: string;
  event: string;
  filter?: any;
}

export class WebSocketProvider extends EventEmitter implements Provider {
  private ws: WebSocket | null = null;
  private url: string;
  private options: ProviderOptions;
  private connected: boolean = false;
  private requestId: number = 1;
  private pending: Map<number, PendingRequest> = new Map();
  private subscriptions: Map<string, SubscriptionHandler> = new Map();
  private reconnectAttempts: number = 0;
  private maxReconnectAttempts: number = 10;
  private reconnectDelay: number = 1000;
  private disconnectRequested: boolean = false;

  // Polling fallback for subscriptions when WS isn't available
  private pollTimers: Map<string, ReturnType<typeof setInterval>> = new Map();

  constructor(url: string, options: ProviderOptions = {}) {
    super();
    this.url = url;
    this.options = options;
  }

  async request(method: string, params: any[] = []): Promise<any> {
    if (!this.connected || !this.ws) {
      await this.connect();
    }

    return new Promise<any>((resolve, reject) => {
      const id = this.requestId++;
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`Request timed out: ${method}`));
      }, this.options.timeout || 30000);

      this.pending.set(id, { resolve, reject, timer });

      const message = JSON.stringify({
        jsonrpc: "2.0",
        id,
        method,
        params,
      });

      try {
        this.ws!.send(message);
      } catch (err) {
        this.pending.delete(id);
        clearTimeout(timer);
        reject(err);
      }
    });
  }

  async connect(): Promise<void> {
    if (this.ws && this.connected) return;

    this.disconnectRequested = false;

    return new Promise<void>((resolve, reject) => {
      try {
        this.ws = new WebSocket(this.url);

        const timeout = setTimeout(() => {
          reject(new Error(`WebSocket connection timeout: ${this.url}`));
        }, this.options.timeout || 10000);

        this.ws.on("open", () => {
          clearTimeout(timeout);
          this.connected = true;
          this.reconnectAttempts = 0;
          this.emit("connected");
          resolve();
        });

        this.ws.on("message", (data: WebSocket.Data) => {
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

        this.ws.on("error", (err: Error) => {
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

  async disconnect(): Promise<void> {
    this.disconnectRequested = true;

    // Clear polling timers
    for (const timer of this.pollTimers.values()) {
      clearInterval(timer);
    }
    this.pollTimers.clear();

    // Clear subscriptions
    this.subscriptions.clear();

    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }

    this.connected = false;
    this.emit("disconnected");
  }

  // Subscriptions
  async subscribe(
    event: string,
    filter?: any,
  ): Promise<string> {
    const id = `${event}_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    const handler: SubscriptionHandler = { id, event, filter };

    if (this.connected) {
      try {
        const result = await this.request("eth_subscribe", [event, filter]);
        handler.id = result || id;
      } catch {
        // WS subscription failed, fall back to polling
        this.startPolling(handler);
      }
    } else {
      this.startPolling(handler);
    }

    this.subscriptions.set(handler.id, handler);

    this.on(event, (...args: any[]) => {
      this.emit(`subscription:${handler.id}`, ...args);
    });

    return handler.id;
  }

  async unsubscribe(subscriptionId: string): Promise<boolean> {
    const handler = this.subscriptions.get(subscriptionId);
    if (!handler) return false;

    // Stop polling if active
    const timer = this.pollTimers.get(subscriptionId);
    if (timer) {
      clearInterval(timer);
      this.pollTimers.delete(subscriptionId);
    }

    if (this.connected) {
      try {
        await this.request("eth_unsubscribe", [subscriptionId]);
      } catch {
        // ignore
      }
    }

    this.subscriptions.delete(subscriptionId);
    return true;
  }

  // Event listener interface
  on(event: string | symbol, fn: (...args: any[]) => void, context?: any): this {
    return super.on(event, fn, context);
  }

  off(event: string | symbol, fn?: ((...args: any[]) => void) | undefined): this {
    return super.off(event, fn);
  }

  // Connection state
  isConnected(): boolean {
    return this.connected;
  }

  getUrl(): string {
    return this.url;
  }

  // Private helpers

  private handleMessage(data: WebSocket.Data): void {
    try {
      const text = data.toString();
      const msg = JSON.parse(text);

      // JSON-RPC response
      if (msg.id !== undefined && this.pending.has(msg.id)) {
        const pending = this.pending.get(msg.id)!;
        clearTimeout(pending.timer);
        this.pending.delete(msg.id);

        if (msg.error) {
          pending.reject(new Error(msg.error.message || "RPC Error"));
        } else {
          pending.resolve(msg.result);
        }
        return;
      }

      // Subscription notification
      if (msg.method === "eth_subscription" && msg.params) {
        const subId = msg.params.subscription;
        const result = msg.params.result;
        const handler = this.subscriptions.get(subId);
        if (handler) {
          this.emit(handler.event, result);
        }
      }
    } catch {
      // Ignore malformed messages
    }
  }

  private startPolling(handler: SubscriptionHandler): void {
    const interval = setInterval(async () => {
      try {
        let result: any;

        switch (handler.event) {
          case "newBlocks": {
            const status = await this.request("block_number");
            result = { height: status?.height || 0 };
            break;
          }
          case "newTransactions": {
            const mempool = await this.request("get_mempool");
            result = { transactions: mempool?.transactions || [] };
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
        // Ignore polling errors
      }
    }, 2000);

    this.pollTimers.set(handler.id, interval);
  }

  private attemptReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      this.emit("error", new Error("Max reconnection attempts reached"));
      return;
    }

    const delay = Math.min(
      this.reconnectDelay * Math.pow(2, this.reconnectAttempts),
      30000,
    );

    this.reconnectAttempts++;
    this.emit("reconnecting", {
      attempt: this.reconnectAttempts,
      maxAttempts: this.maxReconnectAttempts,
      delay,
    });

    setTimeout(async () => {
      try {
        await this.connect();
        // Re-subscribe after reconnection
        for (const [, handler] of this.subscriptions) {
          try {
            await this.subscribe(handler.event, handler.filter);
          } catch {
            // Resume polling for failed subscriptions
            this.startPolling(handler);
          }
        }
        this.emit("reconnected");
      } catch {
        this.attemptReconnect();
      }
    }, delay);
  }

  private rejectAllPending(error: Error): void {
    for (const [, pending] of this.pending) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.pending.clear();
  }
}
