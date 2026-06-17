import axios, { AxiosInstance } from "axios";
import { Provider, ProviderOptions } from "../types/provider";

interface EndpointInfo {
  method: "GET" | "POST";
  path: string;
}

export class HttpProvider implements Provider {
  private client: AxiosInstance;
  private url: string;
  private isConnected: boolean = false;

  constructor(url: string, options: ProviderOptions = {}) {
    // Ensure URL has no trailing slash
    this.url = url.replace(/\/+$/, "");
    this.client = axios.create({
      baseURL: this.url,
      timeout: options.timeout || 30000,
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        ...options.headers,
      },
    });
  }

  async request(method: string, params: any[] = []): Promise<any> {
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

      // Rust RPC returns data directly (not wrapped in JSON-RPC envelope)
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

  private mapMethodToEndpoint(method: string): EndpointInfo {
    const get: Record<string, string> = {
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
      fee_history: "/fee_history/",
    };

    const post: Record<string, string> = {
      send_transaction: "/submit_tx",
      submit_tx: "/submit_tx",
      send_raw_transaction: "/submit_tx",
      connect_peer: "/connect_peer",
      estimate_gas_post: "/estimate_gas",
    };

    if (post[method]) {
      return { method: "POST", path: post[method] };
    }
    return { method: "GET", path: get[method] || "/" };
  }

  private buildGetUrl(basePath: string, params: any[]): string {
    if (params.length === 0) return basePath;
    const param = params[0];
    if (param === undefined || param === null) return basePath;
    const encoded = typeof param === "string" ? encodeURIComponent(param) : String(param);
    return basePath + encoded;
  }

  getUrl(): string {
    return this.url;
  }

  isConnectedToNode(): boolean {
    return this.isConnected;
  }
}
