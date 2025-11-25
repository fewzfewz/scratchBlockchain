// Modular Blockchain JavaScript SDK
// Version: 0.1.0

const crypto = require('crypto');
const fetch = require('node-fetch');
const WebSocket = require('ws');

/**
 * Main SDK class for interacting with the blockchain
 */
class ModularBlockchain {
    constructor(rpcUrl, wsUrl = null) {
        this.rpcUrl = rpcUrl;
        this.wsUrl = wsUrl;
        this.ws = null;
        this.eventHandlers = new Map();
    }

    /**
     * Connect to WebSocket for real-time events
     */
    async connect() {
        if (!this.wsUrl) {
            throw new Error('WebSocket URL not provided');
        }

        return new Promise((resolve, reject) => {
            this.ws = new WebSocket(this.wsUrl);
            
            this.ws.on('open', () => {
                console.log('Connected to blockchain');
                resolve();
            });

            this.ws.on('message', (data) => {
                const event = JSON.parse(data);
                this._handleEvent(event);
            });

            this.ws.on('error', reject);
        });
    }

    /**
     * Subscribe to blockchain events
     */
    on(eventName, handler) {
        if (!this.eventHandlers.has(eventName)) {
            this.eventHandlers.set(eventName, []);
        }
        this.eventHandlers.get(eventName).push(handler);
    }

    _handleEvent(event) {
        const handlers = this.eventHandlers.get(event.type) || [];
        handlers.forEach(handler => handler(event.data));
    }

    /**
     * Get current block height
     */
    async getBlockHeight() {
        const response = await this._rpcCall('GET', '/status');
        return response.block_height;
    }

    /**
     * Get block by height
     */
    async getBlock(height) {
        return await this._rpcCall('GET', `/block/${height}`);
    }

    /**
     * Get account balance
     */
    async getBalance(address) {
        const response = await this._rpcCall('GET', `/balance/${address}`);
        return BigInt(response.balance);
    }

    /**
     * Get transaction receipt
     */
    async getReceipt(txHash) {
        return await this._rpcCall('GET', `/tx/${txHash}`);
    }

    /**
     * Estimate gas for a transaction
     */
    async estimateGas(tx) {
        return await this._rpcCall('POST', '/estimate_gas', tx);
    }

    /**
     * Get current gas price
     */
    async getGasPrice() {
        return await this._rpcCall('GET', '/gas_price');
    }

    /**
     * Submit a signed transaction
     */
    async sendTransaction(signedTx) {
        return await this._rpcCall('POST', '/submit_tx', signedTx);
    }

    /**
     * Get mempool status
     */
    async getMempoolStatus() {
        const response = await this._rpcCall('GET', '/mempool');
        return response;
    }

    /**
     * Get network metrics
     */
    async getMetrics() {
        return await this._rpcCall('GET', '/metrics');
    }

    /**
     * Internal RPC call method
     */
    async _rpcCall(method, path, body = null) {
        const url = `${this.rpcUrl}${path}`;
        const options = {
            method,
            headers: {
                'Content-Type': 'application/json',
            },
        };

        if (body) {
            options.body = JSON.stringify(body);
        }

        const response = await fetch(url, options);
        
        if (!response.ok) {
            throw new Error(`RPC call failed: ${response.statusText}`);
        }

        return await response.json();
    }

    /**
     * Close WebSocket connection
     */
    disconnect() {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
    }
}

/**
 * Account/Wallet management
 */
class Account {
    constructor(privateKey = null) {
        if (privateKey) {
            this.privateKey = Buffer.from(privateKey, 'hex');
        } else {
            // Generate new key
            this.privateKey = crypto.randomBytes(32);
        }
        
        // Derive public key and address (simplified)
        this.publicKey = this._derivePublicKey();
        this.address = this._deriveAddress();
    }

