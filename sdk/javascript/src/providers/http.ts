import axios, { AxiosInstance } from 'axios';
import { Provider, ProviderOptions } from '../types/provider';

export class HttpProvider implements Provider {
  private client: AxiosInstance;
  private url: string;

  constructor(url: string, options: ProviderOptions = {}) {
    this.url = url;
    this.client = axios.create({
      baseURL: url,
      timeout: options.timeout || 30000,
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
    });
  }

  async request(method: string, params: any[] = []): Promise<any> {
    try {
      const response = await this.client.post('/', {
        jsonrpc: '2.0',
        id: Date.now(),
        method,
        params,
      });

      if (response.data.error) {
        throw new Error(response.data.error.message || 'RPC Error');
      }

      return response.data.result;
    } catch (error) {
      if (axios.isAxiosError(error)) {
        throw new Error(`HTTP Error: ${error.message}`);
      }
      throw error;
    }
  }

  getUrl(): string {
    return this.url;
  }
}
