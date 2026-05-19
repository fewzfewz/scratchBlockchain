const API_URL = "http://localhost:9933";

// State
let keyPair = null;
let currentNonce = 0;
let txPollingInterval = null;

// DOM Elements
const els = {
  generateBtn: document.getElementById("generateBtn"),
  addressDisplay: document.getElementById("addressDisplay"),
  privateKeyDisplay: document.getElementById("privateKeyDisplay"),
  toggleKey: document.getElementById("toggleKey"),
  copyAddress: document.getElementById("copyAddress"),
  copyKey: document.getElementById("copyKey"),
  balanceDisplay: document.getElementById("balanceDisplay"),
  refreshBalanceBtn: document.getElementById("refreshBalanceBtn"),
  sendForm: document.getElementById("sendForm"),
  recipientInput: document.getElementById("recipientInput"),
  amountInput: document.getElementById("amountInput"),
  txStatus: document.getElementById("txStatus"),
  txHashDisplay: document.getElementById("txHashDisplay"),
};

// Utility Functions
function toHex(buffer) {
  return Array.from(new Uint8Array(buffer))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

function fromHex(hex) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substr(i, 2), 16);
  }
  return bytes;
}

function shortenAddress(address, start = 12, end = 10) {
  if (!address || address.length <= start + end) return address || "";
  return `${address.slice(0, start)}...${address.slice(-end)}`;
}

function showStatus(message, type = "info") {
  els.txStatus.textContent = message;
  els.txStatus.className = `status-message ${type}`;
  els.txStatus.classList.remove("hidden");

  // Auto-hide after 5 seconds for success messages
  if (type === "success") {
    setTimeout(() => {
      if (els.txStatus.textContent === message) {
        els.txStatus.classList.add("hidden");
      }
    }, 5000);
  }
}

// Key Management
async function generateKeyPair() {
  try {
    // Generate Ed25519 key pair
    const pair = nacl.sign.keyPair();

    keyPair = {
      publicKey: pair.publicKey,
      secretKey: pair.secretKey,
    };

    // Convert to hex for display
    const pubKeyHex = toHex(keyPair.publicKey);
    const privKeyHex = toHex(keyPair.secretKey);

    els.addressDisplay.value = pubKeyHex;
    els.privateKeyDisplay.value = privKeyHex;

    // Save to localStorage (development only)
    localStorage.setItem("nebula_wallet_priv", privKeyHex);
    localStorage.setItem("nebula_wallet_pub", pubKeyHex);

    showStatus(
      "✅ New wallet generated! Save your private key securely.",
      "success",
    );

    // Fetch nonce and balance
    await fetchNonce();
    await updateBalance();
  } catch (error) {
    console.error("Generation error:", error);
    showStatus("❌ Failed to generate keypair", "error");
  }
}

function loadSavedKey() {
  const savedPriv = localStorage.getItem("nebula_wallet_priv");
  const savedPub = localStorage.getItem("nebula_wallet_pub");

  if (savedPriv && savedPub) {
    try {
      const secretKey = fromHex(savedPriv);
      const publicKey = fromHex(savedPub);

      keyPair = {
        publicKey: publicKey,
        secretKey: secretKey,
      };

      els.addressDisplay.value = savedPub;
      els.privateKeyDisplay.value = savedPriv;

      console.log("Loaded saved wallet");
      fetchNonce();
      updateBalance();
    } catch (e) {
      console.error("Failed to load saved key:", e);
      localStorage.removeItem("nebula_wallet_priv");
      localStorage.removeItem("nebula_wallet_pub");
    }
  }
}

// Fetch Account Nonce
async function fetchNonce() {
  if (!keyPair) return 0;

  try {
    const address = els.addressDisplay.value;
    const response = await fetch(`${API_URL}/balance/${address}`);

    if (response.ok) {
      const data = await response.json();
      currentNonce = data.nonce || 0;
      console.log(`Current nonce: ${currentNonce}`);
      return currentNonce;
    }
  } catch (error) {
    console.error("Error fetching nonce:", error);
  }
  return 0;
}

// Balance
async function updateBalance() {
  if (!keyPair) {
    els.balanceDisplay.textContent = "0.00";
    return;
  }

  try {
    const address = els.addressDisplay.value;
    const response = await fetch(`${API_URL}/balance/${address}`);

    if (response.ok) {
      const data = await response.json();
      // Convert from wei/ smallest unit to token (assuming 10^18 decimals)
      const balanceInTokens = (parseInt(data.balance) / 1e18).toFixed(4);
      els.balanceDisplay.textContent = balanceInTokens;
    } else {
      els.balanceDisplay.textContent = "0.0000";
    }
  } catch (error) {
    console.error("Error fetching balance:", error);
    els.balanceDisplay.textContent = "Error";
  }
}