    _derivePublicKey() {
        // In real implementation, use ed25519 or secp256k1
        // For now, just hash the private key (NOT SECURE)
        return crypto.createHash('sha256').update(this.privateKey).digest();
    }

    _deriveAddress() {
        // Address is first 20 bytes of public key hash
        const hash = crypto.createHash('sha256').update(this.publicKey).digest();
        return '0x' + hash.slice(0, 20).toString('hex');
    }

    /**
     * Sign a transaction
     */
    signTransaction(tx) {
        // Create transaction hash
        const txData = JSON.stringify({
            from: tx.from,
            to: tx.to,
            value: tx.value,
            nonce: tx.nonce,
            gasLimit: tx.gasLimit,
            maxFeePerGas: tx.maxFeePerGas,
            maxPriorityFeePerGas: tx.maxPriorityFeePerGas,
            data: tx.data || '',
        });

        const txHash = crypto.createHash('sha256').update(txData).digest();

        // Sign (simplified - in real implementation use ed25519)
        const signature = crypto.createHmac('sha256', this.privateKey)
            .update(txHash)
            .digest();

        return {
            ...tx,
            signature: signature.toString('hex'),
        };
    }

    /**
     * Get private key as hex
     */
    getPrivateKey() {
        return this.privateKey.toString('hex');
    }

    /**
     * Get address
     */
    getAddress() {
        return this.address;
    }
}

/**
 * Transaction builder
 */
class TransactionBuilder {
    constructor(blockchain) {
        this.blockchain = blockchain;
        this.tx = {
            from: null,
            to: null,
            value: 0,
            data: '',
            nonce: null,
            gasLimit: null,
            maxFeePerGas: null,
            maxPriorityFeePerGas: null,
        };
    }

    from(address) {
        this.tx.from = address;
        return this;
    }

    to(address) {
        this.tx.to = address;
        return this;
    }

    value(amount) {
        this.tx.value = amount;
        return this;
    }

    data(hexData) {
        this.tx.data = hexData;
        return this;
    }

    async build() {
        // Auto-fill missing fields
        if (!this.tx.nonce) {
            const balance = await this.blockchain.getBalance(this.tx.from);
            // In real implementation, get nonce from account
            this.tx.nonce = 0;
        }

        if (!this.tx.gasLimit) {
            const estimate = await this.blockchain.estimateGas(this.tx);
            this.tx.gasLimit = estimate.estimated_gas;
        }

        if (!this.tx.maxFeePerGas) {
            const gasPrice = await this.blockchain.getGasPrice();
            this.tx.maxFeePerGas = parseInt(gasPrice.base_fee);
            this.tx.maxPriorityFeePerGas = parseInt(gasPrice.suggested_priority_fee_medium);
        }

        return this.tx;
    }
}

// Export classes
module.exports = {
    ModularBlockchain,
    Account,
    TransactionBuilder,
};

// Example usage:
/*
const { ModularBlockchain, Account, TransactionBuilder } = require('./sdk');

async function main() {
    // Connect to blockchain
    const blockchain = new ModularBlockchain('http://localhost:9933');
    
    // Create or import account
    const account = new Account(); // Generate new
    // const account = new Account('private_key_hex'); // Import existing
    
    console.log('Address:', account.getAddress());
    
    // Get balance
    const balance = await blockchain.getBalance(account.getAddress());
    console.log('Balance:', balance.toString());
    
    // Build and send transaction
    const txBuilder = new TransactionBuilder(blockchain);
    const tx = await txBuilder
        .from(account.getAddress())
        .to('0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb')
        .value(1000000000) // 1 token
        .build();
    
    const signedTx = account.signTransaction(tx);
    const receipt = await blockchain.sendTransaction(signedTx);
    console.log('Transaction sent:', receipt);
    
    // Subscribe to events
    await blockchain.connect();
    blockchain.on('newBlock', (block) => {
        console.log('New block:', block.height);
    });
}

main().catch(console.error);
*/
