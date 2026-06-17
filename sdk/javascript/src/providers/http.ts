import axios, { AxiosInstance } from "axios";
import { Provider, ProviderOptions } from "../types/provider";

export class HttpProvider implements Provider {
  private client: AxiosInstance;
  private url: string;
  private isConnected: boolean = false;

  constructor(url: string, options: ProviderOptions = {}) {
    this.url = url;
    this.client = axios.create({
      baseURL: url,
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
      // Convert method names to match Rust RPC endpoints
      const endpoint = this.mapMethodToEndpoint(method);
      const response = await this.client.post(
        endpoint,
        this.buildRequestBody(method, params),
      );

      if (response.data.error) {
        throw new Error(response.data.error.message || "RPC Error");
      }

      this.isConnected = true;
      return response.data.result || response.data;
    } catch (error) {
      this.isConnected = false;
      if (axios.isAxiosError(error)) {
        throw new Error(`HTTP Error: ${error.message}`);
      }
      throw error;
    }
  }

  private mapMethodToEndpoint(method: string): string {
    const methodMap: Record<string, string> = {
      chain_id: "/status",
      block_number: "/status",
      get_block: "/block/",
      get_balance: "/balance/",
      get_account: "/balance/",
      send_transaction: "/submit_tx",
      get_transaction: "/tx/",
      get_transaction_receipt: "/tx/",
      estimate_gas: "/estimate_gas",
      gas_price: "/gas_price",
      get_mempool: "/mempool",
      get_peers: "/peers",
      get_metrics: "/metrics",
      get_health: "/health",
    };

    return methodMap[method] || "/";
  }

  private buildRequestBody(method: string, params: any[]): any {
    // Handle GET endpoints with params in URL
    if (method === "get_block" && params[0]) {
      return {}; // Will append to URL
    }
    if (method === "get_balance" && params[0]) {
      return {};
    }
    if (method === "get_transaction" && params[0]) {
      return {};
    }

    // POST endpoints
    return params[0] || {};
  }

  getUrl(): string {
    return this.url;
  }

  isConnectedToNode(): boolean {
    return this.isConnected;
  }
}