// Create Transaction Object (matching Rust Transaction struct)
function createTransactionObject(recipientHex, amount, nonce, chainId = 1) {
  // Convert amount to smallest unit (assuming 10^18 decimals)
  const amountInWei = Math.floor(amount * 1e18);

  // Convert hex addresses to byte arrays
  const senderBytes = Array.from(keyPair.publicKey);
  const recipientBytes = Array.from(fromHex(recipientHex.replace("0x", "")));

  // Default gas values
  const GAS_LIMIT = 21000;
  const MAX_FEE_PER_GAS = 1000000000; // 1 Gwei
  const MAX_PRIORITY_FEE_PER_GAS = 100000000; // 0.1 Gwei

  return {
    sender: senderBytes,
    to: recipientBytes,
    nonce: nonce,
    value: amountInWei,
    gas_limit: GAS_LIMIT,
    max_fee_per_gas: MAX_FEE_PER_GAS,
    max_priority_fee_per_gas: MAX_PRIORITY_FEE_PER_GAS,
    payload: [],
    chain_id: chainId,
    signature: [], // Will be filled after signing
  };
}

// Sign Transaction (matching Rust signing)
function signTransaction(txObject) {
  // Create message to sign (must match Rust's transaction.hash())
  const message = createTransactionHash(txObject);

  // Sign the message
  const signature = nacl.sign.detached(message, keyPair.secretKey);

  // Add signature to transaction
  txObject.signature = Array.from(signature);

  return txObject;
}

// Create transaction hash (must match Rust implementation)
function createTransactionHash(tx) {
  const encoder = new TextEncoder();
  let hashInput = new Uint8Array(0);

  // Helper to append bytes
  function appendBytes(bytes) {
    const newArray = new Uint8Array(hashInput.length + bytes.length);
    newArray.set(hashInput);
    newArray.set(bytes, hashInput.length);
    hashInput = newArray;
  }

  // Append sender address (20 bytes)
  appendBytes(new Uint8Array(tx.sender.slice(0, 20)));

  // Append nonce (8 bytes)
  const nonceBytes = new Uint8Array(8);
  new DataView(nonceBytes.buffer).setBigUint64(0, BigInt(tx.nonce), true);
  appendBytes(nonceBytes);

  // Append payload
  appendBytes(new Uint8Array(tx.payload));

  // Append gas limit (8 bytes)
  const gasLimitBytes = new Uint8Array(8);
  new DataView(gasLimitBytes.buffer).setBigUint64(
    0,
    BigInt(tx.gas_limit),
    true,
  );
  appendBytes(gasLimitBytes);

  // Append max fee per gas (8 bytes)
  const maxFeeBytes = new Uint8Array(8);
  new DataView(maxFeeBytes.buffer).setBigUint64(
    0,
    BigInt(tx.max_fee_per_gas),
    true,
  );
  appendBytes(maxFeeBytes);

  // Append max priority fee per gas (8 bytes)
  const maxPriorityBytes = new Uint8Array(8);
  new DataView(maxPriorityBytes.buffer).setBigUint64(
    0,
    BigInt(tx.max_priority_fee_per_gas),
    true,
  );
  appendBytes(maxPriorityBytes);

  // Append chain ID (8 bytes)
  if (tx.chain_id) {
    const chainIdBytes = new Uint8Array(8);
    new DataView(chainIdBytes.buffer).setBigUint64(
      0,
      BigInt(tx.chain_id),
      true,
    );
    appendBytes(chainIdBytes);
  }

  // Append recipient (if exists)
  if (tx.to && tx.to.length > 0) {
    appendBytes(new Uint8Array(tx.to.slice(0, 20)));
  }

  // Append value (8 bytes)
  const valueBytes = new Uint8Array(8);
  new DataView(valueBytes.buffer).setBigUint64(0, BigInt(tx.value), true);
  appendBytes(valueBytes);

  // Double SHA-256 (like Bitcoin)
  const firstHash = crypto.subtle.digestSync("SHA-256", hashInput);
  const finalHash = crypto.subtle.digestSync("SHA-256", firstHash);

  return new Uint8Array(finalHash);
}

