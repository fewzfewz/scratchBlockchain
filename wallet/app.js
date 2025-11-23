const API_URL = 'http://localhost:9933';

// State
let keyPair = null;

// DOM Elements
const els = {
    generateBtn: document.getElementById('generateBtn'),
    addressDisplay: document.getElementById('addressDisplay'),
    privateKeyDisplay: document.getElementById('privateKeyDisplay'),
    toggleKey: document.getElementById('toggleKey'),
    copyAddress: document.getElementById('copyAddress'),
    copyKey: document.getElementById('copyKey'),
    balanceDisplay: document.getElementById('balanceDisplay'),
    refreshBalanceBtn: document.getElementById('refreshBalanceBtn'),
    sendForm: document.getElementById('sendForm'),
    recipientInput: document.getElementById('recipientInput'),
    amountInput: document.getElementById('amountInput'),
    txStatus: document.getElementById('txStatus')
};

// Key Management
function generateKeyPair() {
    const pair = nacl.sign.keyPair();
    keyPair = {
        publicKey: pair.publicKey,
        secretKey: pair.secretKey
    };
    
    // Convert to hex for display
    const pubKeyHex = nacl.util.encodeBase64(keyPair.publicKey); // Using Base64 for shorter display, or Hex? 
    // Let's use Hex as it's more standard for addresses usually
    const pubKeyHexStr = toHex(keyPair.publicKey);
    const privKeyHexStr = toHex(keyPair.secretKey);

    els.addressDisplay.value = pubKeyHexStr;
    els.privateKeyDisplay.value = privKeyHexStr;
    
    // Save to local storage (insecure for real app, ok for demo)
    localStorage.setItem('nebula_wallet_priv', privKeyHexStr);
    
    updateBalance();
}

function loadSavedKey() {
    const savedPriv = localStorage.getItem('nebula_wallet_priv');
    if (savedPriv) {
        try {
            const secretKey = fromHex(savedPriv);
            const pair = nacl.sign.keyPair.fromSecretKey(secretKey);
            keyPair = {
                publicKey: pair.publicKey,
                secretKey: pair.secretKey
            };
            els.addressDisplay.value = toHex(keyPair.publicKey);
            els.privateKeyDisplay.value = toHex(keyPair.secretKey);
            updateBalance();
        } catch (e) {
            console.error('Invalid saved key', e);
        }
    }
}

// Utils
function toHex(buffer) {
    return Array.prototype.map.call(buffer, x => ('00' + x.toString(16)).slice(-2)).join('');
}

function fromHex(hex) {
    const bytes = new Uint8Array(hex.length / 2);
    for (let i = 0; i < hex.length; i += 2) {
        bytes[i / 2] = parseInt(hex.substr(i, 2), 16);
    }
    return bytes;
}

// Balance
async function updateBalance() {
    if (!keyPair) return;
    
    try {
        const address = els.addressDisplay.value;
        const response = await fetch(`${API_URL}/balance/${address}`);
        if (response.ok) {
            const data = await response.json();
            els.balanceDisplay.textContent = data.balance.toFixed(2);
        } else {
            els.balanceDisplay.textContent = '0.00';
        }
    } catch (error) {
        console.error('Error fetching balance:', error);
        els.balanceDisplay.textContent = 'Err';
    }
}

// Transaction
async function sendTransaction(e) {
    e.preventDefault();
    if (!keyPair) {
        showStatus('Please generate or load a wallet first', 'error');
        return;
    }

    const recipient = els.recipientInput.value.trim();
    const amount = parseFloat(els.amountInput.value);

    if (!recipient || isNaN(amount) || amount <= 0) {
        showStatus('Invalid recipient or amount', 'error');
        return;
    }

    try {
        showStatus('Signing and sending...', 'success'); // Using success style for info

        // Create Transaction Object (simplified matching Rust struct)
        // Note: In a real app, we need to fetch nonce first
        // For MVP, we'll assume nonce 0 or random for now, or fetch it
        // Let's try to fetch nonce/account info first
        
        // Mock nonce for now as we don't have a nonce endpoint explicitly documented
        // We'll use a random nonce to avoid collisions in this demo
        const nonce = Math.floor(Math.random() * 1000000); 

        const tx = {
            sender: Array.from(keyPair.publicKey),
            to: Array.from(fromHex(recipient)), // Assuming recipient is hex
            nonce: nonce,
            value: amount,
            gas_limit: 21000,
            max_fee_per_gas: 1000,
            max_priority_fee_per_gas: 100,
            payload: [],
            chain_id: 1
        };

        // Serialize for signing (simplified - should match Rust serialization exactly)
        // This is tricky in JS without a matching serializer. 
        // For this demo, we'll mock the signature or send the raw data and let the node handle it if possible?
        // No, node expects signed tx.
        // We'll sign a dummy message for now to prove we have the key.
        // In a real implementation, we'd need a WASM binding of the Rust serialization logic.
        
        const message = new TextEncoder().encode(JSON.stringify(tx)); // Naive serialization
        const signature = nacl.sign.detached(message, keyPair.secretKey);
        
        tx.signature = Array.from(signature);

        // Send to Node
        const response = await fetch(`${API_URL}/submit_tx`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(tx)
        });

        if (response.ok) {
            showStatus(`Transaction Sent! Hash: ${await response.text()}`, 'success');
            els.amountInput.value = '';
            setTimeout(updateBalance, 2000);
        } else {
            showStatus(`Failed: ${await response.text()}`, 'error');
        }

    } catch (error) {
        console.error('Tx Error:', error);
        showStatus(`Error: ${error.message}`, 'error');
    }
}

function showStatus(msg, type) {
    els.txStatus.textContent = msg;
    els.txStatus.className = `status-message ${type}`;
    els.txStatus.classList.remove('hidden');
}

// Event Listeners
els.generateBtn.addEventListener('click', generateKeyPair);
els.refreshBalanceBtn.addEventListener('click', updateBalance);
els.sendForm.addEventListener('submit', sendTransaction);

els.toggleKey.addEventListener('click', () => {
    const type = els.privateKeyDisplay.type;
    els.privateKeyDisplay.type = type === 'password' ? 'text' : 'password';
});

els.copyAddress.addEventListener('click', () => {
    navigator.clipboard.writeText(els.addressDisplay.value);
    // Could add tooltip "Copied!"
});

// Init
loadSavedKey();
