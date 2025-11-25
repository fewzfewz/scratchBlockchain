export interface Provider {
  request(method: string, params?: any[]): Promise<any>;
  on?(event: string, callback: (...args: any[]) => void): void;
  off?(event: string, callback: (...args: any[]) => void): void;
}

export interface ProviderOptions {
  timeout?: number;
  headers?: Record<string, string>;
}