// Send Transaction
async function sendTransaction(e) {
  e.preventDefault();

  if (!keyPair) {
    showStatus("❌ Please generate or load a wallet first", "error");
    return;
  }

  const recipient = els.recipientInput.value.trim();
  const amount = parseFloat(els.amountInput.value);

  if (!recipient) {
    showStatus("❌ Please enter a recipient address", "error");
    return;
  }

  if (isNaN(amount) || amount <= 0) {
    showStatus("❌ Please enter a valid amount", "error");
    return;
  }

  // Validate recipient address format
  let cleanRecipient = recipient.replace("0x", "");
  if (cleanRecipient.length !== 64 && cleanRecipient.length !== 40) {
    showStatus(
      "❌ Invalid recipient address (must be 32-byte hex or 20-byte address)",
      "error",
    );
    return;
  }

  try {
    showStatus("🔐 Fetching nonce and preparing transaction...", "info");

    // Fetch current nonce
    const nonce = await fetchNonce();

    showStatus("✍️ Creating and signing transaction...", "info");

    // Create transaction object
    let tx = createTransactionObject(recipient, amount, nonce);

    // Sign transaction
    tx = signTransaction(tx);

    showStatus("📡 Sending transaction to node...", "info");

    // Send to node
    const response = await fetch(`${API_URL}/submit_tx`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
      },
      body: JSON.stringify(tx),
    });

    const responseText = await response.text();

    if (response.ok) {
      const txHash = responseText.replace(/^"|"$/g, "");
      const shortHash = shortenAddress(txHash, 8, 8);

      showStatus(
        `✅ Transaction sent! Hash: ${shortHash}\nWaiting for confirmation...`,
        "success",
      );

      // Clear form
      els.amountInput.value = "";

      // Increment nonce locally
      currentNonce++;

      // Poll for confirmation
      await waitForTransaction(txHash);

      // Refresh balance after transaction
      setTimeout(() => {
        updateBalance();
        fetchNonce();
      }, 3000);
    } else {
      showStatus(`❌ Transaction failed: ${responseText}`, "error");
    }
  } catch (error) {
    console.error("Transaction error:", error);
    showStatus(`❌ Error: ${error.message}`, "error");
  }
}

// Wait for transaction confirmation
async function waitForTransaction(txHash, maxAttempts = 15) {
  showStatus(`⏳ Waiting for confirmation... (checking receipt)`, "info");

  for (let i = 0; i < maxAttempts; i++) {
    await new Promise((resolve) => setTimeout(resolve, 2000)); // Wait 2 seconds

    try {
      const response = await fetch(`${API_URL}/tx/${txHash}`);

      if (response.ok) {
        const receipt = await response.json();
        if (receipt && receipt.status === "Success") {
          showStatus(
            `✅ Transaction confirmed! Block: ${receipt.block_height || "unknown"}`,
            "success",
          );
          return true;
        }
      }
    } catch (e) {
      // Still waiting
      console.log(`Waiting for tx ${txHash}, attempt ${i + 1}`);
    }
  }

  showStatus(
    `⚠️ Transaction submitted but not yet confirmed. Check explorer later.`,
    "info",
  );
  return false;
}

// Copy to clipboard
async function copyToClipboard(value, successMessage) {
  if (!value) {
    showStatus("Nothing to copy", "error");
    return;
  }

  try {
    await navigator.clipboard.writeText(value);
    showStatus(successMessage, "success");
  } catch (error) {
    console.error("Clipboard error:", error);
    showStatus("Failed to copy", "error");
  }
}

// Clear wallet (logout)
function clearWallet() {
  if (
    confirm(
      "Are you sure you want to clear the wallet? Make sure you have saved your private key!",
    )
  ) {
    localStorage.removeItem("nebula_wallet_priv");
    localStorage.removeItem("nebula_wallet_pub");
    keyPair = null;
    els.addressDisplay.value = "";
    els.privateKeyDisplay.value = "";
    els.balanceDisplay.textContent = "0.0000";
    showStatus("Wallet cleared. Generate a new one to continue.", "info");
  }
}

// Event Listeners
els.generateBtn.addEventListener("click", generateKeyPair);
els.refreshBalanceBtn.addEventListener("click", () => {
  updateBalance();
  fetchNonce();
  showStatus("Balance refreshed", "success");
});
els.sendForm.addEventListener("submit", sendTransaction);

els.toggleKey.addEventListener("click", () => {
  const type = els.privateKeyDisplay.type;
  els.privateKeyDisplay.type = type === "password" ? "text" : "password";
});

els.copyAddress.addEventListener("click", () => {
  copyToClipboard(els.addressDisplay.value, "Address copied to clipboard");
});

els.copyKey.addEventListener("click", () => {
  copyToClipboard(
    els.privateKeyDisplay.value,
    "Private key copied to clipboard",
  );
});

// Add clear button to UI (optional)
const addClearButton = () => {
  const buttonContainer = document.querySelector(".panel-header");
  if (buttonContainer && !document.getElementById("clearWalletBtn")) {
    const clearBtn = document.createElement("button");
    clearBtn.id = "clearWalletBtn";
    clearBtn.className = "btn-secondary";
    clearBtn.innerHTML = '<i data-lucide="trash-2"></i>';
    clearBtn.style.marginLeft = "8px";
    clearBtn.addEventListener("click", clearWallet);
    buttonContainer.appendChild(clearBtn);
    lucide.createIcons();
  }
};

// Initialize
function init() {
  loadSavedKey();
  addClearButton();

  // Start polling for balance updates every 10 seconds
  setInterval(() => {
    if (keyPair) {
      updateBalance();
    }
  }, 10000);
}

// Wait for DOM and libraries
document.addEventListener("DOMContentLoaded", init);
